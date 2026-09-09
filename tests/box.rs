use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;

const PACKAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/publishing/package.sh");
const CATALOG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/publishing/catalog.sh");

/// The crate version is the only hand-written number in the whole release: the
/// tag repeats it and both generators refuse if it does not match.
const THE_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPO: &str = "an-account/a-repo";

/// A publishing destination with a toy binary next to it. The packer does not
/// care what the binary is, so three bytes measure the same as the 47 MB of the
/// universal one and do not cost the `cp`.
struct Bench {
    path: PathBuf,
}

impl Bench {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "flipchart-box-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the publishing destination can be created");
        fs::write(path.join("toy-binary"), "not a Mach-O").expect("the toy binary is written");
        Self { path }
    }

    fn package(&self, tag: &str) -> Result<PathBuf, String> {
        let run = Command::new(PACKAGE)
            .arg(tag)
            .arg(self.path.join("toy-binary"))
            .arg(&self.path)
            .output()
            .expect("the packer runs");
        if run.status.success() {
            Ok(PathBuf::from(String::from_utf8_lossy(&run.stdout).trim()))
        } else {
            Err(String::from_utf8_lossy(&run.stderr).trim().to_string())
        }
    }

    fn the_zip(&self) -> PathBuf {
        self.package(&format!("v{THE_VERSION}"))
            .expect("the box is packed")
    }

    fn catalog_of(&self, tag: &str, zip: &Path) -> Result<Value, String> {
        let run = Command::new(CATALOG)
            .args([tag, &zip.to_string_lossy(), REPO])
            .output()
            .expect("the catalog generator runs");
        if run.status.success() {
            Ok(serde_json::from_slice(&run.stdout).expect("the catalog is JSON"))
        } else {
            Err(String::from_utf8_lossy(&run.stderr).trim().to_string())
        }
    }

    fn catalog(&self) -> Value {
        let zip = self.the_zip();
        self.catalog_of(&format!("v{THE_VERSION}"), &zip)
            .expect("the catalog is generated")
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// What `zipinfo` says about each entry in the zip: mode and name, which are the
/// two things the binary arriving executable depends on.
fn inside_the_zip(zip: &Path) -> Vec<(String, String)> {
    let looked_at = Command::new("unzip")
        .arg("-Z")
        .arg(zip)
        .output()
        .expect("zipinfo runs");
    let listing = String::from_utf8_lossy(&looked_at.stdout);
    let mut entries: Vec<(String, String)> = listing
        .lines()
        .filter(|line| line.starts_with('-') || line.starts_with('d'))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields[0].to_string(), fields[fields.len() - 1].to_string())
        })
        .collect();
    entries.sort_by(|one, other| one.1.cmp(&other.1));
    entries
}

fn from_the_zip(zip: &Path, entry: &str) -> String {
    let taken_out = Command::new("unzip")
        .arg("-p")
        .arg(zip)
        .arg(entry)
        .output()
        .expect("unzip runs");
    String::from_utf8_lossy(&taken_out.stdout).to_string()
}

fn json_from_the_zip(zip: &Path, entry: &str) -> Value {
    serde_json::from_str(&from_the_zip(zip, entry)).expect("the zip entry is JSON")
}

fn the_versioned_manifest() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("publishing/box/.claude-plugin/plugin.json");
    serde_json::from_str(&fs::read_to_string(path).expect("the box manifest reads"))
        .expect("the box manifest is JSON")
}

#[test]
fn the_box_carries_the_five_files_and_nothing_else() {
    let bench = Bench::new();

    let names: Vec<String> = inside_the_zip(&bench.the_zip())
        .into_iter()
        .filter(|(mode, _)| !mode.starts_with('d'))
        .map(|(_, name)| name)
        .collect();

    assert_eq!(
        names,
        [
            ".claude-plugin/plugin.json",
            ".mcp.json",
            "flipchart",
            "launcher.sh",
            "skills/choosing-the-family/SKILL.md"
        ]
    );
}

/// The one skill of ADR-0018, and the reason the box is packed file by file: a
/// `cp -R` here would ship whatever else the working tree happens to hold.
#[test]
fn the_skill_in_the_repo_is_the_one_that_gets_packed() {
    let bench = Bench::new();
    let versioned = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/publishing/box/skills/choosing-the-family/SKILL.md"
    ))
    .expect("the repo's skill reads");

    let packed = from_the_zip(&bench.the_zip(), "skills/choosing-the-family/SKILL.md");

    assert_eq!(packed, versioned);
}

/// A skill with no `description` is one the agent cannot reach on its own, and
/// reaching it on its own —once it has decided to draw— is its whole job.
#[test]
fn the_skill_declares_a_description_the_agent_can_be_reached_by() {
    let bench = Bench::new();

    let packed = from_the_zip(&bench.the_zip(), "skills/choosing-the-family/SKILL.md");

    let frontmatter = packed
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .expect("the skill opens with frontmatter")
        .0;
    assert!(
        frontmatter.contains("name: choosing-the-family"),
        "{frontmatter}"
    );
    assert!(frontmatter.contains("description: "), "{frontmatter}");
}

/// What the host does is `chmod(mode & 0o777)` when the zip carries an execute
/// bit, so this mode is what decides whether the binary arrives usable.
#[test]
fn the_binary_travels_with_the_execute_bit() {
    let bench = Bench::new();

    let modes = inside_the_zip(&bench.the_zip());

    assert!(modes.contains(&("-rwxr-xr-x".to_string(), "flipchart".to_string())));
}

#[test]
fn the_launcher_travels_with_the_execute_bit() {
    let bench = Bench::new();

    let modes = inside_the_zip(&bench.the_zip());

    assert!(modes.contains(&("-rwxr-xr-x".to_string(), "launcher.sh".to_string())));
}

/// A binary without execute permission is something the Launcher does fix; a
/// binary put as `command` instead of the Launcher takes the Unavailable server
/// down with it, which is the only voice left when the binary is no good.
#[test]
fn the_command_in_the_mcp_json_is_the_launcher() {
    let bench = Bench::new();

    let mcp = json_from_the_zip(&bench.the_zip(), ".mcp.json");

    assert_eq!(
        mcp["mcpServers"]["flipchart"]["command"],
        "${CLAUDE_PLUGIN_ROOT}/launcher.sh"
    );
}

/// The `/plugin` UI does `manifest.version ?? "unknown"`, and on the one screen
/// where the user judges whether they trust an unnotarised native binary it
/// would put `unknown`.
#[test]
fn the_box_manifest_declares_the_version() {
    let bench = Bench::new();

    let manifest = json_from_the_zip(&bench.the_zip(), ".claude-plugin/plugin.json");

    assert_eq!(manifest["version"], THE_VERSION);
}

#[test]
fn the_versioned_manifest_declares_the_crate_version() {
    assert_eq!(the_versioned_manifest()["version"], THE_VERSION);
}

#[test]
fn the_zip_is_named_after_the_version_it_carries_inside() {
    let bench = Bench::new();

    let zip = bench.the_zip();

    assert_eq!(
        zip.file_name().unwrap().to_string_lossy(),
        format!("flipchart-{THE_VERSION}.zip")
    );
}

#[test]
fn a_tag_that_does_not_match_what_is_declared_is_not_packed() {
    let bench = Bench::new();

    let failure = bench.package("v9.9.9").expect_err("the packer refuses");

    assert!(failure.ends_with("and the tag says 9.9.9"), "{failure}");
}

#[test]
fn without_a_binary_nothing_is_packed() {
    let bench = Bench::new();
    fs::remove_file(bench.path.join("toy-binary")).expect("the binary can be deleted");

    let failure = bench
        .package(&format!("v{THE_VERSION}"))
        .expect_err("the packer refuses");

    assert!(
        failure.starts_with("package: there is no binary at"),
        "{failure}"
    );
}

/// Measured: `sha256` is optional in the host's schema and an entry without it
/// installs just the same and checks nothing, without warning. It is the
/// vehicle's only integrity defence and it is lost silently.
#[test]
fn the_catalog_declares_the_sha256_of_the_zip_it_publishes() {
    let bench = Bench::new();
    let zip = bench.the_zip();
    let its_own = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&zip)
        .output()
        .expect("shasum runs");
    let expected = String::from_utf8_lossy(&its_own.stdout)
        .split_whitespace()
        .next()
        .expect("shasum says the digest")
        .to_string();

    let catalog = bench
        .catalog_of(&format!("v{THE_VERSION}"), &zip)
        .expect("the catalog is generated");

    assert_eq!(catalog["plugins"][0]["source"]["sha256"], expected);
}

/// A pinned digest points at an exact byte, so the URL has to be the release
/// asset's —immutable— and not one whose content could change.
#[test]
fn the_catalog_points_at_the_release_asset_of_the_tag() {
    let bench = Bench::new();

    let catalog = bench.catalog();

    assert_eq!(
        catalog["plugins"][0]["source"]["url"],
        format!(
            "https://github.com/{REPO}/releases/download/v{THE_VERSION}/flipchart-{THE_VERSION}.zip"
        )
    );
}

#[test]
fn the_catalog_installs_by_verified_zip_and_not_by_clone() {
    let bench = Bench::new();

    let catalog = bench.catalog();

    assert_eq!(catalog["plugins"][0]["source"]["source"], "archive");
}

#[test]
fn the_catalog_version_comes_from_the_tag() {
    let bench = Bench::new();

    let catalog = bench.catalog();

    assert_eq!(catalog["plugins"][0]["version"], THE_VERSION);
}

/// The name in the `install` is the manifest's, not the repo's, so the one the
/// catalog announces and the one the user types have to be the same.
#[test]
fn the_catalog_announces_the_plugin_by_the_name_in_its_manifest() {
    let bench = Bench::new();

    let catalog = bench.catalog();

    assert_eq!(
        catalog["plugins"][0]["name"],
        the_versioned_manifest()["name"]
    );
}

/// `/plugin update` downloads the whole zip before comparing identities: a
/// catalog pointing at a zip with another version inside gets downloaded, thrown
/// away and never mentioned.
#[test]
fn a_zip_that_declares_another_version_does_not_enter_the_catalog() {
    let bench = Bench::new();
    let zip = bench.the_zip();

    let failure = bench
        .catalog_of("v9.9.9", &zip)
        .expect_err("the generator refuses");

    assert_eq!(
        failure,
        format!("catalog: the zip declares {THE_VERSION} and the tag says 9.9.9")
    );
}

#[test]
fn without_a_zip_there_is_no_catalog() {
    let bench = Bench::new();

    let failure = bench
        .catalog_of(&format!("v{THE_VERSION}"), Path::new("/does-not-exist.zip"))
        .expect_err("the generator refuses");

    assert_eq!(failure, "catalog: there is no zip at /does-not-exist.zip");
}

/// The archive ceiling is 256 MiB and it **has no valve**: going over degrades
/// nothing, it leaves the plugin with no way to install. What eats the margin
/// are the dependencies, which is exactly what nobody looks at when adding one.
#[test]
fn the_box_fits_within_the_archive_ceiling() {
    let bench = Bench::new();

    let bytes = fs::metadata(bench.the_zip())
        .expect("the zip can be measured")
        .len();

    assert!(bytes <= 256 * 1024 * 1024, "{bytes} bytes of archive");
}

#[test]
fn the_launcher_in_the_repo_is_the_one_that_gets_packed() {
    let bench = Bench::new();
    let versioned = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/launcher.sh"))
        .expect("the repo's Launcher reads");

    let packed = from_the_zip(&bench.the_zip(), "launcher.sh");

    assert_eq!(packed, versioned);
}

#[test]
fn the_binary_arrives_whole_in_the_zip() {
    let bench = Bench::new();

    let packed = from_the_zip(&bench.the_zip(), "flipchart");

    assert_eq!(packed, "not a Mach-O");
}

#[test]
fn the_repos_box_carries_no_execute_permissions_it_should_not_have() {
    let mode = fs::metadata(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/publishing/box/.mcp.json"
    ))
    .expect("the versioned .mcp.json can be measured")
    .permissions()
    .mode();

    assert_eq!(mode & 0o111, 0);
}
