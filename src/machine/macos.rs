use eframe::EventLoopBuilderHook;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSWindow};
use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, NSString};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

const USER_INITIATED_AND_LATENCY_CRITICAL: u64 = 0x00FF_FFFF | (1 << 20) | 0xFF_0000_0000;

pub fn keep_awake_while_the_session_lasts() -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    NSProcessInfo::processInfo().beginActivityWithOptions_reason(
        NSActivityOptions(USER_INITIATED_AND_LATENCY_CRITICAL),
        &NSString::from_str("flipchart serves MCP for as long as the session lasts"),
    )
}

pub fn stay_out_of_the_dock() {
    let main_thread = MainThreadMarker::new().expect("main() runs on the main thread");
    NSApplication::sharedApplication(main_thread)
        .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

/// The move up from `Accessory` to `Regular` happens when the event loop is
/// built, which is exactly the first `show`. It has to be here and not later:
/// measured, an app that was born accessory never activates —neither by
/// changing the policy nor ten frames later— and the window appears behind the
/// terminal while the agent says it has drawn.
///
/// And what `winit` does on its own at startup has to be disarmed:
/// `activateIgnoringOtherApps(true)`, which **steals the keyboard mid-sentence**
/// before anyone else gets a say. That is the real thief — without this line,
/// putting the window in front without activating the app changes nothing.
pub fn prepare_the_event_loop() -> Option<EventLoopBuilderHook> {
    Some(Box::new(|builder| {
        builder.with_activation_policy(ActivationPolicy::Regular);
        builder.with_activate_ignoring_other_apps(false);
    }))
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
