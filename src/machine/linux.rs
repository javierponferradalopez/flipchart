use eframe::EventLoopBuilderHook;
use eframe::egui;

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

/// The window is **asked** to the front and not commanded: on Wayland no client
/// can raise itself —the compositor owns stacking, and there is no call for
/// it—, so what is sent is `RequestUserAttention`, at the same instant the
/// macOS path sends `orderFrontRegardless`. `winit` carries it through
/// `xdg_activation` on Wayland and through the urgency hint on X11. Some
/// compositors raise the window, most flash a taskbar entry, a few do nothing
/// at all (ADR-0019).
///
/// `Critical` and not `Informational`: the Agent has just drawn a sheet it is
/// about to talk about, which is the one moment the flipchart has anything to
/// say. What survives everywhere is the promise ADR-0010 was written for —
/// **the keyboard stays where the User had it** —, and here it survives by
/// construction: attention is a request, and no request takes focus.
///
/// The window is put back on screen first, and that is what makes the **rebirth**
/// of ADR-0010 happen on this Machine: closing hides the window and the next
/// `show` is meant to bring it back, but a hidden window has nothing to flash
/// and nobody to ask on its behalf. On the birth there is nothing to put back —
/// `eframe` has already shown it, which is what the Viewer waited for.
pub fn bring_the_window_forward(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
        egui::UserAttentionType::Critical,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_asked_to_the_front_and_not_commanded() {
        let ctx = egui::Context::default();

        bring_the_window_forward(&ctx);

        assert!(what_the_viewport_was_told(&ctx).contains(
            &egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical)
        ));
    }

    #[test]
    fn a_window_that_was_closed_is_put_back_on_screen_to_be_asked_at_all() {
        let ctx = egui::Context::default();

        bring_the_window_forward(&ctx);

        assert!(what_the_viewport_was_told(&ctx).contains(&egui::ViewportCommand::Visible(true)));
    }

    /// A command sent outside a frame waits for the next one, so a frame is run
    /// to read what the Machine asked for. Its textures are dropped here and
    /// `epaint` panics on a delta nobody applied, which is the price of running
    /// a frame with no painter behind it.
    fn what_the_viewport_was_told(ctx: &egui::Context) -> Vec<egui::ViewportCommand> {
        let mut frame = ctx.run_ui(egui::RawInput::default(), |_| {});
        frame.textures_delta.clear();
        frame
            .viewport_output
            .into_values()
            .flat_map(|viewport| viewport.commands)
            .collect()
    }
}
