//! The Viewer: the flipchart that shows the sheet the agent put at the front.

use eframe::egui;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

use crate::mac::bring_the_window_forward;
mod raster;
mod zoom;

use self::raster::{Rasterizer, Rendered, Scale};
use self::zoom::{MINIMUM_ZOOM, Zoom};
use crate::wire::{Command, Commands, DeckSnapshot};

/// Deferred start: the main thread stays on the channel and does not create the
/// event loop —nor the 97 MB it costs— until the agent asks for the first
/// drawing.
pub fn open_at_the_first_show(commands: Commands) {
    let Some(first) = wait_for_the_first_show(&commands) else {
        return;
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title())
            .with_inner_size([1200.0, 800.0]),
        // The move up from `Accessory` to `Regular` happens when the event loop
        // is built, which is exactly the first `show`. It has to be here and not
        // later: measured, an app that was born accessory never activates
        // —neither by changing the policy nor ten frames later— and the window
        // appears behind the terminal while the agent says it has drawn.
        //
        // And what `winit` does on its own at startup has to be disarmed:
        // `activateIgnoringOtherApps(true)`, which **steals the keyboard
        // mid-sentence** before anyone else gets a say. That is the real thief —
        // without this line, putting the window in front without activating the
        // app changes nothing.
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_activation_policy(ActivationPolicy::Regular);
            builder.with_activate_ignoring_other_apps(false);
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "flipchart",
        options,
        Box::new(move |cc| Ok(Box::new(Viewer::new(cc, commands, first)))),
    );
}

fn wait_for_the_first_show(commands: &Commands) -> Option<DeckSnapshot> {
    loop {
        match commands.recv()? {
            Command::Show(snapshot) if !snapshot.sheets.is_empty() => return Some(snapshot),
            Command::Show(_) => {}
            Command::SessionOver => return None,
        }
    }
}

fn title() -> String {
    match working_directory() {
        Some(directory) => format!("Flipchart — {directory}"),
        None => "Flipchart".to_string(),
    }
}

fn working_directory() -> Option<String> {
    let path = std::env::current_dir().ok()?;
    Some(path.file_name()?.to_string_lossy().into_owned())
}

/// A sheet on screen: what the Viewer remembers of it is its drawing and the
/// **zoom is its own** — the fit its size earns it and the choice the user made
/// over it, pan included. A replaced View inherits neither.
struct Sheet {
    number: u64,
    id: String,
    natural: Option<egui::Vec2>,
    painted: Option<(Scale, egui::TextureHandle)>,
    awaited: Option<Scale>,
    zoom: Zoom,
    pan: egui::Vec2,
}

impl Sheet {
    fn new(number: u64, id: String) -> Self {
        Self {
            number,
            id,
            natural: None,
            painted: None,
            awaited: None,
            zoom: Zoom::default(),
            pan: egui::Vec2::ZERO,
        }
    }
}

/// The sheets there are and which one is being looked at. The cursor is **local
/// to the Viewer** —the user's only control— and **the next `show` overwrites
/// it**: a `clear` leaves it looking where it was.
#[derive(Default)]
struct Deck {
    sheets: Vec<Sheet>,
    cursor: usize,
}

impl Deck {
    fn accept(&mut self, snapshot: &DeckSnapshot) {
        let looked_at = self.showing().map(|sheet| sheet.number);
        let front = snapshot.front.unwrap_or(0);
        let turned = snapshot
            .sheets
            .get(front)
            .is_some_and(|drawn| self.position(drawn.number).is_none());

        let mut previous = std::mem::take(&mut self.sheets);
        for drawn in &snapshot.sheets {
            let sheet = match previous.iter().position(|kept| kept.number == drawn.number) {
                Some(position) => previous.remove(position),
                None => Sheet::new(drawn.number, drawn.id.clone()),
            };
            self.sheets.push(sheet);
        }

        self.cursor = match looked_at {
            Some(number) if !turned => self.position(number).unwrap_or(front),
            _ => front,
        };
    }

    fn position(&self, number: u64) -> Option<usize> {
        self.sheets.iter().position(|sheet| sheet.number == number)
    }

    fn showing(&self) -> Option<&Sheet> {
        self.sheets.get(self.cursor)
    }

    fn showing_mut(&mut self) -> Option<&mut Sheet> {
        self.sheets.get_mut(self.cursor)
    }

    fn sheet_mut(&mut self, number: u64) -> Option<&mut Sheet> {
        self.sheets.iter_mut().find(|sheet| sheet.number == number)
    }

    fn back(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn forward(&mut self) {
        self.cursor = (self.cursor + 1).min(self.sheets.len().saturating_sub(1));
    }
}

struct Viewer {
    commands: Commands,
    rasterizer: Rasterizer,
    deck: Deck,
    window: Window,
    session_over: bool,
}

impl Viewer {
    fn new(cc: &eframe::CreationContext<'_>, commands: Commands, first: DeckSnapshot) -> Self {
        commands.this_is_where_to_wake_me(cc.egui_ctx.clone());
        let mut viewer = Self {
            commands,
            rasterizer: Rasterizer::spawn(cc.egui_ctx.clone()),
            deck: Deck::default(),
            window: Window::default(),
            session_over: false,
        };
        viewer.accept(first);
        viewer
    }

    fn accept(&mut self, snapshot: DeckSnapshot) {
        if !snapshot.sheets.is_empty() {
            self.window.a_show_arrived();
        }
        self.deck.accept(&snapshot);
        self.rasterizer.deck(
            snapshot
                .sheets
                .into_iter()
                .map(|drawn| (drawn.number, drawn.svg))
                .collect(),
        );
    }

    fn collect(&mut self, ctx: &egui::Context) {
        for rendered in self.rasterizer.collect() {
            match rendered {
                Rendered::Measured {
                    sheet: number,
                    natural,
                } => {
                    if let Some(sheet) = self.deck.sheet_mut(number) {
                        sheet.natural = Some(natural);
                    }
                }
                Rendered::Painted {
                    sheet: number,
                    scale,
                    image,
                } => {
                    let Some(sheet) = self.deck.sheet_mut(number) else {
                        continue;
                    };
                    let name = format!("{}#{number}", sheet.id);
                    let texture = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
                    sheet.awaited = None;
                    sheet.painted = Some((scale, texture));
                }
            }
        }
    }

    fn request(&mut self, zoom: f32, points_per_pixel: f32) -> Option<egui::TextureHandle> {
        let sheet = self.deck.showing_mut()?;
        let wanted = Scale::nearest(zoom * points_per_pixel);
        let up_to_date = sheet
            .painted
            .as_ref()
            .is_some_and(|(scale, _)| *scale == wanted);
        if !up_to_date && sheet.awaited != Some(wanted) {
            sheet.awaited = Some(wanted);
            self.rasterizer.paint(sheet.number, wanted);
        }

        let (_, texture) = sheet.painted.as_ref()?;
        Some(texture.clone())
    }

    /// The flipchart's header: the sheet, its name, two arrows and «sheet N of
    /// M». **No index** — an index is an administration control, and the user
    /// does not administer: they watch.
    fn header(&mut self, ui: &mut egui::Ui) {
        let sheets = self.deck.sheets.len();
        let cursor = self.deck.cursor;
        let Some(name) = self.deck.showing().map(|sheet| sheet.id.clone()) else {
            return;
        };
        ui.horizontal(|ui| {
            if ui.add_enabled(cursor > 0, egui::Button::new("‹")).clicked() {
                self.deck.back();
            }
            ui.strong(name);
            if ui
                .add_enabled(cursor + 1 < sheets, egui::Button::new("›"))
                .clicked()
            {
                self.deck.forward();
            }
            ui.weak(format!("sheet {} of {sheets}", cursor + 1));
        });
    }
}

/// The pan stops where the sheet stops having more to give: further than the
/// overflow and the room looks at the desk, not at the sheet.
fn panned_within(pan: egui::Vec2, size: egui::Vec2, room: egui::Vec2) -> egui::Vec2 {
    let overflow = (size - room).max(egui::Vec2::ZERO) / 2.0;
    egui::vec2(
        pan.x.clamp(-overflow.x, overflow.x),
        pan.y.clamp(-overflow.y, overflow.y),
    )
}

/// The window is born on the first `show` and is **reborn** on the next one
/// after a ⌘W, which hides it and does not kill it: in `eframe` closing it ends
/// the application —and with it the MCP server, leaving the agent without tools
/// mid-conversation— and on macOS a `winit` event loop cannot be started again,
/// so if it died there would never be a second window.
///
/// It is sent to the front when it is born and when it is reborn, and **never on
/// an update**: jumping to the front every time the agent touches something up,
/// while the user is typing in the terminal, is intolerable.
///
/// **Being born waits for the first frame.** `eframe` creates its window hidden
/// and shows it itself, with `makeKeyAndOrderFront`, as soon as it has painted
/// something; sending it to the front before that is useless —measured, it stays
/// **behind** the terminal and there it stays—. On the rebirth it waits for
/// nothing: the window has already painted.
#[derive(Debug, Default)]
struct Window {
    open: bool,
    asked_for: bool,
    painted: bool,
}

impl Window {
    fn a_show_arrived(&mut self) {
        self.asked_for = true;
    }

    fn hidden(&mut self) {
        self.open = false;
    }

    fn a_frame_was_painted(&mut self) {
        self.painted = true;
    }

    fn born(&mut self) -> bool {
        if !self.painted {
            return false;
        }
        std::mem::take(&mut self.asked_for) && !std::mem::replace(&mut self.open, true)
    }

    /// The `show` has already arrived and the window does not exist yet: we have
    /// to come back through here as soon as it has painted, or it stays behind
    /// forever.
    fn waiting_to_be_born(&self) -> bool {
        self.asked_for && !self.painted
    }
}

/// Fit: shrink, never enlarge. Enlarging lies — it puts a three-node diagram at
/// 128 % next to a twenty-node one at 27 %.
fn fit(natural: egui::Vec2, room: egui::Vec2) -> f32 {
    if natural.x <= 0.0 || natural.y <= 0.0 {
        return 1.0;
    }
    (room.x / natural.x)
        .min(room.y / natural.y)
        .clamp(MINIMUM_ZOOM, 1.0)
}

impl eframe::App for Viewer {
    /// With the window hidden `eframe` does not run an egui pass, so
    /// **everything that is not painting lives here**: it is the only thing that
    /// keeps being called when the window is away, and without it a ⌘W would
    /// leave it never to be reborn.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window.hidden();
        }

        while let Some(command) = self.commands.try_recv() {
            match command {
                Command::Show(snapshot) => self.accept(snapshot),
                Command::SessionOver => self.session_over = true,
            }
        }

        if self.window.born() {
            bring_the_window_forward();
        } else if self.window.waiting_to_be_born() {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.window.a_frame_was_painted();
        self.collect(&ctx);

        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            if self.session_over {
                ui.heading("Session over");
                ui.weak("The conversation that prompted this flipchart has ended.");
                return;
            }
            if self.deck.sheets.is_empty() {
                ui.weak("The flipchart is empty.");
                return;
            }
            self.header(ui);
            ui.add_space(6.0);

            let room = ui.available_size();
            let Some(sheet) = self.deck.showing() else {
                return;
            };
            let Some(natural) = sheet.natural else { return };
            let mut zoom = sheet.zoom;
            let mut pan = sheet.pan;
            let fitted = fit(natural, room);

            let (response, painter) = ui.allocate_painter(room, egui::Sense::click_and_drag());
            let pinch = ctx.input(|input| input.zoom_delta());
            let wheel = ctx.input(|input| input.smooth_scroll_delta.y);
            if response.hovered() && (pinch != 1.0 || wheel != 0.0) {
                zoom.scrolled(pinch * (wheel / 200.0).exp2(), fitted);
            }
            if response.double_clicked() {
                zoom.reset();
                pan = egui::Vec2::ZERO;
            }
            if response.dragged() {
                pan += response.drag_delta();
            }

            let size = natural * zoom.of(fitted);
            pan = panned_within(pan, size, room);
            if let Some(sheet) = self.deck.showing_mut() {
                sheet.zoom = zoom;
                sheet.pan = pan;
            }
            if let Some(texture) = self.request(zoom.of(fitted), ctx.pixels_per_point()) {
                let corner = response.rect.min + (room - size) / 2.0 + pan;
                painter.image(
                    texture.id(),
                    egui::Rect::from_min_size(corner, size),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Drawn, wire};

    #[test]
    fn the_window_is_born_with_the_first_show() {
        let mut window = Window::default();

        window.a_show_arrived();
        window.a_frame_was_painted();

        assert!(window.born());
    }

    #[test]
    fn the_first_show_does_not_front_a_window_that_has_not_painted_yet() {
        let mut window = Window::default();

        window.a_show_arrived();

        assert!(!window.born());
    }

    #[test]
    fn a_show_that_arrived_before_the_first_frame_stays_pending() {
        let mut window = Window::default();

        window.a_show_arrived();
        window.born();

        assert!(window.waiting_to_be_born());
    }

    #[test]
    fn a_window_already_at_the_front_leaves_nothing_pending() {
        let window = born();

        assert!(!window.waiting_to_be_born());
    }

    #[test]
    fn a_show_on_a_standing_window_does_not_front_it_again() {
        let mut window = born();

        window.a_show_arrived();

        assert!(!window.born());
    }

    #[test]
    fn after_a_cmd_w_the_next_show_reborns_the_window_and_there_it_does_go_to_the_front() {
        let mut window = born();

        window.hidden();
        window.a_show_arrived();

        assert!(window.born());
    }

    #[test]
    fn a_hidden_window_is_not_reborn_without_a_show() {
        let mut window = born();

        window.hidden();

        assert!(!window.born());
    }

    fn born() -> Window {
        let mut window = Window::default();
        window.a_show_arrived();
        window.a_frame_was_painted();
        window.born();
        window
    }

    #[test]
    fn a_sheet_that_does_not_fit_shrinks_until_it_does() {
        let zoom = fit(egui::vec2(2400.0, 800.0), egui::vec2(1200.0, 800.0));

        assert_eq!(zoom, 0.5);
    }

    #[test]
    fn a_sheet_that_fits_is_left_at_its_size_and_is_not_enlarged() {
        let zoom = fit(egui::vec2(300.0, 200.0), egui::vec2(1200.0, 800.0));

        assert_eq!(zoom, 1.0);
    }

    #[test]
    fn the_side_that_fits_worst_rules() {
        let zoom = fit(egui::vec2(1200.0, 3200.0), egui::vec2(1200.0, 800.0));

        assert_eq!(zoom, 0.25);
    }

    #[test]
    fn a_window_shrunk_to_nothing_does_not_ask_for_a_scale_of_zero() {
        let zoom = fit(egui::vec2(1200.0, 800.0), egui::vec2(1.0, 1.0));

        assert_eq!(zoom, MINIMUM_ZOOM);
    }

    #[test]
    fn the_title_carries_the_working_directory() {
        let expected = format!("Flipchart — {}", working_directory().unwrap());

        assert_eq!(title(), expected);
    }

    #[test]
    fn the_first_show_is_what_brings_the_window_out_and_a_clear_alone_does_not() {
        let (viewer, commands) = wire();
        viewer.send(flipchart(vec![], None));
        viewer.send(flipchart(vec![drawn(1, "current")], Some(0)));

        let first = wait_for_the_first_show(&commands).unwrap();

        assert_eq!(first.sheets.len(), 1);
    }

    #[test]
    fn a_session_that_dies_without_drawing_anything_opens_no_window() {
        let (viewer, commands) = wire();
        drop(viewer);

        assert!(wait_for_the_first_show(&commands).is_none());
    }

    #[test]
    fn a_session_that_ends_before_the_first_show_opens_no_window_to_say_goodbye() {
        let (viewer, commands) = wire();
        viewer.say_goodbye();

        assert!(wait_for_the_first_show(&commands).is_none());
    }

    fn drawn(number: u64, id: &str) -> Drawn {
        Drawn {
            number,
            id: id.to_string(),
            svg: "<svg/>".to_string(),
        }
    }

    fn flipchart(sheets: Vec<Drawn>, front: Option<usize>) -> DeckSnapshot {
        DeckSnapshot { sheets, front }
    }

    fn three_sheets() -> Deck {
        let mut deck = Deck::default();
        deck.accept(&flipchart(
            vec![
                drawn(1, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
            ],
            Some(2),
        ));
        deck
    }

    fn looking_at(deck: &Deck) -> &str {
        &deck.showing().expect("there is a sheet at the front").id
    }

    #[test]
    fn a_show_leaves_its_sheet_at_the_front() {
        let deck = three_sheets();

        assert_eq!(looking_at(&deck), "variant B");
    }

    #[test]
    fn the_back_arrow_steps_back_one_sheet() {
        let mut deck = three_sheets();

        deck.back();

        assert_eq!(looking_at(&deck), "variant A");
    }

    #[test]
    fn the_forward_arrow_returns_to_the_next_sheet() {
        let mut deck = three_sheets();
        deck.back();

        deck.forward();

        assert_eq!(looking_at(&deck), "variant B");
    }

    #[test]
    fn the_first_sheet_has_no_previous() {
        let mut deck = three_sheets();

        deck.back();
        deck.back();
        deck.back();

        assert_eq!(looking_at(&deck), "current");
    }

    #[test]
    fn the_last_sheet_has_no_next() {
        let mut deck = three_sheets();

        deck.forward();

        assert_eq!(looking_at(&deck), "variant B");
    }

    #[test]
    fn the_next_show_overwrites_the_users_cursor() {
        let mut deck = three_sheets();
        deck.back();
        deck.back();

        deck.accept(&flipchart(
            vec![
                drawn(1, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
                drawn(4, "variant C"),
            ],
            Some(3),
        ));

        assert_eq!(looking_at(&deck), "variant C");
    }

    #[test]
    fn removing_a_sheet_that_was_not_being_looked_at_leaves_the_user_where_they_were() {
        let mut deck = three_sheets();
        deck.back();

        deck.accept(&flipchart(
            vec![drawn(2, "variant A"), drawn(3, "variant B")],
            Some(1),
        ));

        assert_eq!(looking_at(&deck), "variant A");
    }

    #[test]
    fn removing_the_sheet_being_looked_at_moves_to_the_one_at_the_front() {
        let mut deck = three_sheets();
        deck.back();

        deck.accept(&flipchart(
            vec![drawn(1, "current"), drawn(3, "variant B")],
            Some(1),
        ));

        assert_eq!(looking_at(&deck), "variant B");
    }

    #[test]
    fn a_live_sheet_keeps_what_it_already_had_drawn() {
        let mut deck = three_sheets();
        deck.sheets[0].natural = Some(egui::vec2(300.0, 200.0));

        deck.accept(&flipchart(
            vec![
                drawn(1, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
                drawn(4, "variant C"),
            ],
            Some(3),
        ));

        assert_eq!(deck.sheets[0].natural, Some(egui::vec2(300.0, 200.0)));
    }

    #[test]
    fn replacing_a_view_gives_a_new_sheet_that_gets_drawn_again() {
        let mut deck = three_sheets();
        deck.sheets[0].natural = Some(egui::vec2(300.0, 200.0));

        deck.accept(&flipchart(
            vec![
                drawn(4, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
            ],
            Some(0),
        ));

        assert_eq!(deck.sheets[0].natural, None);
    }

    #[test]
    fn the_emptied_flipchart_is_left_with_no_sheets_to_show() {
        let mut deck = three_sheets();

        deck.accept(&flipchart(vec![], None));

        assert!(deck.showing().is_none());
    }

    #[test]
    fn a_replaced_view_arrives_at_the_fit_and_inherits_no_zoom() {
        let mut deck = three_sheets();
        deck.sheets[0].zoom.scrolled(2.0, 0.5);

        deck.accept(&flipchart(
            vec![
                drawn(4, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
            ],
            Some(0),
        ));

        assert_eq!(deck.sheets[0].zoom.of(0.5), 0.5);
    }

    #[test]
    fn a_show_over_another_view_leaves_the_looked_at_sheets_zoom_alone() {
        let mut deck = three_sheets();
        deck.back();
        deck.sheets[1].zoom.scrolled(2.0, 0.25);

        deck.accept(&flipchart(
            vec![
                drawn(1, "current"),
                drawn(2, "variant A"),
                drawn(3, "variant B"),
            ],
            Some(0),
        ));

        assert_eq!(deck.sheets[1].zoom.of(0.25), 0.5);
    }
}
