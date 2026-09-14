# The window comes to the front without taking the keyboard

**Status:** accepted · **Date:** 2026-09-04 · **Refined by** [0019](./0019-two-machines-one-box.md)

**Deferred startup.** The main thread **does not call `run_native` until the first
`show`**: it starts the server thread and blocks waiting on the channel. A session that
opens a repo and never asks for a diagram must not pay 97 MB for a window that does not
exist; until then the cost is on the order of 5 MB. And if the session dies without using
the flipchart, it exits without ever touching the GPU. (99.6 % of the memory cost is paid
on creating the event loop — ADR-0001.)

**The window is born at the front, without touching the keyboard, and without asking
permission.** *Dock* and *focus* looked like the same package, and they are **two
different calls**: on the first `show` the app goes up from `Accessory` to `Regular`
(`NSApplication.setActivationPolicy`), which is what gives it the Dock icon, and the
window is sent to the front with **`orderFrontRegardless`**, which moves the screen and
not the focus. **The keyboard stays where the user had it.** Measured on 2026-09-04
(report 18, harness in prototype 25): the window comes out **in front of the terminal
100 % of the time** with the **keyboard intact**, on the first `show` and on rebirth,
3 of 3 clean runs, and the Dock icon is preserved.

## Three pieces, and none of them is redundant

This is what makes it fragile and why it is written down here:

1. **`with_activate_ignoring_other_apps(false)`**, because **`winit` steals the keyboard
   on its own** by calling `activateIgnoringOtherApps(true)` when the event loop starts.
   Without disarming that, nothing else helps: changing the call **changes nothing** (it
   steals just the same, 3/3).
2. **Sending the window to the front only after the first frame**, because `eframe`
   creates its window hidden and shows it itself on painting. Getting ahead of it leaves
   the window **behind the terminal forever** — 3/3, which is *worse* than the outcome we
   were trying to avoid.
3. **`orderFrontRegardless` instead of `Visible(true)` + `Focus`.**

*All three are macOS. [0019](./0019-two-machines-one-box.md) says what is left of this page on
the other Machine: the keyboard is still never taken —on Wayland no client can take it— but the
window is **asked** to the front and not commanded, because no Wayland client can raise itself.
The Flipchart sends `RequestUserAttention` at the same instant this one sends
`orderFrontRegardless`, and what happens next is the compositor's to decide.*

The first two are internals of the versions the repo pins: **raising `eframe` forces
re-measuring this.**

## When it comes to the front, and when it does not

- **The window is sent to the front when it is born or reborn** —first `show`, or first
  `show` after the user closed it— and **never on an update**. A `show` over an open
  window repaints in ~55 ms without touching anything; jumping to the front every time the
  agent touches something up, while the user is typing in the terminal, is intolerable.
- And like any window of an app that is not active, **the flipchart goes behind as soon as
  the user activates their terminal**; the next `show` does not bring it back.
- **The title carries the session's working directory** — `Flipchart — flipchart` —, which is
  what the user has in mind when looking at two terminals.

**Closing the window hides, it does not kill.** In `eframe`, closing the window terminates
the application, which means ⌘W would take the MCP server with it and leave the agent
without tools mid-conversation. And on macOS a `winit` event loop **cannot be started
again** in the same process, so this is not a preference but an obligation: if it died,
there would never be a second window.

**Price accepted:** the window is born without the keyboard, so **⌘W does not reach it
until the user clicks it** — and the reflex ⌘W is eaten by the terminal, where it may
close a tab. It is the normal behaviour of any unfocused window, and it is paid only when
somebody wants to close it; stealing the keyboard was paid every time.

## Permission is not asked

The window appears without asking. What that would have bought is already bought, for
free: the agent **announces** before drawing, 8 of 8, *"I'll draw it on the flipchart"*
and calls. And the real consent is earlier — the user asked for it, or pasted the line
themself that commands it (ADR-0012). The term **Permission to open** is kept in the
glossary precisely so it does not get reinvented.

## Considered options

- **`activate()` as the suspect** — rejected by measurement, and it is the part that had to
  be measured. The keyboard thief was not the `activate()` this was blamed on: it is
  `winit`. Swapping the call without disarming `winit` changes nothing.
- **`Visible(true)` + `Focus`** — rejected. It brings the focus with it, which is the whole
  thing being avoided.
- **Starting the event loop up front instead of on the first `show`** — rejected. 97 MB and
  a GPU context for sessions that never draw, and two Claude Codes would be a swarm.
- **Letting ⌘W close the window** — rejected, and not by preference: in `eframe` it ends the
  process, and the event loop cannot be restarted, so there would never be a second window.

---

*The reports and prototypes cited above by number lived in `docs/research/` until
2026-09-04, when the repo became official and they were withdrawn. The number still
identifies them: they are recovered from the git history.*
