# The box carries one skill: which family to draw

**Status:** accepted · **Date:** 2026-09-09

**Refines** [0013](./0013-the-plugin-is-the-only-install-path.md), whose box was closed at
four files with `no skills/` written into it, and extends the family measurement of
[0002](./0002-mermaid-as-the-language.md). The vehicle does not change: a fifth file
inside the same verified zip.

## The trigger was solved and the choice was not

0012 and 0017 spent themselves on **whether** the agent draws, and settled it outside the
binary: the pasted line. Nothing anywhere ever said **what** to draw. The tool description
takes "Mermaid source" and names no family; the recommended line names two cases and no
family; the only place the families are enumerated is 0002, which the agent never reads.

With no signal, the agent falls back to the most frequent Mermaid shape there is and paints
`flowchart` at everything — including the exchanges over time and the type relations that
have their own family. 0002 had the symptom written down from the other side and read it as
reassurance: *the agent picks unmeasured families on its own,* `sequenceDiagram`, **4 of 17
spontaneous diagrams**. The other thirteen are the finding.

## Why 0013's `no skills/` does not forbid this one

0013's reason is one sentence and it is about ownership: *a zero-toll skill is exactly the
one the model **cannot** invoke, so it cannot own anything.* True, and untouched here. **This
skill owns nothing.** It does not trigger the flipchart and is not offered as a way of
reaching it; the line of 0017 still does that, alone. It fires **after** the agent has already
decided to draw, on a decision that until now had nothing behind it. The box stays closed
against anything that would claim the trigger.

## The six families are measured, and three of them are new

Measured on 2026-09-09 with `flipchart check` over a bank of fourteen families, against the
build at `v0.2.0`.

| Family | Outcome |
|---|---|
| `flowchart` / `graph` | drawn |
| `classDiagram` | drawn |
| `sequenceDiagram` | drawn, `alt` and `loop` included |
| `erDiagram` | drawn, attributes included |
| `architecture-beta` | drawn, without icons (0002) |
| `requirementDiagram` | drawn |
| `stateDiagram-v2` | **drawn without `[*]`**; rejected with it |
| `journey`, `mindmap`, `block-beta` | rejected |
| `C4Context`, `gitGraph`, `timeline` | drawn, with an **empty `Graph`** |

Three results are worth keeping apart from the list:

**`sequenceDiagram` and `erDiagram` draw.** 0002 promised one family and called the rest
untested; two of the three an agent explaining code needs most are now measured, and the
skill can name them without promising anything that is not true.

**`stateDiagram-v2` is rejected by `[*]` and by nothing else.** 0004 had the cause
—`__start_root__`, a legitimate synthetic id the traceable-node rule cannot tell from an
apocryphal one— but not the shape of the way out: naming the first and last states draws.
The workaround is in the skill; the rule's own fate stays open where 0004 left it.

**`mindmap` is a rejection 0004 does not list**, and its cause is a third one: the ids come
back with the label's spaces turned into underscores, so `Visual Protocol` reaches the
`Graph` as `Visual_Protocol` and matches nothing in the source.

The `Graph` of `C4Context`, `gitGraph` and `timeline` comes back empty while the SVG does
not —2786, 2922 and 7964 bytes, all three with `<text>` inside—, which is 0002's *none of
the 23 comes out empty* seen from the other end: they draw, and **the honest limit inspects
nothing in them.** No rule of 0004 runs over a diagram with no nodes.

## The price

**Context load on every drawing turn, not on every turn.** The skill's description stays
loaded so the agent can reach it on its own; the body only arrives when it fires. That is
the shape of the toll and it is the reason the content is not in the tool description, which
loads whole in every session that lists the tools, drawing or not.

**Whether the agent reaches it is not measured.** The skill fires by its description, and
0012's finding about this product is precisely that text in the agent's context does not
make it act: 0 of 36 turns, four wordings. That measurement was about triggering the
*flipchart*, and this description competes for a much narrower moment —the agent has already
decided to draw— so the prior does not transfer whole. It does not transfer to zero either.
**If the skill turns out not to fire, the fallback is the one channel this product has
measured at 100 %:** a clause in the recommended line naming it, at the cost 0017 wrote down.

**It goes stale against mmdr and nothing warns.** Every row of that table is a fact about a
pinned layout engine (0003). If the pin moves, the skill keeps saying what was true — the
same failure the pin exists to make rare, now with a second place to update.

**A `flipchart:choosing-the-family` appears in the user's menu.** 0013 rejected `commands/`
because a `/flipchart:*` promises a control over the flipchart that the user does not have.
This one promises no control: it is advice about drawing, addressed to the agent, and typing
it does nothing to the window.

## The skill name is composed by the host, and here it is the one you would assume

Measured on 2026-09-09 against the packed box loaded with `--plugin-dir`, Claude Code
2.1.228 presents the skill as **`flipchart:choosing-the-family`**: `<plugin>:<skill>`, with
the plugin's own name already in front. **So the name in the frontmatter does not carry the
`flipchart` token**, which would come out as `flipchart:flipchart-choosing-the-family`.

This is worth writing down because it is 0012's finding the other way round, and the same
mistake is available in both directions. There the assumption was that the host would leave
the name alone and it composed it —`mcp__flipchart__show` names a tool that does not
exist—; here the temptation is to compose it ourselves and the host already has. The two
forms are not even the same shape: an MCP tool arrives as
`mcp__plugin_<plugin>_<server>__<tool>` and a skill as the plain `<plugin>:<skill>`.

The pull towards putting it in anyway has a precedent behind it: of the official plugins that
ship skills, most name the skill after the plugin, and `hookify` —the one whose skill name
would otherwise be generic— carries its product token in the frontmatter `name`
(`writing-hookify-rules`) while keeping the directory generic (`writing-rules`). What makes
that unnecessary here is the prefix: `family` is a word of this glossary and nowhere else,
and the prefix is what says whose glossary.

## Consequences

The box is five files, and `package.sh` copies the fifth one by one like the others — there
is still no `cp -R` and so no way in for a sixth. `tests/box.rs` is what has it measured.

## Considered options

- **Putting the family table in the tool description** — rejected. Not for 0012's 0 of 36,
  which measured triggering and does not transfer to a decision the agent has already taken;
  for the toll. The description is paid by every session that lists the tools, and the table
  is only ever needed by the ones that draw.
- **A third case in the recommended line** — rejected. 0017 closed that list at two and gave
  the reason: a clause that reopens it turns *exactly two cases* back into *whenever it seems
  useful*. And the line reaches nobody who already pasted it.
- **Naming the unmeasured families anyway, for completeness** — rejected. A skill that sends
  the agent at `journey` or `mindmap` buys a rejection, and a rejection sends it back to prose
  **silently**: the user sees fewer drawings, not better ones. What is measured is what is
  named.
- **Shipping the skill and leaving 0013 as it reads** — rejected. `no skills/` is written into
  the box's own packer as the reason there is no `cp -R`; contradicting it in the zip while
  leaving it standing in the ADR is how the next person restores the old behaviour and is
  right to.
