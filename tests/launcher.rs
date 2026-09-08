use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const LAUNCHER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/launcher.sh");

/// Five seconds is a test deadline, not the product's: the Launcher promises
/// milliseconds, and what this deadline buys is that a silent Launcher fails
/// instead of hanging the suite.
const DEADLINE: Duration = Duration::from_secs(5);

/// The plugin directory exactly as the host leaves it: the binary next to the
/// Launcher, in one of its four states.
struct PluginBox {
    path: PathBuf,
}

impl PluginBox {
    fn without_a_binary() -> Self {
        Self::empty("missing")
    }

    /// The real binary, symlinked instead of copied: what is measured is that
    /// the Launcher hands its place over, not how long an 80 MB `cp` takes.
    fn with_the_good_binary() -> Self {
        let plugin = Self::empty("good");
        symlink(env!("CARGO_BIN_EXE_flipchart"), plugin.binary())
            .expect("the flipchart binary can be symlinked");
        plugin
    }

    fn with_the_binary_without_permission() -> Self {
        let plugin = Self::empty("no-permission");
        fs::copy(env!("CARGO_BIN_EXE_flipchart"), plugin.binary())
            .expect("the flipchart binary can be copied");
        plugin.give_it_these_permissions(0o644);
        plugin
    }

    /// The `chmod` that cannot: a read-only file system cannot be mounted
    /// inside a test, and `chflags uchg` reproduces it just the same —not even
    /// the owner can change its permissions—.
    fn with_a_binary_that_cannot_be_fixed() -> Self {
        let plugin = Self::empty("unfixable");
        fs::write(plugin.binary(), "").expect("the fake binary is written");
        plugin.give_it_these_permissions(0o644);
        plugin.chflags("uchg");
        plugin
    }

    /// A PowerPC Mach-O header and nothing behind it: `exec` rejects it with
    /// `ENOEXEC`, and the zeros are what stops bash from taking it for a script
    /// and trying to run it.
    fn with_a_binary_of_another_architecture() -> Self {
        let plugin = Self::empty("another-architecture");
        let mut header = vec![0u8; 96];
        header[..4].copy_from_slice(&0xfeed_facfu32.to_le_bytes());
        header[4..8].copy_from_slice(&0x0100_0012u32.to_le_bytes());
        fs::write(plugin.binary(), header).expect("the fake binary is written");
        plugin.give_it_these_permissions(0o755);
        plugin
    }

    fn empty(state: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "flipchart-launcher-{}-{state}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the plugin box can be created");
        Self { path }
    }

    fn binary(&self) -> PathBuf {
        self.path.join("flipchart")
    }

    fn chflags(&self, flags: &str) {
        let set = Command::new("chflags")
            .args(["-R", flags])
            .arg(&self.path)
            .status()
            .expect("chflags runs");
        assert!(set.success());
    }

    fn give_it_these_permissions(&self, mode: u32) {
        fs::set_permissions(self.binary(), fs::Permissions::from_mode(mode))
            .expect("the binary's permissions can be set");
    }
}

impl Drop for PluginBox {
    fn drop(&mut self) {
        self.chflags("nouchg");
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The Launcher started the way the host starts it: over stdio and with the
/// plugin box in `CLAUDE_PLUGIN_ROOT`.
struct Session {
    process: Child,
    input: Option<ChildStdin>,
    output: Receiver<String>,
    greeting: Value,
    next_id: u64,
}

impl Session {
    fn open(plugin: &PluginBox) -> Self {
        let mut session = Self::raw(&plugin.path);
        session.greeting = session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        session.notify("notifications/initialized");
        session
    }

    fn raw(root: &Path) -> Self {
        let mut process = Command::new(LAUNCHER)
            .env("CLAUDE_PLUGIN_ROOT", root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the Launcher starts");
        let input = process.stdin.take();
        let output = lines_of(process.stdout.take().unwrap());
        Self {
            process,
            input,
            output,
            greeting: Value::Null,
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let answer = self.request_with_id(json!(id), method, params);
        assert_eq!(answer["id"], json!(id));
        answer["result"].clone()
    }

    fn request_with_id(&mut self, id: Value, method: &str, params: Value) -> Value {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.answer()
    }

    fn notify(&mut self, method: &str) {
        self.send(json!({ "jsonrpc": "2.0", "method": method }));
    }

    fn send(&mut self, message: Value) {
        let input = self.input.as_mut().expect("the session is still open");
        writeln!(input, "{message}").expect("the Launcher is listening");
        input.flush().unwrap();
    }

    fn answer(&self) -> Value {
        let line = self
            .output
            .recv_timeout(DEADLINE)
            .expect("the Launcher answers");
        serde_json::from_str(&line).expect("readable JSON-RPC")
    }

    fn tools(&mut self) -> Vec<Value> {
        self.request("tools/list", json!({}))["tools"]
            .as_array()
            .expect("tools/list brings a list")
            .clone()
    }

    fn names_of_its_tools(&mut self) -> Vec<String> {
        let mut names: Vec<String> = self
            .tools()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        names
    }

    fn closes_its_input(&mut self) {
        drop(self.input.take());
    }

    fn receives_sigterm(&mut self) {
        let killed = Command::new("kill")
            .args(["-TERM", &self.process.id().to_string()])
            .status()
            .expect("kill -TERM runs");
        assert!(killed.success());
    }

    fn exits_before(&mut self, deadline: Duration) -> ExitStatus {
        let limit = Instant::now() + deadline;
        while Instant::now() < limit {
            if let Some(status) = self
                .process
                .try_wait()
                .expect("the process can be looked at")
            {
                return status;
            }
            sleep(Duration::from_millis(20));
        }
        panic!("the Launcher did not exit");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn lines_of(output: ChildStdout) -> Receiver<String> {
    let (sends, receives) = channel();
    spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { return };
            if sends.send(line).is_err() {
                return;
            }
        }
    });
    receives
}

fn the_warning_of(session: &mut Session) -> String {
    let tools = session.tools();
    let [warning] = &tools[..] else {
        panic!("the Unavailable server announces a single tool");
    };
    warning["description"]
        .as_str()
        .expect("the tool carries a description")
        .to_string()
}

#[test]
fn with_the_good_binary_the_launcher_hands_its_place_over() {
    let plugin = PluginBox::with_the_good_binary();
    let mut session = Session::open(&plugin);

    assert_eq!(session.names_of_its_tools(), ["clear", "marks", "show"]);
}

/// `check` opens no window and does not speak MCP, so it serves as a witness
/// that the arguments made it across the `exec`.
#[test]
fn the_launcher_passes_the_binary_the_arguments_it_was_called_with() {
    let plugin = PluginBox::with_the_good_binary();

    let run = Command::new(LAUNCHER)
        .env("CLAUDE_PLUGIN_ROOT", &plugin.path)
        .args(["check", "/does-not-exist.mmd"])
        .output()
        .expect("the Launcher runs");

    assert!(String::from_utf8_lossy(&run.stdout).contains("== /does-not-exist.mmd"));
}

#[test]
fn a_binary_without_execute_permission_gets_it_put_on_and_starts() {
    let plugin = PluginBox::with_the_binary_without_permission();
    let mut session = Session::open(&plugin);

    assert_eq!(session.names_of_its_tools(), ["clear", "marks", "show"]);
}

#[test]
fn without_a_binary_it_answers_the_handshake_all_the_same() {
    let plugin = PluginBox::without_a_binary();

    let session = Session::open(&plugin);

    assert_eq!(session.greeting["serverInfo"]["name"], json!("flipchart"));
}

#[test]
fn the_unavailable_servers_handshake_speaks_the_version_it_is_spoken_to_in() {
    let plugin = PluginBox::without_a_binary();

    let session = Session::open(&plugin);

    assert_eq!(session.greeting["protocolVersion"], json!("2025-06-18"));
}

#[test]
fn with_a_binary_of_another_architecture_it_answers_the_handshake_in_milliseconds() {
    let plugin = PluginBox::with_a_binary_of_another_architecture();

    let start = Instant::now();
    let _session = Session::open(&plugin);

    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn the_unavailable_server_announces_a_single_tool() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    assert_eq!(session.names_of_its_tools(), ["unavailable"]);
}

#[test]
fn the_warning_tool_asks_for_no_arguments() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    let schema = session.tools()[0]["inputSchema"].clone();

    assert_eq!(schema, json!({ "type": "object", "properties": {} }));
}

#[test]
fn without_a_binary_the_warning_says_it_is_missing_and_that_it_must_be_reinstalled() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: the flipchart \
         binary is not in the plugin directory. Nothing will appear on screen, so do not offer \
         the user a diagram - explain in prose instead. Reinstalling the plugin is what brings \
         it back."
    );
}

#[test]
fn with_a_binary_of_another_architecture_the_warning_says_this_machine_will_not_run_it() {
    let plugin = PluginBox::with_a_binary_of_another_architecture();
    let mut session = Session::open(&plugin);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: this machine \
         refused to execute the flipchart binary, which is a macOS build - another platform or \
         architecture cannot run it. Nothing will appear on screen, so do not offer the user a \
         diagram - explain in prose instead. Reinstalling the plugin is what brings it back."
    );
}

#[test]
fn with_a_binary_that_cannot_be_fixed_the_warning_says_there_is_no_execute_permission() {
    let plugin = PluginBox::with_a_binary_that_cannot_be_fixed();
    let mut session = Session::open(&plugin);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: the flipchart \
         binary could not be given execute permission. Nothing will appear on screen, so do not \
         offer the user a diagram - explain in prose instead. Reinstalling the plugin is what \
         brings it back."
    );
}

#[test]
fn calling_the_warning_tool_comes_back_marked_as_an_error() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    let result = session.request(
        "tools/call",
        json!({ "name": "unavailable", "arguments": {} }),
    );

    assert_eq!(result["isError"], json!(true));
}

#[test]
fn calling_the_warning_tool_returns_the_same_warning() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);
    let announcement = the_warning_of(&mut session);

    let result = session.request(
        "tools/call",
        json!({ "name": "unavailable", "arguments": {} }),
    );

    assert_eq!(result["content"][0]["text"], json!(announcement));
}

#[test]
fn a_text_id_comes_back_as_written() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    let answer = session.request_with_id(json!("warning-1"), "tools/list", json!({}));

    assert_eq!(answer["id"], json!("warning-1"));
}

#[test]
fn the_initialized_notification_carries_no_answer() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    session.notify("notifications/initialized");

    assert_eq!(
        session.request_with_id(json!(7), "tools/list", json!({}))["id"],
        json!(7)
    );
}

#[test]
fn the_unavailable_server_exits_with_zero_when_its_input_is_closed() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    session.closes_its_input();

    assert_eq!(session.exits_before(DEADLINE).code(), Some(0));
}

#[test]
fn the_unavailable_server_exits_with_zero_when_it_is_killed() {
    let plugin = PluginBox::without_a_binary();
    let mut session = Session::open(&plugin);

    session.receives_sigterm();

    assert_eq!(session.exits_before(DEADLINE).code(), Some(0));
}
