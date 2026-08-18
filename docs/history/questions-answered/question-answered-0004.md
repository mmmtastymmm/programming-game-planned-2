*Closed record — see [../README.md](../README.md). Not spec.*

# Q4 — The opposition

## Ruling

*Ruling (2026-08-17):* **PvE first, PvP later.**

*Reasoning.* PvE lets tuning be forgiving while the sim is young; PvP demands
fairness from the first ranked match. The ordering costs nothing architecturally,
because lockstep determinism is being built now regardless — it is not
retrofittable, which is the whole reason CLAUDE.md's rules are non-negotiable
rather than aspirational.

*Consequences.* The golden-replay gate earns its keep from day one even with no
PvP, because the same determinism is what makes PvE reproducible and its bugs
reportable. Balance questions may be deferred; determinism questions may not.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **PvE first, PvP later** *(chosen)* | Balance work is deferred, not avoided; the PvE tuning that ships first may not survive contact with a ladder. |
| PvE only | Balance becomes a design problem rather than an arms race, but the lockstep architecture is then largely unexercised by its motivating use case. |
| PvP only | Lockstep pays off immediately, but everything must be fair from day one, with no forgiving period while the sim is young. |
| Co-op / sandbox | Competition optional or absent; least pressure on fairness, and least pull toward the determinism discipline already committed to. |

Deferring PvP is free architecturally because lockstep is not retrofittable and
is therefore being built now either way. What it buys is slack on balance during
the period when the sim is changing fastest.
