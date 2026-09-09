# Ephemeral flipchart for agents

A temporary visual channel for an AI agent to explain itself. The agent expresses
meaning —what there is and how it relates— and the system decides how it looks;
never the other way round.

This is a glossary. The reasons behind these terms live in [`docs/adr/`](./docs/adr/),
linked where a term rests on a decision.

## Language

### The actors

**Agent**:
The one who writes the diagram and turns the page. The flipchart is its channel for
explaining itself, so it is the only one who shows and clears.
_Avoid_: model, assistant, client

**User**:
The one the explanation is for. They read, go back one sheet, and mark — they never
write the diagram.
_Avoid_: human, viewer — the Viewer is a piece, not a person

**Host**:
The program the agent runs inside, and which launches the Flipchart process as a child
and talks to it over stdio.
_Avoid_: IDE, editor, Claude Code — the term is the role, not one product

### The channel

**Flipchart** — `flipchart` in code:
The temporary visual channel as a whole. It holds N Views and shows one at a time with
no index of the others, and the agent is the one who turns the page.
_Avoid_: canvas, board, whiteboard; and as identifiers, `canvas`, `board`, `whiteboard`
_Why_: [0009](./docs/adr/0009-one-sheet-no-index.md), [0010](./docs/adr/0010-window-to-the-front-without-the-keyboard.md), [0012](./docs/adr/0012-the-trigger-lives-outside-the-binary.md), [0017](./docs/adr/0017-drawing-is-the-exception.md)

**View**:
One of the N named representations that coexist on the flipchart at once. Its `id` is
also its visible name, and showing over that `id` replaces it.
_Avoid_: diagram, scene, tab
_Why_: [0008](./docs/adr/0008-two-tools-and-only-two.md)

**Ephemeral**:
That it dies with the MCP session: not saved, not exported, not outliving it. An MCP
session is **not** a conversation — `/clear` ends the conversation and the flipchart
survives.
_Avoid_: temporary, volatile
_Why_: [0011](./docs/adr/0011-the-mcp-session-rules.md)

### The return channel

**Mark**:
What the User draws over a sheet: ink on glass, never an edit to the diagram underneath.
It crosses back to the Agent as a picture of the sheet with the ink on it — the Agent
speaks meaning and the User speaks ink — and it stays until the Agent has resolved it.
_Avoid_: annotation, note, comment, drawing, edit — each of those names another thing
_Why_: [0016](./docs/adr/0016-the-return-channel.md)

### The layers

**Visual Protocol**:
The subset of Mermaid the agent is allowed to write: meaning —nodes, relationships,
containment— with every id declared and nothing that asks for pixels.
_Avoid_: format, payload, DSL, and "Mermaid" on its own — the whole language is not the protocol
_Why_: [0002](./docs/adr/0002-mermaid-as-the-language.md)

**Family**:
The kind of diagram a VisualDocument is written as —`flowchart`, `classDiagram`,
`sequenceDiagram`, `erDiagram`, `stateDiagram-v2`, `architecture-beta`—, declared by its
first line. The agent chooses it from what it is explaining; six of them are measured and
the rest are untested.
_Avoid_: type, kind, diagram type, chart type
_Why_: [0002](./docs/adr/0002-mermaid-as-the-language.md), [0018](./docs/adr/0018-the-box-carries-one-skill.md)

**VisualDocument**:
The complete semantic state of a View, written in the Visual Protocol. It is the truth
of the View, not a copy of something earlier.
_Avoid_: model, scene, graph, source

**Layout Engine**:
The piece that turns a VisualDocument into a PositionedScene. It decides alone: neither
the agent nor the user takes part, and it is not swappable.
_Avoid_: positioning engine, autolayout
_Why_: [0003](./docs/adr/0003-one-layout-engine-pinned.md), [0007](./docs/adr/0007-layout-direction-is-imposed.md)

**PositionedScene**:
A VisualDocument with its geometry already resolved: what is left before a single pixel
is painted.
_Avoid_: layout, positioned scene

**Drawing Surface**:
What the picture is finally painted with inside the Viewer. What it is made of is
invisible to everything upstream.
_Avoid_: renderer, canvas, painter

**Delivery surface**:
How the Viewer reaches the user's eyes: a native window on their machine. Distinct from
the Drawing Surface, which is what the painting is done with inside.
_Avoid_: surface on its own
_Why_: [0015](./docs/adr/0015-what-this-product-is-not.md)

### The honest limit

**Honest limit** — `diagram/honest_limit.rs` in code:
The boundary the flipchart does not cross by drawing: past it, it does not draw worse,
it stops and says so. **What is seen in excess is rejected; what is seen short is drawn
and warned about.**
_Avoid_: limit on its own, truncation, degradation
_Why_: [0004](./docs/adr/0004-the-honest-limit.md), [0005](./docs/adr/0005-a-rejection-is-a-result.md)

**Phantom node**:
The node the language invents from an id that appears only in a relationship, so it
comes out empty next to labelled ones. Rejected: it reads as something we know less
about, not as the error it is.
_Avoid_: implicit node, auto-creation, typo

**Apocryphal node**:
The node whose id the agent never wrote, manufactured by whoever parses on giving up on
a line. Rejected, and worse than the Phantom because **it carries a label**.
_Avoid_: fallback node, made-up node, hallucination, garbage

**Literal markup**:
The markup the agent writes inside a label's text and that reaches the drawing exactly
as written. Warned about and not rejected: it is one word in excess, not a node.
_Avoid_: HTML on its own, garbage, markup without qualification
_Why_: [0006](./docs/adr/0006-the-flipchart-owns-the-style.md)

### The live pieces

**Flipchart process**:
The only process there is, and which is both MCP server and Viewer. It splits the two
roles between threads: the main one draws, the secondary one serves and decides when
the process exits.
_Avoid_: daemon, server on its own
_Why_: [0001](./docs/adr/0001-one-process-two-threads-no-ipc.md)

**MCP server**:
The role that exposes the tools to the agent and **owns the state** of the flipchart.
The truth lives here.
_Avoid_: backend, host — the Host is the program that launches us

**Viewer** — `viewer.rs` in code:
The role that receives a scene and paints it in its own window. Dumb by design: it keeps
nothing, and closing its window loses nothing.
_Avoid_: frontend, client, app, page

**Wire** — `wire.rs` in code:
The in-memory channel between the two roles, and the only thing the two threads share:
the deck goes down it, the ink comes back up. It is more than a `Sender`: it also wakes
the event loop, which macOS stops —not slows— while the window is covered. Whoever holds
it needs to know nothing about the Viewer, which is why the Viewer depends on nobody.
_Avoid_: channel — that is the Flipchart as a whole; also bus, queue, IPC, socket
_Why_: [0001](./docs/adr/0001-one-process-two-threads-no-ipc.md)

**Launcher** — `launcher.sh` in code:
What the Host invokes so that the Flipchart process exists. It does not bring the
executable, only makes it usable — and it never fails.
_Avoid_: installer, wrapper, shim, startup script
_Why_: [0013](./docs/adr/0013-the-plugin-is-the-only-install-path.md), [0014](./docs/adr/0014-the-launcher-never-fails.md)

**Unavailable server**:
The Launcher's face when there is no Flipchart process to hand its place over to. It
states that the flipchart is not operational and nothing else, because silence is the
only failure this product cannot afford.
_Avoid_: stub, fallback, degraded mode, server on its own
_Why_: [0014](./docs/adr/0014-the-launcher-never-fails.md)

### Terms that do not exist

Kept so they are not reinvented, not because they name anything.

**Permission to open**:
The yes the flipchart was going to ask for before the first show. It was never built:
the agent announces and draws, and waits for nothing.
_Why_: [0010](./docs/adr/0010-window-to-the-front-without-the-keyboard.md), [0012](./docs/adr/0012-the-trigger-lives-outside-the-binary.md)

**Renderer**:
A contaminated word: it blurs the Layout Engine with the Drawing Surface, distinct
stages that today arrive in the same piece. Name the one you mean.
