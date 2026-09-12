//! The Machine seam: the one place per-Machine behaviour lives.
//!
//! Every Machine answers the same four phrases, so `main.rs` and `viewer.rs`
//! call them and never learn which Machine they are on. What each phrase means
//! —and what it deliberately does not do on Linux— is ADR-0019.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    bring_the_window_forward, keep_awake_while_the_session_lasts, prepare_the_event_loop,
    stay_out_of_the_dock,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{
    bring_the_window_forward, keep_awake_while_the_session_lasts, prepare_the_event_loop,
    stay_out_of_the_dock,
};

#[cfg(target_os = "windows")]
compile_error!(
    "Windows is not a Machine the flipchart runs on: there is no \
     src/machine/windows.rs. Adding one means answering four phrases — \
     keep_awake_while_the_session_lasts, stay_out_of_the_dock, \
     bring_the_window_forward and prepare_the_event_loop — and reading \
     docs/adr/0010 before touching the window, because that page was measured \
     on macOS and nowhere else. Building without them would ship a binary \
     whose window never comes forward and which nobody promised."
);

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!(
    "This Machine is neither macOS nor Linux, and the flipchart has no \
     src/machine module for it. Adding one means answering four phrases — \
     keep_awake_while_the_session_lasts, stay_out_of_the_dock, \
     bring_the_window_forward and prepare_the_event_loop — and reading \
     docs/adr/0019. Building without them would ship a binary whose window \
     never comes forward and which nobody promised."
);
