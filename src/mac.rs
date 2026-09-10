use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSWindow};
use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, NSString};

const USER_INITIATED_AND_LATENCY_CRITICAL: u64 = 0x00FF_FFFF | (1 << 20) | 0xFF_0000_0000;

pub fn keep_awake_while_the_session_lasts() -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    NSProcessInfo::processInfo().beginActivityWithOptions_reason(
        NSActivityOptions(USER_INITIATED_AND_LATENCY_CRITICAL),
        &NSString::from_str("flipchart serves MCP for as long as the session lasts"),
    )
}

pub fn stay_out_of_the_dock(main_thread: MainThreadMarker) {
    NSApplication::sharedApplication(main_thread)
        .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

/// Puts the window in front **without activating the app**, which is what
/// leaves the keyboard where it was: where the user had it. *Dock* and *focus*
/// came in the same package —moving up to `Regular` and `activate()`—, but they
/// are two distinct calls, and this is the one that only moves the screen.
///
/// It only takes hold on a window the system already has mounted: called before
/// the first frame, the window stays **behind** the terminal. The one who waits
/// for that frame is the Viewer.
pub fn bring_the_window_forward() {
    if let Some(window) = the_window() {
        window.orderFrontRegardless();
    }
}

/// The only one there is: the Viewer shows one sheet at a time.
fn the_window() -> Option<Retained<NSWindow>> {
    application()?.windows().iter().next()
}

fn application() -> Option<Retained<NSApplication>> {
    Some(NSApplication::sharedApplication(MainThreadMarker::new()?))
}
