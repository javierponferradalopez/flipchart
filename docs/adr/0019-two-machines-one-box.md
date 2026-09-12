# Two Machines, one box: the Launcher chooses

**Status:** accepted · **Date:** 2026-09-11 · **Refines** [0010](./0010-window-to-the-front-without-the-keyboard.md), [0013](./0013-the-plugin-is-the-only-install-path.md), [0014](./0014-the-launcher-never-fails.md)

**The flipchart runs on macOS and on Linux, from one box.** The zip carries both binaries —
`flipchart-macos`, the universal Mach-O, and `flipchart-linux-x86_64` — and the **Launcher
reads the Machine and hands its place over to the one that belongs there**. One catalog
entry, one digest, one install line: ADR-0013's install story does not change, and the User
never chooses.

The choice had to live in the Launcher because there is nowhere else for it. Measured in
ADR-0014: a marketplace entry's schema **has no platform field at all** — no `os`, no
`platform`, no `arch`, no `requires`. The catalog cannot route anybody anywhere.

**Windows is not here.** It is the next Machine and the seam is shaped for it, but nothing
is built, and a build for it stops at a `compile_error!` that names what is missing.

## What the User gets on Linux, and the two things that differ

The whole product: the Agent draws, the User goes back a sheet and forward again, marks with
the pencil, and the ink crosses back up the Wire. Two behaviours are not the macOS ones, and
both are decisions rather than gaps.

**The window is asked to the front, not commanded.** On Wayland no client can raise itself —
the compositor owns stacking, and there is no call for it. So on Linux the Flipchart sends
`RequestUserAttention`, which `winit` carries through `xdg_activation` on Wayland and through
the urgency hint on X11, at the same instant the macOS path sends `orderFrontRegardless`:
window born, first frame painted. Some compositors raise the window. Most flash a taskbar
entry. A few do nothing at all.

What survives everywhere is the promise ADR-0010 was really written for: **the keyboard stays
where the User had it.** Three of the four pieces of the macOS code exist to stop a steal —
ADR-0010 names `winit` as "the real thief" — and on Wayland the steal is impossible by
construction. The raise was the reachable half of that ADR on one Machine; it is best-effort
on the other, and that asymmetry is the price of the platform.

**Keep-awake is a macOS behaviour.** The call exists to defeat App Nap, which no other Machine
has; that it also disables idle system sleep came inside `NSActivityUserInitiated` as a side
effect and was never the want. Linux has an equivalent for the side effect — `Inhibit` on
`org.freedesktop.login1` — and it costs a D-Bus client in a box every macOS User also
downloads, to match a behaviour nobody asked for. On Linux a laptop that suspends suspends the
Host too: Claude Code and the flipchart go down together, and the session was ending anyway.

**Staying out of the Dock needs no equivalent.** On Linux the taskbar entry comes from the
window, and ADR-0010's deferred startup means there is no window — and no event loop — until
the first `show`. A Linux flipchart that never draws is invisible for free.

## The two silences the Launcher now breaks

ADR-0014 exists because a failed start **bans the server for fifteen minutes**, reinstalling
does not cure it, and all the User sees is `✘ failed`. Linux brings two ways to fail that the
Launcher could not see.

**A binary the loader refuses.** On macOS a Mach-O of the wrong architecture fails **at
`execve`**, so `shopt -s execfail` keeps Bash alive and the Unavailable server speaks. On
Linux the same class of failure does not work that way: a binary built against a newer glibc
is a valid ELF, `execve` **succeeds**, and the dynamic loader fails afterwards. By then Bash
has been replaced and there is nobody left to answer the handshake. So the Launcher **probes
before it hands over**: it runs the chosen binary once with an argument that starts it and
exits, output discarded, and `exec`s only on a zero exit. The cost is one process start, on
the order of milliseconds. Without it, ADR-0014's promise would be true on one Machine and
false on the other, in the same script.

**A session with no display.** Over SSH, in a container, in a devcontainer — normal ways to
run Claude Code on Linux — there is no display and `winit` cannot create an event loop at all.
The old code discarded that error (`let _ = eframe::run_native(…)`), so `main` returned and
**the process exited, taking the MCP server with it**, after the Agent had already been told
the `show` succeeded. Now: on Linux, if neither `DISPLAY` nor `WAYLAND_DISPLAY` is set, the
Launcher does not hand over — it runs the Unavailable server, whose message is written for
exactly this, and the session keeps a healthy server. And behind that, in the binary, the
`run_native` error stops being discarded: the main thread parks and the server keeps
answering.

**The gap that leaves, stated:** a parked process still accepts `show` calls that draw
nothing. Carrying that news back in the tool result is a change to the tool contract, and it
is not made here.

## The Machine seam

`src/mac.rs` becomes `src/machine/`, with `macos.rs` and `linux.rs` chosen by
`#[cfg(target_os)]`, exposing the phrases the product already uses. `main.rs` and `viewer.rs`
call those names and never learn which Machine they are on — including the event-loop builder,
where the `Accessory → Regular` move and the disarming of `activate_ignoring_other_apps` are
macOS-only and the Linux side contributes nothing.

**A third Machine stops the compiler.** Not silence, and not a no-op: silence would let a
Windows build produce a binary whose window never comes forward and which nobody promised.
ADR-0013 chose the words "not declared impossible: declared untested and unpromised", and a
`compile_error!` keeps that true in the compiler and not only in prose. When Windows comes,
the error is the checklist.

## What is published

**`x86_64` only, glibc 2.35**, built on the `ubuntu-22.04` runner — the oldest GitHub hosts.
That covers Ubuntu 22.04 and later, Debian 12 and current Fedora; it excludes RHEL 9, Debian
11, Ubuntu 20.04 and every ARM Linux, and those Users meet the Launcher's message instead of a
crash. An architecture is not free: the box is shared, so a binary nobody asked for is weight
every User carries. `aarch64` is an addition, not a rewrite.

The box is six files and still closed — the manifest, the `.mcp.json`, the Launcher, the
skill of ADR-0018, and the two binaries — copied one by one, so there is no way in for a
seventh. `[profile.release]` gains `strip = true`: the symbols are worth real megabytes across
three slices, and the crash reports we do not collect are all it costs.

**`make verify` runs on both Machines**, because a promise with no test on it is what ADR-0013
refuses everywhere else. One test stays macOS-only, with its reason beside it: the box whose
permissions cannot be fixed needs `chflags uchg`, and Linux has no unprivileged equivalent.
Packing stays on the macOS runner, which needs `lipo` and `codesign` for the Apple slices; the
Linux binary reaches it as an artifact.

## Considered options

- **One plugin entry per Machine** — rejected. Each User would download only what they run, and
  would have to choose correctly, with nothing in the schema to stop a wrong choice. It spends
  the install story — two lines, one name — to save 40 MB of disk, and turns an impossible
  failure into the User's fault.
- **X11 only, with Wayland out** — rejected. Wayland is the default session on current Fedora,
  Ubuntu and the Steam Deck, and XWayland would hand those Users a blurry window on HiDPI. The
  raise is worth less than the platform.
- **A true raise on X11**, reaching the window through `raw-window-handle` and restacking with
  `x11rb` — rejected for now, and available later as an addition. It buys the macOS behaviour
  for X11 Users at the price of re-opening ADR-0010's fragility on a second window system, and
  ADR-0010 already warns that raising `eframe` forces re-measuring it. Not a debt to take on
  twice before Linux has one User.
- **A D-Bus sleep inhibitor on Linux** — rejected, above.
- **Building for an older glibc in a container** such as `manylinux_2_28` — rejected. It buys
  distributions a Claude Code User is unlikely to be sitting at, and it is CI machinery to own.
- **Cross-compiling the Linux binary on the macOS runner** — rejected. A native build on the
  Machine that sets the floor is the one whose floor we can state.
- **Letting the Linux tests ride on the macOS runner** — rejected. A Linux-only regression would
  ship, and the failure we most fear — the event loop taking the MCP server with it — cannot
  happen on macOS at all.
