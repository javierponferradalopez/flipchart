---
name: choosing-the-family
description: Pick the Mermaid family before drawing on the flipchart, and avoid the rejections that cost a turn. Use when about to call mcp__plugin_flipchart_flipchart__show, when deciding between a flowchart, a class, a sequence, a state or an ER diagram, and when a show came back rejected for undeclared nodes.
---

# Choosing the family

The family follows **what you are explaining**, not what you usually draw. A flowchart
is the right answer for structure and the wrong one for an exchange over time, and
reaching for it by default spends the user's attention on boxes that carry less than the
sentence did.

| What you are explaining | Write |
|---|---|
| Structure, layers, what depends on what | `flowchart`, with `subgraph` per layer |
| Types, their fields, and how they relate | `classDiagram` |
| An exchange over time between parts | `sequenceDiagram` |
| Data and the relations between records | `erDiagram` |
| A lifecycle: what a thing can be, and what moves it | `stateDiagram-v2` |
| Services and where they run | `architecture-beta` |

All six are measured against this flipchart and draw. Pick the row, then write the
smallest diagram that carries the point.

## Three lines that get a diagram rejected

A rejection draws nothing and costs the turn, and all three causes are cheap to avoid.

**Every id in a relation carries a label when any other one does.** This is the
flipchart's own rule, and it is the most common rejection.

- `A --> B` alone — drawn. A graph of bare ids is honest.
- `API[API Layer] --> Db` — rejected. `Db` has no label next to one that has it.

**In a `classDiagram`, give every class a body, or give none of them one.** A bare
`class PositionedScene` standing next to a `class Order { +place() }` is read as
undeclared and rejected.

**In a `stateDiagram-v2`, do not write `[*]`.** It manufactures `__start_root__` and
`__end_root__`, which appear nowhere in your source, and the diagram is rejected. Name
the first and last states instead — `Empty --> Drawn` — which draws.

## What the flipchart decides, so you do not spend tokens on it

**Direction is imposed**: diagrams are laid out left to right, and a `TD` or `TB` in your
source is ignored and warned about. Write the header without a direction.

**Style is the flipchart's**: colors, shapes, `classDef`, `style` and any HTML in a label
are emptied or warned about. Write meaning — what there is and how it relates — and
nothing that asks for pixels.

## Families to spend no turn on

`journey`, `mindmap` and `block-beta` are **rejected** as they stand: each manufactures
ids that are not in your source.

`C4Context`, `gitGraph` and `timeline` draw a picture, but the flipchart's honest limit
finds nothing in them to inspect, so nothing catches an invented node. `C4Context` also
loses its title.
