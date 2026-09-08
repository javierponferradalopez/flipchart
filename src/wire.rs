//! The wire: the in-memory channel between the two threads of the Flipchart
//! process. The server thread owns the state and hands the whole deck over; the
//! main thread only draws what arrives — and pushes the user's ink back up
//! (ADR-0016).
//!
//! It is more than a `Sender` because of what ADR-0001 measured: with the window
//! covered macOS **stops** the event loop, so whoever sends also has to wake it.
//! The return lane needs no waking: it is push-only, and the server comes to
//! read on the agent's own time.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

/// An already drawn sheet, exactly as it arrives. Whoever draws it numbers it:
/// a `show` over an id that already existed gives a new sheet with the same
/// name.
#[derive(Debug)]
pub struct Drawn {
    pub number: u64,
    pub id: String,
    pub svg: String,
}

/// The whole flipchart every time: the sheets **in creation order** and **which
/// one is at the front**. The Viewer decides neither of those two things.
#[derive(Debug)]
pub struct DeckSnapshot {
    pub sheets: Vec<Drawn>,
    pub front: Option<usize>,
}

/// What the server sends the Viewer: draw the whole flipchart, or say goodbye —
/// which happens once and is the last thing to come across.
#[derive(Debug)]
pub enum Command {
    Show(DeckSnapshot),
    SessionOver,
}

/// The user's ink coming back up: the picture of one whole sheet with the marks
/// baked in, pushed by the Viewer on each completed stroke. Push, not pull: the
/// server folds it into its inbox and the Viewer remembers nothing.
#[derive(Debug)]
pub struct Ink {
    pub view_id: String,
    pub png: Vec<u8>,
}

/// The in-memory channel from the server to the Viewer. It is more than a
/// `Sender`: it wakes the event loop, which macOS does not slow down but
/// **stops** when the window is covered — and covered is the normal case, with
/// the user in their terminal.
#[derive(Debug, Clone)]
pub struct Wire {
    commands: Sender<Command>,
    ink: InkWaiting,
    awake: Waker,
}

impl Wire {
    pub fn send(&self, snapshot: DeckSnapshot) {
        self.tell(Command::Show(snapshot));
    }

    /// The ink that came up while nobody was looking. The server drains it at
    /// every turn of the agent's —`show`, `clear`, `marks`— so that each push
    /// is folded against the sheet it was drawn over, not the one on screen by
    /// the time it is read.
    pub(crate) fn take_the_ink(&self) -> Vec<Ink> {
        let pending = self
            .ink
            .lock()
            .expect("the ink is never held across a panic");
        std::iter::from_fn(|| pending.try_recv().ok()).collect()
    }

    /// The Viewer's goodbye, sent by the server thread because it is the only
    /// one that knows what time it is. It says whether there was a window to
    /// say goodbye to: a session that never drew anything has nobody to tell.
    pub fn say_goodbye(&self) -> bool {
        self.tell(Command::SessionOver);
        self.awake.get().is_some()
    }

    fn tell(&self, command: Command) {
        let _ = self.commands.send(command);
        if let Some(ctx) = self.awake.get() {
            ctx.request_repaint();
        }
    }
}

#[derive(Debug)]
pub struct Commands {
    commands: Receiver<Command>,
    ink: Sender<Ink>,
    awake: Waker,
}

impl Commands {
    pub(crate) fn recv(&self) -> Option<Command> {
        self.commands.recv().ok()
    }

    pub(crate) fn try_recv(&self) -> Option<Command> {
        self.commands.try_recv().ok()
    }

    /// The Viewer's half of the return lane: one completed stroke, one push.
    /// It wakes nobody — there is nobody to wake; the server comes to read when
    /// the agent asks.
    #[allow(
        dead_code,
        reason = "no GUI produces ink yet; until that ticket, only the tests play the Viewer"
    )]
    pub(crate) fn push_ink(&self, ink: Ink) {
        let _ = self.ink.send(ink);
    }

    /// The Viewer arms the waker with its own context once the event loop
    /// exists. Until it does there is nobody to wake, and `Wire` reads that
    /// absence to know whether there was ever a window.
    pub(crate) fn this_is_where_to_wake_me(&self, ctx: egui::Context) {
        let _ = self.awake.set(ctx);
    }
}

type Waker = Arc<OnceLock<egui::Context>>;
type InkWaiting = Arc<Mutex<Receiver<Ink>>>;

pub fn wire() -> (Wire, Commands) {
    let (commands, pending) = channel();
    let (ink, ink_waiting) = channel();
    let awake: Waker = Arc::default();
    (
        Wire {
            commands,
            ink: Arc::new(Mutex::new(ink_waiting)),
            awake: awake.clone(),
        },
        Commands {
            commands: pending,
            ink,
            awake,
        },
    )
}
