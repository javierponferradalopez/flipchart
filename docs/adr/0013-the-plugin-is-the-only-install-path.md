# The plugin is the only install path: a verified zip, no git

**Status:** accepted · **Date:** 2026-09-03 · **Refined by** [0018](./0018-the-box-carries-one-skill.md), [0019](./0019-two-machines-one-box.md)

**A Claude Code plugin, and it is the only install path.** The binary is still an ordinary
MCP server over stdio —free by construction— but it **is not documented, not tested and not
supported** outside Claude Code.

**The box carries four files —the manifest, the `.mcp.json`, the Launcher and the binary—
and nothing else:** no `skills/`, no `commands/`, no `hooks/`. A zero-toll skill is exactly
the one the model **cannot** invoke, so it cannot own anything; and a `/flipchart:*` in the
menu is product surface promising a control over the flipchart that the user does not have.

*[0018](./0018-the-box-carries-one-skill.md) adds a fifth file, one skill, and the reason
above is what it had to answer: that skill owns nothing —it does not trigger the flipchart—
and it fires on a decision the agent has already taken. The box stays closed against
anything that would claim the trigger.*

*[0019](./0019-two-machines-one-box.md) adds a sixth, and it is the same binary again for
another Machine: `flipchart-macos` and `flipchart-linux-x86_64` travel together and the
Launcher chooses between them. It has to be this way and not two catalog entries, because —as
[0014](./0014-the-launcher-never-fails.md) measured— a marketplace entry has no platform field
at all. The box is still closed: six files, copied one by one.*

## The vehicle: a verified zip, and zero git on the client

- **The catalog is a JSON served over a URL** — `source: "url"`, on
  `raw.githubusercontent.com` at `main`. A **mutable and stable** URL: the user types it
  once and it has to keep serving the right catalog three releases later. The host's `url`
  branch downloads the JSON to the cache and stops: **it never invokes git.**
- **The plugin is a zip over HTTPS with a verified `sha256`** — `source: "archive"`, hosted
  as a release asset. An **immutable** URL
  (`…/releases/download/v0.1.0/flipchart-0.1.0.zip`): a pinned digest points at an exact
  byte, so if the URL could change content the pin would be worth nothing.

With that **the client never executes git**, integrity becomes **verified** —the host checks
the digest on every download and **refuses the installation** if it does not match, with the
error in the foreground— and what is left on disk is a two-kilobyte JSON plus the binary
extracted into the versioned cache, which prunes itself (`.orphaned_at`, collected after 14
days). All three ends are **measured**: not one `.git` file, 2717 bytes of flat catalog, and
the digest refusal with expected and obtained inside the message.

**The disk peak is `2 × B`, not `B`:** measured, `update` leaves both versions in the cache
until the prune runs. With the **49.2 MB** measured for the universal binary that is **~98
MB** between the update and the collection; the zip does not count, because it travels as an
`arraybuffer` and never touches disk.

Measured caps, to be respected because not all of them have a valve:

| | |
|---|---|
| Adding the catalog | **10 s**, **5 MiB** |
| Downloading the archive | **120 s with no valve**, **256 MiB** |
| Redirects | up to 5, with anti-SSRF policy revalidated at each hop |
| Throughput from GitHub's CDN | 25.7–27.9 MB/s (42 MB ≈ 1.6 s: ~75× margin) |

There is no environment variable for the `archive` timeout. The URL policy demands `https://`
and forbids loopback, link-local and cloud metadata hosts — so **there is no local shortcut
for testing this: it has to be really hosted.**

## The shape of the zip, and versioning

Inside the zip: **`.claude-plugin/plugin.json`, `.mcp.json`, the Launcher and the binary.**
Both JSONs are versioned under `publishing/box/`; the Launcher is the repo's `launcher.sh`,
the same one the tests run.

- **It is packed with Info-ZIP** (`zip`), which produces `version made by == 3` (Unix) with
  the `0755` modes intact. **The CI's `zip` is part of the contract**: the host reads the
  external attributes and does `chmod(mode & 0o777)` when there is any execute bit, but that
  depends on who packs. Never from the Finder — it adds `__MACOSX/` and `.DS_Store`.
- **The `sha256` is always declared in the entry, and the generator verifies it.** Measured:
  the host's schema treats it as **optional**, and an entry without it **installs anyway and
  checks nothing**, with no warning. Forgetting it breaks nothing visible: it silently
  disarms the vehicle's only integrity defence.
- **`version` is declared**, and **the one that rules is `plugin.json`'s, inside the zip**,
  not the catalog entry's. It is declared rather than left to the digest because `/plugin`'s
  UI does `manifest.version ?? "unknown"`, and on the one screen where the user judges whether
  to trust an unnotarized native binary the version would read `unknown`, always.
- **The `marketplace.json` is generated from the release tag and never edited by hand.**
  `version`, `url` and `sha256` all come from the same place, so forgetting the bump stops
  being possible. **This is a correctness requirement, not convenience:** `/plugin update`
  downloads the whole zip *before* comparing identities, so a fix published without raising
  the version **is downloaded, thrown away, and not reported** (`already at the latest
  version`).

As a bonus, measured: the release asset **redirects off-origin** (302 to
`release-assets.githubusercontent.com`) and there the host drops the headers inherited from
the catalog — **a private or authenticated asset is impossible by this route**.
`raw.githubusercontent.com` answers 200 with no redirect.

## The release CI

1. Build arm64 and x86_64.
2. `lipo` for the universal binary.
3. **Re-sign ad-hoc** (`codesign -s -`) and verify with **`codesign --verify`**, plus a
   `codesign -dv --arch` per architecture. On Apple Silicon every executable needs at least an
   ad-hoc signature to run; Rust generates it on build, but **only on the native half** —the
   cross-compiled `x86_64` comes out unsigned— and `lipo` preserves the asymmetry. Measured,
   a bare `codesign -dv` over that universal answers `Signature=adhoc` because it reads the
   native slice, and the binary runs on the Mac that built it: **the defect is invisible where
   it is compiled and only bites on Intel Macs.** `-dv` over the file is not a verification.
4. Pack with Info-ZIP.
5. Compute the `sha256`.
6. Generate the `marketplace.json` from the tag, upload the zip as a release asset and commit
   the JSON to `main`.

*[0019](./0019-two-machines-one-box.md) puts a step before the first: an `ubuntu-22.04` job
builds the Linux binary natively and uploads it as an artifact, which the macOS job picks up
before packing. Packing stays on macOS, which is where `lipo` and `codesign` are, and the two
steps that treat the Apple slices are untouched.*

The six steps live in `.github/workflows/publishing.yml`, triggered by the tag and by
nothing else. The two with rules of their own are separate scripts, which is why they can be
tested without publishing anything: `publishing/package.sh` builds the box and closes it
—copying the files one by one, so there is nowhere to slip one more in— and
`publishing/catalog.sh` generates the catalog from the tag. Both **refuse** if the tag's
version is not the one declared by the manifest they are about to publish; `tests/box.rs` is
what has that measured on every `make verify`.

**Never document "download the zip by hand"**: whoever downloads with a browser or Mail gets
`com.apple.quarantine`, and that is the case Gatekeeper kills.

## Quarantine: measured, and there is none

Measured on 2026-09-03 against a really hosted release (report 15). It was the only thing that
could have changed the install vehicle and not just the code: if the host marked
`com.apple.quarantine` on what it extracts, **Gatekeeper would kill an ad-hoc signed,
unnotarized binary**. It does not mark it. The extracted file arrives **without quarantine**,
with mode **`100755`**, and **runs** (`rc=0`) —launched by the host through the Launcher, and
before its `chmod +x`—, the same in the extraction as in the versioned cache copy.

Two log readings not to be mistaken for the opposite of what they say: **`spctl -a` answers
`rejected`** —it always will over an unnotarized ad-hoc— and the binary **runs anyway**,
because without quarantine execution does not consult Gatekeeper; and **`com.apple.provenance`
does appear** on everything installed, but it is not quarantine and prevents nothing.

## Installation, update and uninstallation

**One declared step**, plus the recommended line of ADR-0012:

```
/plugin marketplace add <url-to-the-marketplace.json>
/plugin install flipchart@<marketplace>
```

*(the name for `install` is the manifest's `name` field, not the repo's)*

Verified mechanics: `${CLAUDE_PLUGIN_DATA}` **is available from the `.mcp.json`** —it arrives
expanded as an argument and as environment, and the directory is created empty before
startup— although the MVP does not use it; the plugin is copied to
`~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/` with `.in_use/<pid>` markers, so
**the host already versions by directory and counts references per process**; and
`/plugin uninstall` **deletes the plugin's data** (`--keep-data` exists to avoid that), so
**the README needs no `rm -rf` line**.

**And `update` wants the name with its marketplace.** Measured over `v0.1.0` → `v0.1.1`
(report 20 §5): `update flipchart@flipchart` updates in 2.2 s, and a bare `update flipchart`
answers `Plugin "flipchart" not found` with the plugin installed and listed as `enabled`.
`uninstall`, by contrast, does resolve the short name. The README writes the long form in
both, which is the one that always works.

## Honest requirements

- **macOS 11 or later** (Intel or Apple Silicon). **Confirmed against the build on
  2026-09-04**, and the number is not the provisional one: the Mach-O declares it —`minos
  11.0` on the `arm64` slice and `10.12` on the `x86_64`—, and below what is declared the
  loader will not start it. The greater of the two is promised, which is also the first that
  exists on Apple Silicon. There is no weak-linked API in the binary: the symbols it imports
  from AppKit and Foundation —`beginActivityWithOptions:reason:`, `setActivationPolicy:`,
  `orderFrontRegardless`— are from 10.9 and earlier. **What has not been measured, and is
  written as such: running it on a macOS older than the bank's 26.6.2.**
- **Linux `x86_64`, glibc 2.35 or later** ([0019](./0019-two-machines-one-box.md)). The floor is
  the `ubuntu-22.04` runner's, which is the oldest GitHub hosts, and below it the loader will not
  start the binary — so the Launcher probes before handing over, and what the User reads is a
  sentence and not `✘ failed`. One binary serves X11 and Wayland: `winit` reaches each through
  `dlopen`, so neither is linked. A session with **no display** —over SSH, in a container— gets
  the Unavailable server, because there the window cannot exist at all.
- A version of Claude Code with plugin support.
- **No Node, no Python, no browser, no Rust toolchain.**

**Windows is not declared impossible: it is declared untested and unpromised.**

## Considered options

- **The binary inside the marketplace clone** — rejected. Its `B × (N + 2)` bill is permanent
  (the clone, plus every version in history), and it puts git on the client.
- **Notarizing** — rejected. 99 $/year against an attack that does not happen: measured, what
  the host extracts carries no quarantine, so execution never consults Gatekeeper.
- **`brew` as the main route** — rejected for the MVP; it comes back once the MVP exists.
- **An npm package, or manual registration in Cursor, VS Code Copilot or Claude Desktop** —
  rejected. Every install path that is not the plugin is undocumented, untested and
  unsupported.
- **`experimental.binaries`** — rejected because it is not available. It does *exactly* what
  flipchart needs (files pinned by sha256 fetched into `bin/` at install time, digest
  verified, mode `0755`, shared cache) and it sits behind a feature gate and a `Set` of
  marketplace names reserved to Anthropic. It only comes back if Anthropic opens it.
- **Packing from the Finder** — rejected. It adds `__MACOSX/` and `.DS_Store` and does not
  write the Unix modes the `0755` depends on.

---

*The reports and prototypes cited above by number lived in `docs/research/` until
2026-09-04, when the repo became official and they were withdrawn. The number still
identifies them: they are recovered from the git history.*
