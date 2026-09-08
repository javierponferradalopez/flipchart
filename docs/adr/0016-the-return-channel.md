# The return channel: the user speaks ink, the agent reads with its eyes

**Status:** accepted · **Date:** 2026-09-08

The flipchart was one-directional by declaration: the agent explains, "the user observes"
(ADR-0015). But explaining goes both ways — the user needs to **point**: circle this,
underline that, hint a movement. This ADR opens the return channel, and in doing so it
**amends three**: ADR-0015 (the user now marks, and never writes the diagram), ADR-0008
(a third tool exists, and an image crosses back — "not one byte of the SVG" now reads
"not one byte *of ours*; the user's ink crosses whole"), and ADR-0001's reading of the
Wire (the deck goes down and **the ink comes up**; one process, two threads, no IPC,
all hold).

## The law, run symmetric

The front page's law governs the downward direction: *the agent expresses meaning and
the system decides how it looks — never the other way round.* The return channel gets
its own law, symmetric and not a mirror: **the agent speaks meaning, the user speaks
ink — and the model has eyes.** Each side talks in its native medium. The user draws
pixels; nothing resolves them into ids; the agent reads the picture.

## The payload is the picture, not the meaning

The `marks` tool returns, per View with undelivered marks, **an image of the whole
sheet with the ink baked in, at the sheet's natural size** — not the viewport (the
user's zoom is theirs), never the ink alone (ink without the diagram is pixels without
context, the downward sin committed upward).

That the ink crosses **unresolved** is a decision, not a shortcut. The Viewer has no
geometry: what crosses the Wire today is an SVG string that `raster.rs` turns into
pixels, so resolving *"circled `orders`"* would require per-node boxes that exist
nowhere — parsed out of the rendered SVG (fragile; mmdr's groups are ADR-0015's known
wound) or carried from the Layout Engine (a new payload, a new truth). And any gesture
vocabulary — circle, underline, arrow — is a classifier that must be written and can be
wrong, covering three shapes while the user's eyes handle a scribble, a written word, a
bent arrow, a question mark. The model is multimodal; the flagship host serves it
vision. **A model without eyes gets nothing from `marks`**, and that is accepted.

The cost is paid once per marking episode, not per frame: the image crosses **when
read**, and only undelivered marks cross.

## The mark's life

- Ink is drawn → the Viewer pushes the image up the Wire → the server's inbox holds,
  per View, **the latest marked image and an undelivered flag**.
- **Reading informs, it does not delete.** `marks` delivers the undelivered images and
  clears their flags; a second call returns only what arrived since.
- **Unread marks survive a replace** — delivered later with the annotation that they
  refer to the sheet *as it was before the last replace*. ADR-0014's law: silence is
  the only failure this product cannot afford; the channel must not eat the user's
  words because the agent drew again.
- **The redraw is the reply.** Delivered marks die at the agent's next `show` over that
  View: the new diagram answers the circles, and replied-to ink has no glass to live
  on. Marks answered in prose only simply linger — invisible to `marks`, costless —
  until the same end.
- **`clear` kills the marks of what it clears**, read or not — a chosen consequence:
  the agent can wipe the inbox with a `clear()`, as it can wipe the Views.
- **The session over flushes everything.** Ephemeral means ephemeral.

## The Wire carries the ink up

On each **completed stroke** the Viewer composites the ink over the natural-size raster
and pushes the image up; the server **replaces** the previous one for that View. Push,
not pull-at-read: a pull would make the Wire a request-response protocol and the Viewer
a keeper of state that answers questions — the first step toward the IPC ADR-0001
exists to prevent. With the push, the server still owns all state and the Viewer stays
dumb: it renders and forwards, it does not remember.

## Considered options

- **Meaning, not pixels** (hit-tested node and edge ids, a circle/underline/arrow
  vocabulary) — rejected. Needs geometry nobody has, a classifier nobody needs, and it
  covers less than the eyes it replaces. It also dies on the mark that touches nothing:
  every rule for inventing its meaning manufactures an Apocryphal target.
- **The marks riding along on the next `show`/`clear`** — rejected. Marks are
  information **created after the last call**; ADR-0008's free-ride reasoning does not
  reach them. Reading by firing a side-effecting `show` is a query wearing a costume,
  and a session that never draws again would never deliver.
- **Reading consumes** — rejected. Marks outlive their reading until resolved, or the
  agent that reads twice loses its own inbox.
- **An explicit acknowledgement of resolution** (`marks(view_id, resolved)`) —
  rejected. A fourth concept the agent forgets to call, reintroducing through the back
  door the lingering the rule exists to prevent. The redraw is the reply.
- **Pull at read time** — rejected. Turns the Wire into a protocol and the Viewer into
  a stateful answerer; buys nothing felt.
- **A stamp palette ("move", "delete", "improve") instead of free ink** — rejected. It
  turns the pencil into a form; the user's sentence in the terminal carries the intent,
  the ink carries the pointing.
