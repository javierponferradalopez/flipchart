# The launcher never fails

**Status:** accepted · **Date:** 2026-09-02 · **Refined by** [0019](./0019-two-machines-one-box.md)

> **The Launcher never fails.** It answers the handshake **always**, in milliseconds,
> binary or no binary, network or no network, and exits with 0.

The reason is not the timeout, which is a probability: it is the veto, which is a failure
mode **the product cannot repair**.

## What the host does, and what a failure looks like

Measured with Claude Code 2.1.228:

- **30 000 ms hard** for the handshake, and **the plugin cannot extend them** — not through
  `settings.json` nor with `timeout` / `startupTimeout` / `initializationTimeout` in the
  `.mcp.json`. Only the user can, with `MCP_TIMEOUT`. It cuts at 30.0 s and sends `SIGTERM`
  2 s later.
- **A failed startup vetoes the server for 15 minutes**: it is recorded in
  `~/.claude/mcp-needs-auth-cache.json` (TTL 900 000 ms) and **the following sessions do not
  even launch the process**. It applies to **any** failure, not just the timeout — a `command`
  that exits with an error in 64 ms leaves the same mark. **Reinstalling the plugin does not
  cure it**; the only thing that clears the entry is a successful connection, which is exactly
  what the veto prevents. And it is a price **exclusive to plugin stdio servers**.
- **What the user sees is two words:** `✘ failed` inside `/mcp`, and nothing in the welcome.
  The `command`'s stderr is captured but goes to the debug log: **the error message we write
  does not arrive**. The only text with a cause is in `claude mcp list`.
- **Startup does not block the first turn.** With the launcher taking 10 s —well under the
  deadline— the turn ran without the tool. You do not have to take 30 s to end up with no
  flipchart: taking longer than the user takes to type is enough.

**No network code in the MVP.** There is no download and no clone: when the binary does not
work, the Launcher **says so and does not try to fix it**. Worst case accepted: *reinstall the
plugin*.

## The Launcher and the Unavailable server

The `.mcp.json`'s `command` is a shell script, in **bare bash 3.2**. What it does:

1. `chmod +x` on the binary in its own directory (`${CLAUDE_PLUGIN_ROOT}`). It is a
   **backup**, not the mechanism: the host preserves `0755` when the zip comes from Info-ZIP
   (ADR-0013), but nothing in its schema promises that.
2. `exec` the binary.
3. If it cannot, **it does not die: it stays and talks itself** as the **Unavailable server**.

The Unavailable server is the same file: it enters a loop reading stdin and answering JSON-RPC
by hand —MCP's stdio transport is JSON per line, with no `Content-Length` framing— and handles
`initialize`, the `initialized` notification, `tools/list` and `tools/call`.

**It announces exactly one tool, with no arguments**, whose *description* carries the message:
the flipchart is not operational, this is what was found —binary missing, no execute permission,
or a different architecture— and the plugin needs reinstalling.

**Its flagship case, measured by absence:** a marketplace entry's schema **has no platform field
at all** —no `os`, no `platform`, no `arch`, no `requires`. Nothing stops someone on Linux
running `/plugin install flipchart`, getting a Mach-O extracted and having `exec` return
`ENOEXEC`. For that user the Unavailable server is not a degraded mode: **it is the only message
they will ever receive.** Behind them come quarantine, a `chmod` failing on a read-only
filesystem, a half-finished extraction on a full disk, and manual deletion.

*[0019](./0019-two-machines-one-box.md) takes the flagship case away and puts two others in its
place. The box now carries a Linux binary too, so the Launcher **chooses** before it hands over,
and the User on Linux gets a flipchart instead of a message. What it cannot choose away is a
binary the loader refuses —on Linux `execve` succeeds and the loader fails afterwards, where
`execfail` cannot reach— and a session with no display at all. The Launcher probes for the first
and looks at `DISPLAY` and `WAYLAND_DISPLAY` for the second; both end here, at the Unavailable
server, which is why it is the piece that did not have to change.*

## Considered options

- **Returning an error when the binary is not there** — rejected. A failed startup vetoes the
  server for 15 minutes, reinstalling does not cure it, and the user sees two words. Never
  failing is a first-order constraint, not robustness for its own sake.
- **Announcing the two real tools from the Unavailable server** — rejected on behaviour: the
  agent would **try to draw and fail**, and the user would discover the problem in the middle of
  something else, with a turn spent. With one tool that is never called, the model knows from
  the first moment that the flipchart is not available and **does not offer it**.
- **Announcing nothing** — rejected. That is absolute silence, which is the one failure this
  product cannot afford. A broken channel that announces itself beats one that gets discovered.
- **Python, Perl or `jq` instead of bare bash** — rejected, for four reasons. `python3` is not a
  dependency: `/usr/bin/python3` is a Command Line Tools shim, so a machine without Xcode has no
  Python. `perl` and `ruby` are there, but Apple has spent years warning that scripting runtimes
  are leaving the system. `jq` **is** Apple's and signed, but it is recent, and leaning on it
  would **put a floor under macOS from our side** — and that floor is set by `eframe`/`winit`,
  not by us. And the good reason: without `jq` the message `id` has to come out of text matching,
  which is fragile **unless it is made impossible by construction**. The only message that can
  carry a nested `"id"` is a `tools/call`, and **with a single tool that takes no arguments** no
  message that gets as far as being parsed ever contains a nested `"id"`: the line's first
  `"id":` is always the JSON-RPC one. **The fragility is not mitigated, it is eliminated** — which
  is why the two decisions (one tool, no `jq`) fit together.
