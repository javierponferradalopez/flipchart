use std::env;
use std::process::ExitCode;
use std::thread;

use flipchart::{
    check, keep_awake_while_the_session_lasts, open_at_the_first_show, serve, stay_out_of_the_dock,
    wire,
};

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.split_first() {
        None => {
            flipchart();
            ExitCode::SUCCESS
        }
        Some((subcommand, paths)) if subcommand == "check" && !paths.is_empty() => {
            check(paths);
            ExitCode::SUCCESS
        }
        // The Launcher's probe: it starts, it exits, and the zero it exits
        // with is the whole answer — that this Machine's loader took the
        // binary. On Linux one built against a newer glibc is a valid ELF,
        // `execve` succeeds and the loader fails afterwards, with Bash already
        // replaced and nobody left to answer the handshake (ADR-0019). No
        // window and not a byte on stdout: the Launcher throws the output away
        // and reads the exit code.
        Some((subcommand, nothing)) if subcommand == "probe" && nothing.is_empty() => {
            ExitCode::SUCCESS
        }
        Some(_) => {
            eprintln!("usage: flipchart [check <diagram.mmd>... | probe]");
            ExitCode::FAILURE
        }
    }
}

fn flipchart() {
    let _activity = keep_awake_while_the_session_lasts();
    stay_out_of_the_dock();

    let (viewer, commands) = wire();
    thread::spawn(move || serve(viewer));

    open_at_the_first_show(commands);
}
