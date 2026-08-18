*Closed record — see [../README.md](../README.md). Not spec.*

# Q1 — Relationship to the predecessor project

## Ruling

*Ruling (2026-08-17):* a **fresh take on the same core idea**. Lockstep
multiplayer, players program their units, deterministic sim in plain Rust. What
is redone is everything around that: the corpus discipline, the scope, and the
command surface.

*Reasoning.* The predecessor's determinism architecture was sound — it is
already ported here and proven green in CI on a machine that is not the author's.
What went wrong there was not architectural. It was corpus discipline (registers
that appended forever until one reached 166 KB, counts maintained by hand that
drifted four times in a day, a split convention that had silently inverted in
seven of 62 part files) and an ever-widening command surface. Keeping the
architecture and re-deciding the game is the split that preserves what worked
without inheriting what did not.

*Consequences.* The determinism rules in CLAUDE.md stand unchanged and are not
reopened by this design pass. `crates/sim`'s placeholder world model gets
replaced rather than extended.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section; the
  architecture-fixed-before-the-design list. [CLAUDE.md](../../../CLAUDE.md) —
  the determinism rules are inherited unchanged and not reopened.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Fresh take, same core idea** *(chosen)* | Inherits the predecessor's architecture and its habits both. Guards against the habits explicitly or they return. |
| Same genre, different shape | Keeps programmable units and lockstep but changes scale, setting and match structure. More redesign for less carried-over certainty. |
| Deliberately different game | Only determinism carries over. Maximum freedom, and the ported determinism kit becomes the only thing the port bought. |
| Undecided — explore first | Defers the framing decision that every other question hangs off. Cheap now, expensive at the first contradiction. |

The port is already done and green, which asymmetrically favours the first two:
the architecture is not a bet any more, it is a fact.
