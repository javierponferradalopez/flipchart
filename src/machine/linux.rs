use eframe::EventLoopBuilderHook;

/// Nothing to hold awake. Keep-awake exists to defeat App Nap, which is macOS's
/// alone; that it also stopped idle sleep was a side effect and never the want
/// (ADR-0019). The guard is here so the session holds the same shape on every
/// Machine.
#[derive(Debug)]
pub struct Activity;

pub fn keep_awake_while_the_session_lasts() -> Activity {
    Activity
}

/// Nothing to stay out of. The taskbar entry comes from the window, and with
/// ADR-0010's deferred startup there is no window —and no event loop— until the
/// first `show`: a flipchart that never draws is invisible for free.
pub fn stay_out_of_the_dock() {}

/// Nothing to contribute. The `Accessory → Regular` move and the disarming of
/// `activate_ignoring_other_apps` are macOS's, and no other Machine has a say
/// in how the loop is built.
pub fn prepare_the_event_loop() -> Option<EventLoopBuilderHook> {
    None
}

/// On Wayland no client can raise itself, so here the window is **asked** to
/// the front and not commanded: `RequestUserAttention`, at the same instant the
/// macOS path sends `orderFrontRegardless`. That is not built yet — it arrives
/// with the rest of the Linux behaviour, and until then this Machine stays
/// silent rather than pretending.
pub fn bring_the_window_forward() {}
