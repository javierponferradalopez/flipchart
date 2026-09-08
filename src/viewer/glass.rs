//! The glass: the ink the user marks a sheet with, and the hand and the
//! pencil — the two ways of holding it (ADR-0016).

use eframe::egui;

/// One ink and one nib, fixed: the flipchart owns the style, and the ink is no
/// exception — ADR-0006's law, run in both directions. The moment the user
/// chooses colours, the image starts carrying information the agent must
/// interpret, and the answer is always no.
pub const INK: egui::Color32 = egui::Color32::from_rgb(196, 30, 30);

/// The width of the nib, in points of the screen: thick enough to be read over
/// a dense sheet, thin enough not to cover what it points at.
pub const NIB: f32 = 2.5;

/// The two ways to hold the glass. The hand pans, the pencil inks — they never
/// fight over the drag, which is why the choice is explicit and its pressed
/// state visible. The glass is born in the hand: marking is the exception, not
/// the habit.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Hand,
    Pencil,
}

/// One run of the pencil between its down and its up, in diagram coordinates.
pub type Stroke = Vec<egui::Pos2>;

/// The ink over one sheet: its strokes in **diagram coordinates**, so that what
/// was drawn on something stays on it across zoom and pan. It belongs to the
/// sheet the way the zoom does, and a replaced View starts with clean glass.
#[derive(Debug, Default)]
pub struct Glass {
    strokes: Vec<Stroke>,
}

impl Glass {
    /// The strokes there are, in the sheet's own coordinates.
    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    /// Pencil down: a stroke begins where it touches.
    pub fn began(&mut self, at: egui::Pos2) {
        self.strokes.push(vec![at]);
    }

    /// The drag's next point. A hand that holds still adds nothing — and a
    /// point with no stroke open before it starts none: the pencil was never
    /// put down.
    pub fn continued(&mut self, at: egui::Pos2) {
        if let Some(stroke) = self.strokes.last_mut()
            && stroke.last() != Some(&at)
        {
            stroke.push(at);
        }
    }

    /// Undo: takes back the last stroke, down to none. There is no eraser —
    /// undo covers mis-strokes, and removal of live marks is the redraw's
    /// business.
    pub fn undo(&mut self) {
        self.strokes.pop();
    }

    /// Whether the glass carries any ink.
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_stroke(glass: &mut Glass, points: &[egui::Pos2]) {
        let mut points = points.iter().copied();
        glass.began(points.next().expect("a stroke has a first point"));
        for point in points {
            glass.continued(point);
        }
    }

    #[test]
    fn a_sheet_is_born_with_clean_glass() {
        assert!(Glass::default().is_empty());
    }

    #[test]
    fn a_stroke_keeps_the_diagram_coordinates_it_was_given() {
        let mut glass = Glass::default();

        a_stroke(
            &mut glass,
            &[egui::pos2(10.0, 20.0), egui::pos2(30.0, 40.0)],
        );

        assert_eq!(
            glass.strokes(),
            &[vec![egui::pos2(10.0, 20.0), egui::pos2(30.0, 40.0)]]
        );
    }

    #[test]
    fn a_second_stroke_is_a_second_stroke() {
        let mut glass = Glass::default();

        a_stroke(&mut glass, &[egui::pos2(1.0, 1.0)]);
        a_stroke(&mut glass, &[egui::pos2(2.0, 2.0)]);

        assert_eq!(glass.strokes().len(), 2);
    }

    #[test]
    fn a_hand_that_holds_still_adds_nothing() {
        let mut glass = Glass::default();

        a_stroke(&mut glass, &[egui::pos2(5.0, 5.0), egui::pos2(5.0, 5.0)]);

        assert_eq!(glass.strokes(), &[vec![egui::pos2(5.0, 5.0)]]);
    }

    #[test]
    fn a_point_without_a_pencil_down_before_it_starts_nothing() {
        let mut glass = Glass::default();

        glass.continued(egui::pos2(1.0, 1.0));

        assert!(glass.is_empty());
    }

    #[test]
    fn undo_takes_back_the_last_stroke() {
        let mut glass = Glass::default();
        a_stroke(&mut glass, &[egui::pos2(1.0, 1.0)]);
        a_stroke(&mut glass, &[egui::pos2(2.0, 2.0)]);

        glass.undo();

        assert_eq!(glass.strokes(), &[vec![egui::pos2(1.0, 1.0)]]);
    }

    #[test]
    fn undo_goes_all_the_way_down_to_none() {
        let mut glass = Glass::default();
        a_stroke(&mut glass, &[egui::pos2(1.0, 1.0)]);

        glass.undo();
        glass.undo();

        assert!(glass.is_empty());
    }
}
