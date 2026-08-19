*Closed record — see [../README.md](../README.md). Not spec.*

# Q3 — The player's role during a match

## Ruling

*Ruling (2026-08-17):* the player **writes and updates programs while the match
runs**. A program update is a lockstep-synchronized command. There is no *other*
live input: no direct orders to an individual unit, no clicking a unit and
telling it where to go.

*Reasoning.* Observing a program fail and patching it mid-match is the core loop
of this genre — the game is programming, not program submission. The alternative
considered seriously was pure programming: submit a set, watch it play out. That
buys real simplifications, and they compound (a match determined entirely by
`(seed, program set)`, a replay of a few kilobytes, one code path for single- and
multiplayer, PvP fairness independent of reaction speed). It was rejected anyway,
because it removes the observe–diagnose–patch cycle that is the fantasy. A
player who cannot intervene is watching a submission, not playing.

The restriction that *survives* is the one that matters: updates are to
**programs**, never to individual units. The player never issues an order. That
keeps Q2's "programs address a role or group, never a unit" intact and keeps the
skill being tested squarely on the code.

*Consequences.*

- `Command` is a real ordered per-tick log, not a single opening value. Its
  principal variant is a program deploy.
- A program update must take effect on the **same tick on every peer**, which is
  the standard lockstep scheduling problem: an update is agreed for a future tick
  so every peer holds it before that tick executes. How far ahead, and what
  happens when a peer is late, is Q12.
- Determinism rule 7 becomes load-bearing rather than housekeeping. Programs are
  byte-exact text, versions are identified by hashing source bytes, and which
  version a unit is running is part of the state hash.
- What happens to a unit **mid-execution** when its program changes — restart,
  resume at the same point, keep or discard local state — is hash-affecting and
  unresolved. That is Q11, and it cannot be deferred past the language question.
- A replay is `(seed, timed command log)`. Still small, because deploys are rare
  next to ticks, but no longer a single value.
- PvP fairness re-enters: with unrestricted updates, reaction speed matters
  again. Deferred alongside PvP itself (Q4) and noted in Q12.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, and the
  flow diagram, which now routes player edits and a peer's edits through the same
  ordered command log. [CLAUDE.md](../../../CLAUDE.md) — the opening description.
- **Task:** [T7](../../TASKS.md) — the placeholder `Command` variants command
  individual units, which this ruling forbids. Recorded there as wrong to copy
  from rather than as a register entry, because the placeholder never claimed to
  be the model.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Programming, with mid-match program updates** *(chosen)* | Every update is a lockstep command, so program identity becomes hashed per-tick state and hot-swap semantics become a hash-affecting design problem (Q11). Reaction speed re-enters PvP fairness. |
| Pure programming, no live input | Rejected. Buys a match determined by `(seed, program set)`, a few-kilobyte replay, one code path, and fairness independent of reaction speed — but removes the observe–diagnose–patch loop that is the genre. |
| Live orders to individual units as well | Rejected outright. Breaks Q2's "programs address a role or group, never a unit" and moves the skill being tested from code to clicking. |
| Between rounds only | Keeps programming central without twitch play, but multiplies match-structure decisions (round length, carry-over, scoring) before anything else is settled. |

The axis that actually decided it is **what the player is allowed to touch**, not
*whether* they may act during a match. Updating a program is authorship;
commanding a unit is play. This design permits the first and forbids the second,
which is a sharper line than "live input yes/no" and the reason the pure-
programming option lost without its simplifications being in doubt.

Those simplifications are worth naming, since they are what was paid: the replay
artifact grows from a single value to a timed command log, program versions must
be byte-exact and hashed, and mid-execution swap semantics become load-bearing.
All three are tractable; none is free.
