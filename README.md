# flipchart — an ephemeral whiteboard for agents

A temporary visual channel for an AI agent to explain itself: where it would otherwise
paint the graph in ASCII inside its answer, it draws it in a native window instead — and it
draws when you ask it to. It holds several views and shows one at a time; it dies with the
session and stores nothing.

## Requirements

- **macOS 11 or later**, Intel or Apple Silicon. Tested on macOS 26.6.2 arm64.
  Linux and Windows are neither tested nor promised.
- A version of Claude Code with plugin support.
- **Nothing else**: no Node, no Python, no browser, no Rust toolchain.

## Installation

It installs as a Claude Code plugin, and that is the only install path
([ADR 0013](./docs/adr/0013-the-plugin-is-the-only-install-path.md)). Two lines inside
Claude Code:

```
/plugin marketplace add https://raw.githubusercontent.com/javierponferradalopez/flipchart/main/marketplace.json
/plugin install flipchart@flipchart
```

And a third step that is **not optional**: paste this line into your `CLAUDE.md`.

```
Explain in prose. Draw on the flipchart with mcp__plugin_flipchart_flipchart__show
in exactly two cases: when you catch yourself starting an ASCII diagram, and when
I ask you to draw something.
```

Without it the flipchart sits installed and never gets used: on its own initiative the
agent never offers it —**0 out of 36 turns** measured
([ADR 0012](./docs/adr/0012-the-trigger-lives-outside-the-binary.md))— and paints the graph
in ASCII inside its answer.

The line asks for **less** than the one measured in 0012, which drew on any explanation of a
structure and so drew on almost every answer
([ADR 0017](./docs/adr/0017-drawing-is-the-exception.md)). Prose is the default and the two
cases are the whole list. If you would rather have a flipchart that volunteers more, add a
third case — the wide 0012 wording is the only one with a measurement behind it.

The box also ships one skill, `choosing-the-family`, which the agent reaches on its own
once it has decided to draw: which Mermaid family fits what it is explaining —a flowchart
for structure, a sequence for an exchange over time, a class diagram for types— and the
three lines that get a diagram rejected. It installs with the plugin and needs no line of
yours ([ADR 0018](./docs/adr/0018-the-box-carries-one-skill.md)).

That `mcp__plugin_flipchart_flipchart__show` is the name Claude Code presents the tool
under when flipchart arrives as a plugin: the host composes the server name as
`plugin:<plugin>:<server>`. Leave it as `mcp__flipchart__show` and you are naming a tool
that does not exist.

## Updating and uninstalling

```
/plugin update flipchart@flipchart
/plugin uninstall flipchart@flipchart
```

**The trailing `@flipchart` is not optional on `update`**: with the short name it answers
`Plugin "flipchart" not found`, even though it is installed and `/plugin` lists it. What
comes after the `@` is the marketplace, and it is called the same as the plugin. If you
would rather not type names, `/plugin` opens the menu and does the same.

`uninstall` takes the plugin's data with it, so there is no `rm -rf` to type. To turn it
off without uninstalling it, `/plugin`.

## What it does and what it does not

What the flipchart does today —one sheet at a time, the window does not steal the keyboard,
only the directed graph is tested, the style is its own— and what it will never do live in
[`docs/adr/`](./docs/adr/), one decision per file with the measurement behind it. Start
with [ADR 0015](./docs/adr/0015-what-this-product-is-not.md) and
[ADR 0004](./docs/adr/0004-the-honest-limit.md).

## Development

The language of the domain is in [`CONTEXT.md`](./CONTEXT.md); the decisions, in
[`docs/adr/`](./docs/adr/). How it is built, what the gate is and how it is published, in
[`CLAUDE.md`](./CLAUDE.md).
