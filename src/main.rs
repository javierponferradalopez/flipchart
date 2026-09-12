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
        Some(_) => {
            eprintln!("usage: flipchart [check <diagram.mmd>...]");
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
