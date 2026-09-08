# The trigger lives outside the binary

**Status:** accepted · **Date:** 2026-09-03 · **Refined by** [0017](./0017-drawing-is-the-exception.md)

This is not a documentation detail: it is the product's main trigger, and it is outside
the binary knowingly.

## The agent does not offer the flipchart on its own. Measured

**0 of 36 turns**, with four wordings of the tool description —the one decided on; without
the asymmetry clause; without any rule about asking permission; and one that attacks
drawing in ASCII head-on— and 22 sessions over a repo with real layers. What it does
instead is paint the graph **in ASCII inside its answer**; and the ASCII was not the cause
but the symptom: turning it off with a `CLAUDE.md` that forbids it, the flipchart **still
went unused** and it fell back to prose with lists.

**Two things do trigger it, both at 100 %:**

| Channel | Result |
|---|---|
| One line in the project instructions | **8 attempts in 5 turns** |
| The user naming the tool | 9 in 7 |

With the line, the flagship case came out with nobody asking for it: `Dependencias
actuales`, `Quién sabe de líneas hoy`, `Después · variante A`, `Después · variante B`.

**The degraded mode is accepted as the product's correct behaviour:** installed and
without the line, the flipchart is never used on the agent's initiative. **Price written
down:** the product has two qualities of installation, with the line and without it, and
the one that works depends on the user pasting something.

*Method note, for whoever repeats the measurement:* in Claude Code 2.1.228
`--allowedTools` **does not grant MCP tools in `-p` mode**, so the instrument is the
`tool_use` count in the history, not the server's log. Neither do project settings —which
require having trusted the directory first— nor a `permissions.allow` passed with
`--settings`: **the only thing that grants them is a `PreToolUse` hook answering
`permissionDecision: allow`** (report 20). And the subject is today's model: the
actionable result is *the text is not enough*, not a figure.

## The recommended line is the last step of the installation

**flipchart does not write anybody's `CLAUDE.md`** — the box carries the `.mcp.json`, the
Launcher and the binary, and nothing else. What it does is **ask the user for it from the
installation documentation**, as the last step and not as an optional appendix. It is
advice in a README, not a channel the plugin controls — and the advice works where the
description does not.

It is also the **discovery channel**: if the reliable trigger is the user naming the
flipchart, the user has to know it exists, and the product's only channel is the
installation.

## The tool name is composed by the host, and it is not the one you would assume

Measured on 2026-09-04 against the installed `v0.1.0` release, Claude Code 2.1.228
presents it as **`mcp__plugin_flipchart_flipchart__show`**: the `mcp__` prefix carries the
server name inside, which for a plugin is `plugin:<plugin>:<server>` with the `:` swapped
for `_`. `mcp__flipchart__show` **names a tool that does not exist**. It comes out the same
installed from the marketplace and loaded with `--plugin-dir`, so it does not depend on the
marketplace: only on the plugin and its server both being called `flipchart`.

And the line with that name is **verified end to end**: with the real plugin and the
flagship case above —the user describes the move they are considering and asks to
understand the dependencies, without asking for a drawing— the agent loaded the tool and
drew `Dependencias actuales` on the first turn (report 20).

## Consequences

**Renaming the plugin or its server is no longer trivial.** The composed name lives inside
the line people have pasted into their own `CLAUDE.md`, and that file is not ours to write.
A rename leaves every existing installation naming a tool that does not exist, silently and
with no way to warn. (Renaming the *repo* is trivial and unrelated.)

## Considered options

- **Making the tool description carry a minimum on its own** — rejected, and it is
  literally what was tried four times: 0 of 36 turns. Not a wording problem.
- **flipchart writing the user's project instructions** — rejected. It is out of the
  product's scope: what it does is *recommend* the line from the installation
  documentation. Advice, not a channel the plugin controls.
- **Treating the line as an optional appendix in the README** — rejected. Without it the
  flipchart is installed and never used; it is the last step of the installation, and it is
  written as not optional.

---

*The reports and prototypes cited above by number lived in `docs/research/` until
2026-09-04, when the repo became official and they were withdrawn. The number still
identifies them: they are recovered from the git history.*
