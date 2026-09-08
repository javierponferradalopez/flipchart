# Drawing is the exception, not the default

**Status:** accepted · **Date:** 2026-09-08

**Refines** [0012](./0012-the-trigger-lives-outside-the-binary.md), which stands: the
trigger still lives outside the binary, and the line is still the last step of the
installation. What changes is what the line asks for.

## The line that works too well

The wording 0012 recommended —*when you explain a structure or a change of structure to me,
draw it*— fires on almost every technical answer. "A structure or a change of structure" is
what an agent explaining a codebase is doing nearly all the time, so the flipchart went up
for two boxes and an arrow, for a sequence of steps, for anything a sentence already
carried.

The measurement in 0012 counted attempts —**8 in 5 turns**— and read that as the channel
working. The instrument was blind to the thing that turned out to matter: **drawing on
every explanation and drawing when it helps are the same number in a count of attempts.**
In use it was the first.

## Two cases, and prose everywhere else

The recommended line now names them and closes the list:

> Explain in prose. Draw on the flipchart with `mcp__plugin_flipchart_flipchart__show`
> in exactly two cases: when you catch yourself starting an ASCII diagram, and when
> I ask you to draw something.

**The ASCII impulse is the trigger, because ASCII was the symptom.** 0012 measured what the
agent does when nobody tells it to draw: it paints the graph in ASCII inside its answer.
That impulse is the agent's own signal that prose is not carrying the thing — the moment
this product was built for, and the agent is better placed than the user to notice it.

**The user asking is the other channel, and it was already the strongest one:** 9 attempts
in 7 turns in the same measurement.

## The price

**Not re-measured.** 0012's figures describe the wide line, not this one. Nothing here
replaces them.

**The ASCII impulse may never fire.** 0012 has the counter-example written down already:
with a `CLAUDE.md` that forbade ASCII, the agent did not reach for the flipchart — it fell
back to **prose with lists**, and the flipchart went unused. A trigger that waits for an
impulse the agent can route around is weaker than one that names a topic. If that happens
here, the honest description of the product becomes *it draws when you ask*, with the first
case contributing little.

**Accepted knowingly:** a flipchart that draws too much is worse than one that draws
rarely. The window is a native window that comes up over the user's work, and drawing for
two boxes spends their attention on something a sentence carried. **Of the two ways to be
wrong, this decision prefers the quiet one.**

## Consequences

The README ships the new line as the install step. Anyone who pasted the 0012 wording keeps
the old behaviour until they change it, and nothing warns them — the same consequence 0012
already wrote down about a line that is not ours to write.

## Considered options

- **Keeping the wide line and leaving the rest to the agent's judgement** — rejected.
  *Draw when it helps* gives the agent nothing checkable, and 0012 measured what happens
  with nothing checkable: 0 of 36 turns.
- **Moving the narrowing into the tool description** — rejected, again. Four wordings, 0 of
  36 turns; the description is not where triggers live.
- **A third case for genuinely complex shapes** —cycles, layering, fan-out— rejected for
  now. It is a judgement call wearing the clothes of a rule, and it is the clause that
  turns *exactly two cases* back into *whenever it seems useful*. It can come back with a
  measurement behind it.
