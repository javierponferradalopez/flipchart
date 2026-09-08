//! The user's zoom: the choice over the fit, and it belongs to the sheet.

/// The floor every zoom in the Viewer shares, the fit's and the user's alike:
/// below it a sheet is not small, it is gone.
pub const MINIMUM_ZOOM: f32 = 0.05;

/// Enlargement is the user's to ask for, and four times is as close as
/// inspecting gets: past it the pixels run out before the curiosity does.
pub const MAXIMUM_ZOOM: f32 = 4.0;

/// The user's zoom of a sheet: an override of the fit — and unlike the fit it
/// **may enlarge**, because an explicit choice lies about nothing. A sheet is
/// born fitted: a scroll or a pinch turns it, a double-click hands it back, and
/// a replaced View starts over.
#[derive(Debug, Default, Clone, Copy)]
pub struct Zoom {
    chosen: Option<f32>,
}

impl Zoom {
    /// The zoom the sheet is at: the user's choice where they made one, the
    /// fit where they have not.
    pub fn of(&self, fit: f32) -> f32 {
        self.chosen.unwrap_or(fit)
    }

    /// Scroll or pinch: turn whatever the sheet is at — the fit included — by
    /// `by`, inside the floor and the cap.
    pub fn scrolled(&mut self, by: f32, fit: f32) {
        self.chosen = Some((self.of(fit) * by).clamp(MINIMUM_ZOOM, MAXIMUM_ZOOM));
    }

    /// Double-click: back to the fit, whose zoom was never the user's to keep.
    pub fn reset(&mut self) {
        self.chosen = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_is_born_at_the_fit() {
        assert_eq!(Zoom::default().of(0.27), 0.27);
    }

    #[test]
    fn a_scroll_turns_the_zoom_by_the_factor() {
        let mut zoom = Zoom::default();

        zoom.scrolled(2.0, 0.25);

        assert_eq!(zoom.of(0.25), 0.5);
    }

    #[test]
    fn the_users_zoom_may_enlarge_past_one() {
        let mut zoom = Zoom::default();

        zoom.scrolled(4.0, 0.5);

        assert_eq!(zoom.of(0.5), 2.0);
    }

    #[test]
    fn the_users_zoom_never_goes_below_the_minimum() {
        let mut zoom = Zoom::default();

        zoom.scrolled(0.001, 1.0);

        assert_eq!(zoom.of(1.0), MINIMUM_ZOOM);
    }

    #[test]
    fn the_users_zoom_never_goes_above_the_cap() {
        let mut zoom = Zoom::default();

        zoom.scrolled(1_000_000.0, 1.0);

        assert_eq!(zoom.of(1.0), MAXIMUM_ZOOM);
    }

    #[test]
    fn the_choice_stays_made_across_turns_of_the_same_factor() {
        let mut zoom = Zoom::default();
        zoom.scrolled(2.0, 0.25);

        zoom.scrolled(2.0, 0.25);

        assert_eq!(zoom.of(0.25), 1.0);
    }

    #[test]
    fn the_choice_is_absolute_and_does_not_follow_the_room() {
        let mut zoom = Zoom::default();
        zoom.scrolled(2.0, 0.5);

        assert_eq!(zoom.of(0.9), 1.0);
    }

    #[test]
    fn a_double_click_hands_the_sheet_back_to_the_fit() {
        let mut zoom = Zoom::default();
        zoom.scrolled(3.0, 0.25);

        zoom.reset();

        assert_eq!(zoom.of(0.25), 0.25);
    }
}
