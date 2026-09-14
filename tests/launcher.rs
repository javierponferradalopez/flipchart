use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const LAUNCHER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/launcher.sh");

/// The two names the box carries. A third Machine has none here for the same
/// reason it has no `src/machine` module: nothing is published for it, and the
/// box would carry nothing for it either.
const THE_MACOS_BINARY: &str = "flipchart-macos";
const THE_LINUX_BINARY: &str = "flipchart-linux-x86_64";

/// The same two names read from the Machine the suite is running on: the
/// Launcher chooses one of them and never looks at the other, so which is which
/// is what these tests are about.
#[cfg(target_os = "macos")]
const THE_BINARY_OF_THIS_MACHINE: &str = THE_MACOS_BINARY;
#[cfg(target_os = "macos")]
const THE_BINARY_OF_THE_OTHER_MACHINE: &str = THE_LINUX_BINARY;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const THE_BINARY_OF_THIS_MACHINE: &str = THE_LINUX_BINARY;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const THE_BINARY_OF_THE_OTHER_MACHINE: &str = THE_MACOS_BINARY;

#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
compile_error!(
    "The box carries no binary for this Machine, so there is no name here for \
     the Launcher to choose. Publishing one means reading docs/adr/0019, which \
     says what x86_64-only costs and who it leaves out."
);

/// Five seconds is a test deadline, not the product's: the Launcher promises
/// milliseconds, and what this deadline buys is that a silent Launcher fails
/// instead of hanging the suite.
const DEADLINE: Duration = Duration::from_secs(5);

/// What a stand-in binary prints, and the only witness there is that the
/// Launcher `exec`ed instead of staying: nothing else in this file writes on
/// the Launcher's stdout except JSON-RPC.
const THE_STAND_IN_SPEAKING: &str = "the stand-in binary was reached";

/// The display of the session the Launcher is started in — `DISPLAY` for X11,
/// `WAYLAND_DISPLAY` for Wayland, and neither over SSH, in a container or in a
/// devcontainer, which are normal ways to run Claude Code on Linux.
///
/// Every Launcher this suite starts says which one it has, because the suite's
/// own environment is one thing on a runner with a display and another over
/// SSH, and no test should read differently on the two.
#[derive(Clone, Copy)]
enum Display {
    X11,
    Wayland,
    None,
}

impl Display {
    fn reaches(self, launcher: &mut Command) {
        launcher.env_remove("DISPLAY");
        launcher.env_remove("WAYLAND_DISPLAY");
        match self {
            Self::X11 => {
                launcher.env("DISPLAY", ":0");
            }
            Self::Wayland => {
                launcher.env("WAYLAND_DISPLAY", "wayland-0");
            }
            Self::None => {}
        }
    }
}

/// The Launcher as the Host starts it on the Machine running the suite, with a
/// display, which is the case every test that is not about the display wants.
fn the_launcher() -> Command {
    let mut launcher = Command::new(LAUNCHER);
    Display::X11.reaches(&mut launcher);
    launcher
}

/// A directory of its own for each piece of scenery, named after what it is
/// for so a leftover from a failed run says where it came from.
fn a_directory_for(what: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let path = std::env::temp_dir().join(format!(
        "flipchart-{what}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("the directory can be created");
    path
}

/// The plugin directory exactly as the host leaves it: the binary next to the
/// Launcher, in one of its states.
struct PluginBox {
    path: PathBuf,
}

impl PluginBox {
    fn without_a_binary() -> Self {
        Self::empty("missing")
    }

    /// The box as it reaches the Machine it was not built for: both names
    /// travel in it, and the one this Machine would run is the one missing.
    fn with_only_the_other_machines_binary() -> Self {
        let plugin = Self::empty("the-other-machine");
        fs::write(
            plugin.path.join(THE_BINARY_OF_THE_OTHER_MACHINE),
            "a binary for the other Machine",
        )
        .expect("the other Machine's binary is written");
        plugin
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
    ///
    /// macOS only, and the state is not the Machine's: a binary on a read-only
    /// mount is as real on Linux. What Linux has no unprivileged way to do is
    /// **reach** it from inside a test — `chattr +i` needs root, and taking
    /// write permission off the directory does not stop the owner's `chmod`.
    /// So this one is measured where it can be measured, rather than deleted to
    /// make the other Machine green.
    #[cfg(target_os = "macos")]
    fn with_a_binary_that_cannot_be_fixed() -> Self {
        let plugin = Self::empty("unfixable");
        fs::write(plugin.binary(), "").expect("the fake binary is written");
        plugin.give_it_these_permissions(0o644);
        plugin.chflags("uchg");
        plugin
    }

    /// A PowerPC Mach-O header and nothing behind it: `ENOEXEC` is what the
    /// probe gets for trying to start it, and what `exec` would have got. The
    /// zeros are what stops bash from taking it for a script and running it.
    fn with_a_binary_of_another_architecture() -> Self {
        let plugin = Self::empty("another-architecture");
        let mut header = vec![0u8; 96];
        header[..4].copy_from_slice(&0xfeed_facfu32.to_le_bytes());
        header[4..8].copy_from_slice(&0x0100_0012u32.to_le_bytes());
        fs::write(plugin.binary(), header).expect("the fake binary is written");
        plugin.give_it_these_permissions(0o755);
        plugin
    }

    /// The failure `execfail` cannot reach: something `execve` accepts and
    /// that dies afterwards, which on Linux is what a binary built against a
    /// newer glibc does —the loader fails once bash has already been replaced
    /// (ADR-0019)—. A script has the same shape and needs no second glibc to
    /// arrange one.
    fn with_a_binary_that_starts_and_fails() -> Self {
        let plugin = Self::empty("starts-and-fails");
        fs::write(plugin.binary(), "#!/bin/bash\nexit 1\n").expect("the fake binary is written");
        plugin.give_it_these_permissions(0o755);
        plugin
    }

    /// One Machine's binary as a stand-in that says out loud it was reached,
    /// named for the Machine it belongs to and not for the one running the
    /// suite: the display is asked about on a Machine this suite may not be
    /// sitting on, where no binary of ours could even start. What is measured
    /// through it is whether the Launcher hands its place over at all, not what
    /// it hands it over to — so a script, which both Machines run, is the whole
    /// of what is needed.
    fn with_a_stand_in_for(machine: &str) -> Self {
        let plugin = Self::empty("stand-in");
        let binary = plugin.path.join(machine);
        fs::write(
            &binary,
            format!("#!/bin/bash\necho {THE_STAND_IN_SPEAKING}\n"),
        )
        .expect("the stand-in binary is written");
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))
            .expect("the stand-in binary can be made executable");
        plugin
    }

    fn empty(state: &str) -> Self {
        Self {
            path: a_directory_for(&format!("launcher-{state}")),
        }
    }

    fn binary(&self) -> PathBuf {
        self.path.join(THE_BINARY_OF_THIS_MACHINE)
    }

    #[cfg(target_os = "macos")]
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
        #[cfg(target_os = "macos")]
        self.chflags("nouchg");
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A Machine other than the one running the suite, which is the only way to
/// reach the Users the box carries nothing for: ARM Linux, and everything else.
///
/// `uname` is the whole of what the Launcher asks about the Machine, and it is
/// POSIX and comes off the `PATH` — so answering it is standing at the
/// Machine's own boundary, not at anything of ours.
struct Machine {
    path: PathBuf,
    display: Display,
}

impl Machine {
    fn that_says_it_is(system: &str, architecture: &str) -> Self {
        let path = a_directory_for("machine");
        let uname = path.join("uname");
        fs::write(
            &uname,
            format!(
                "#!/bin/bash\ncase $1 in\n  -s) echo {system} ;;\n  -m) echo {architecture} ;;\nesac\n"
            ),
        )
        .expect("the uname of another Machine is written");
        fs::set_permissions(&uname, fs::Permissions::from_mode(0o755))
            .expect("the uname of another Machine can be made executable");
        Self {
            path,
            display: Display::X11,
        }
    }

    fn with_the_display_in_wayland(mut self) -> Self {
        self.display = Display::Wayland;
        self
    }

    fn with_no_display(mut self) -> Self {
        self.display = Display::None;
        self
    }

    fn launcher(&self) -> Command {
        let mut launcher = Command::new(LAUNCHER);
        launcher.env(
            "PATH",
            format!(
                "{}:{}",
                self.path.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
        self.display.reaches(&mut launcher);
        launcher
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
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
        Self::handshake(Self::raw(the_launcher(), &plugin.path))
    }

    fn open_on(plugin: &PluginBox, machine: &Machine) -> Self {
        Self::handshake(Self::raw(machine.launcher(), &plugin.path))
    }

    fn handshake(mut session: Self) -> Self {
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

    fn raw(mut launcher: Command, root: &Path) -> Self {
        let mut process = launcher
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

/// The Launcher run to its end on another Machine: with nothing on its stdin
/// there is nothing for the Unavailable server to answer, so what is left on
/// stdout is either the stand-in speaking —the hand-over happened— or nothing
/// at all.
fn what_the_launcher_left_on_stdout(plugin: &PluginBox, machine: &Machine) -> String {
    let run = machine
        .launcher()
        .env("CLAUDE_PLUGIN_ROOT", &plugin.path)
        .stdin(Stdio::null())
        .output()
        .expect("the Launcher runs");
    String::from_utf8_lossy(&run.stdout).to_string()
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

    let run = the_launcher()
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

/// The probe is inside this measurement: the process start it costs is paid
/// on every session, out of the same milliseconds the handshake is promised
/// in.
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
         refused to execute the flipchart binary the box carries for it. Nothing will appear on \
         screen, so do not offer the user a diagram - explain in prose instead. Reinstalling the \
         plugin is what brings it back."
    );
}

/// The probe's whole reason for being: without it the Launcher `exec`s this
/// box, bash is replaced by a process that exits, and the handshake is left
/// for nobody to answer.
#[test]
fn a_binary_that_starts_and_fails_afterwards_says_this_machine_will_not_run_it() {
    let plugin = PluginBox::with_a_binary_that_starts_and_fails();
    let mut session = Session::open(&plugin);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: this machine \
         refused to execute the flipchart binary the box carries for it. Nothing will appear on \
         screen, so do not offer the user a diagram - explain in prose instead. Reinstalling the \
         plugin is what brings it back."
    );
}

/// The choice, measured from the side it has to refuse: the box carries the
/// other Machine's binary and the Launcher does not reach for it.
#[test]
fn the_binary_of_the_other_machine_is_not_one_this_one_can_start() {
    let plugin = PluginBox::with_only_the_other_machines_binary();
    let mut session = Session::open(&plugin);

    assert_eq!(session.names_of_its_tools(), ["unavailable"]);
}

#[test]
fn on_a_machine_with_no_binary_the_handshake_is_answered_in_milliseconds() {
    let plugin = PluginBox::with_the_good_binary();
    let arm_linux = Machine::that_says_it_is("Linux", "aarch64");

    let start = Instant::now();
    let _session = Session::open_on(&plugin, &arm_linux);

    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn on_a_machine_with_no_binary_it_announces_a_single_tool() {
    let plugin = PluginBox::with_the_good_binary();
    let arm_linux = Machine::that_says_it_is("Linux", "aarch64");
    let mut session = Session::open_on(&plugin, &arm_linux);

    assert_eq!(session.names_of_its_tools(), ["unavailable"]);
}

/// What was found, said out loud: without it the User reads that something is
/// wrong and goes looking for the fault in their own setup, where it is not.
#[test]
fn on_a_machine_with_no_binary_the_warning_names_what_was_found() {
    let plugin = PluginBox::with_the_good_binary();
    let arm_linux = Machine::that_says_it_is("Linux", "aarch64");
    let mut session = Session::open_on(&plugin, &arm_linux);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: the box carries \
         no flipchart binary for this machine, which is Linux aarch64. Nothing will appear on \
         screen, so do not offer the user a diagram - explain in prose instead. Reinstalling the \
         plugin is what brings it back."
    );
}

#[test]
fn on_a_machine_with_no_binary_it_exits_with_zero_when_its_input_is_closed() {
    let plugin = PluginBox::with_the_good_binary();
    let arm_linux = Machine::that_says_it_is("Linux", "aarch64");
    let mut session = Session::open_on(&plugin, &arm_linux);

    session.closes_its_input();

    assert_eq!(session.exits_before(DEADLINE).code(), Some(0));
}

// ── The display, which only Linux is asked about ──────────────────────────────
//
// Over SSH, in a container, in a devcontainer there is no display and `winit`
// cannot create an event loop at all (ADR-0019). The Machine is the faked one
// here on both Machines, because the question belongs to Linux and the suite
// has to ask it from macOS too — and because the suite's own session, which
// does have a display on the runner, is not the one under test.

#[test]
fn on_linux_with_no_display_it_announces_a_single_tool() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let over_ssh = Machine::that_says_it_is("Linux", "x86_64").with_no_display();
    let mut session = Session::open_on(&plugin, &over_ssh);

    assert_eq!(session.names_of_its_tools(), ["unavailable"]);
}

/// The criterion behind the tool list: the binary is there, it answers the
/// probe, and the Launcher still does not hand its place over to it.
#[test]
fn on_linux_with_no_display_the_binary_is_never_reached() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let over_ssh = Machine::that_says_it_is("Linux", "x86_64").with_no_display();

    assert_eq!(what_the_launcher_left_on_stdout(&plugin, &over_ssh), "");
}

#[test]
fn on_linux_with_no_display_the_warning_names_the_missing_display() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let over_ssh = Machine::that_says_it_is("Linux", "x86_64").with_no_display();
    let mut session = Session::open_on(&plugin, &over_ssh);

    assert_eq!(
        the_warning_of(&mut session),
        "The flipchart is not available in this session and cannot draw anything: this machine \
         has no display, because neither DISPLAY nor WAYLAND_DISPLAY is set. Nothing will appear \
         on screen, so do not offer the user a diagram - explain in prose instead. Reinstalling \
         the plugin is what brings it back."
    );
}

#[test]
fn on_linux_with_no_display_the_handshake_is_answered_in_milliseconds() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let over_ssh = Machine::that_says_it_is("Linux", "x86_64").with_no_display();

    let start = Instant::now();
    let _session = Session::open_on(&plugin, &over_ssh);

    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn on_linux_with_no_display_it_exits_with_zero_when_its_input_is_closed() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let over_ssh = Machine::that_says_it_is("Linux", "x86_64").with_no_display();
    let mut session = Session::open_on(&plugin, &over_ssh);

    session.closes_its_input();

    assert_eq!(session.exits_before(DEADLINE).code(), Some(0));
}

#[test]
fn on_linux_with_an_x11_display_the_launcher_hands_its_place_over() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let x11 = Machine::that_says_it_is("Linux", "x86_64");

    assert!(what_the_launcher_left_on_stdout(&plugin, &x11).contains(THE_STAND_IN_SPEAKING));
}

#[test]
fn on_linux_with_a_wayland_display_the_launcher_hands_its_place_over() {
    let plugin = PluginBox::with_a_stand_in_for(THE_LINUX_BINARY);
    let wayland = Machine::that_says_it_is("Linux", "x86_64").with_the_display_in_wayland();

    assert!(what_the_launcher_left_on_stdout(&plugin, &wayland).contains(THE_STAND_IN_SPEAKING));
}

/// There is a window server wherever a User is logged in and `DISPLAY` means
/// nothing on macOS, so the absence of both variables is not news there.
#[test]
fn on_macos_the_absence_of_a_display_changes_nothing() {
    let plugin = PluginBox::with_a_stand_in_for(THE_MACOS_BINARY);
    let macos = Machine::that_says_it_is("Darwin", "arm64").with_no_display();

    assert!(what_the_launcher_left_on_stdout(&plugin, &macos).contains(THE_STAND_IN_SPEAKING));
}

#[cfg(target_os = "macos")]
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

// ── The probe, from the side that answers it ──────────────────────────────────
//
// The Launcher's question is only worth asking if the binary answers it with a
// zero and nothing else. `check` is the prior art —a subcommand that opens no
// window— and the probe goes one further: it does not speak either. That it
// answers at all is what says no window was opened, since a window takes the
// event loop and never gives it back.

fn the_binary_asked(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_flipchart"))
        .args(arguments)
        .output()
        .expect("the flipchart binary runs")
}

#[test]
fn the_binary_answers_the_probe_by_exiting_with_zero() {
    assert_eq!(the_binary_asked(&["probe"]).status.code(), Some(0));
}

#[test]
fn the_probe_writes_nothing_on_stdout() {
    let run = the_binary_asked(&["probe"]);

    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
}

#[test]
fn an_argument_that_is_neither_check_nor_probe_gets_the_usage_line() {
    let run = the_binary_asked(&["draw"]);

    let said = String::from_utf8_lossy(&run.stderr);
    assert!(said.starts_with("usage: flipchart"), "{said}");
}
