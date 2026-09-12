use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A session driven from outside the process, the way the Host drives it: over
/// stdio, one JSON-RPC line at a time.
struct Session {
    process: Child,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

impl Session {
    /// Really initialised —with the answer read—, which is what makes sure the
    /// server thread is up and listening for the two deaths.
    fn open() -> Self {
        Self::started(Command::new(env!("CARGO_BIN_EXE_flipchart")))
    }

    /// The session ADR-0019 names: over SSH, in a container, in a devcontainer.
    /// `winit` cannot create an event loop with no display at all, so this is
    /// the flipchart that is asked to draw and cannot.
    ///
    /// Linux only, and not because the promise is Linux's: macOS has a window
    /// server wherever a user is logged in and no unprivileged way to take it
    /// away from a process, so `run_native` cannot be made to fail there. This
    /// one is measured where it can be measured.
    #[cfg(target_os = "linux")]
    fn with_no_display_to_draw_on() -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_flipchart"));
        command.env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY");
        Self::started(command)
    }

    fn started(mut command: Command) -> Self {
        let mut process = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the flipchart binary starts");
        let output = BufReader::new(process.stdout.take().unwrap());
        let mut session = Self {
            process,
            output,
            next_id: 1,
        };
        session.initialise();
        session
    }

    fn initialise(&mut self) {
        self.asks(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        )
        .expect("the server answers the initialize");
        self.notifies("notifications/initialized");
    }

    /// One `show`, answered — which is the Agent being told the drawing
    /// happened. What comes after it is the question: whether there is still
    /// anybody to talk to.
    fn draws_a_diagram(&mut self) {
        self.asks(
            "tools/call",
            json!({
                "name": "show",
                "arguments": {
                    "view_id": "current",
                    "diagram": "flowchart LR\n  A[One] --> B[Two]\n"
                }
            }),
        )
        .expect("the server answers the show");
    }

    fn is_still_answering(&mut self) -> bool {
        self.asks("ping", json!({})).is_some()
    }

    /// The answer to the request, or **nothing**: a process that took the MCP
    /// server down with it is read here as the stream ending where an answer
    /// should have been.
    fn asks(&mut self, method: &str, params: Value) -> Option<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let input = self.process.stdin.as_mut().unwrap();
        writeln!(input, "{request}").ok()?;
        input.flush().ok()?;
        self.answer_to(id)
    }

    fn answer_to(&mut self, id: u64) -> Option<Value> {
        loop {
            let mut line = String::new();
            if self.output.read_line(&mut line).ok()? == 0 {
                return None;
            }
            let message: Value = serde_json::from_str(line.trim()).expect("readable JSON-RPC");
            if message.get("id") == Some(&json!(id)) {
                return Some(message);
            }
        }
    }

    fn notifies(&mut self, method: &str) {
        let notification = json!({ "jsonrpc": "2.0", "method": method });
        let input = self.process.stdin.as_mut().unwrap();
        writeln!(input, "{notification}").unwrap();
        input.flush().unwrap();
    }

    fn closes_its_input(&mut self) {
        drop(self.process.stdin.take());
    }

    fn receives_sigint(&mut self) {
        let killed = Command::new("kill")
            .args(["-INT", &self.process.id().to_string()])
            .status()
            .expect("kill -INT runs");
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
        panic!("the process outlived its session");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

const MARGIN: Duration = Duration::from_secs(5);

#[test]
fn the_eof_on_stdin_ends_the_process() {
    let mut session = Session::open();

    session.closes_its_input();

    assert!(session.exits_before(MARGIN).success());
}

#[test]
fn the_sigint_ends_the_process() {
    let mut session = Session::open();

    session.receives_sigint();

    assert!(session.exits_before(MARGIN).success());
}

#[test]
fn the_server_survives_having_drawn() {
    let mut session = Session::open();

    session.draws_a_diagram();

    assert!(session.is_still_answering());
}

/// The failure ADR-0019 stopped discarding. A `run_native` that cannot start
/// used to return from `open_at_the_first_show`, return from `main`, and take
/// the MCP server with it — after the Agent had been told the `show` succeeded,
/// so the next tool call reads EOF and the Agent is left without tools
/// mid-conversation. Now the main thread parks and the server keeps answering.
///
/// The gap this does not close, and ADR-0019 accepts: the process that answers
/// here draws nothing.
#[cfg(target_os = "linux")]
#[test]
fn the_server_survives_a_window_that_could_not_be_opened() {
    let mut session = Session::with_no_display_to_draw_on();

    session.draws_a_diagram();

    assert!(session.is_still_answering());
}
