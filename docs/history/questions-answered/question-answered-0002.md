*Closed record — see [../README.md](../README.md). Not spec.*

# Q2 — What the player programs

## Ruling

*Ruling (2026-08-17):* a **fleet of identical units**, driven by a small number
of player-written programs. Many units, few programs.

*Reasoning.* The fantasy is emergence from copies: fifty units running three
programs produce behavior the author did not write line by line, and the scale
knob is fleet size rather than puzzle intricacy. The alternatives each move the
difficulty somewhere this design does not want it — into a single program's
sophistication, into coordination between hand-written specialists, or into
routing a system whose agents are dumb.

*Consequences.* Programs are addressed to a **role or group, not to an
individual unit**, so the language needs no unit-identity concept in its core.
Determinism rule 6 (sorted queries, ties broken by entity id) becomes
load-bearing rather than theoretical: fifty units query the same world state in
the same tick, and any unstable ordering is a desync. And the failure mode — one
bad program is fifty dead units — makes fault behavior a headline design problem
rather than a footnote, which is why it is opened as its own question rather than
settled inside the language question.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | Scale knob | Failure mode | What it costs |
|---|---|---|---|
| **A fleet of identical units** *(chosen)* | Fleet size | One bad program, fifty dead units | Requires stable sorted queries under heavy concurrent sensing; makes fault behavior a headline problem |
| One unit, deeply | Puzzle complexity | You, out-thought | Difficulty must come from hand-authored puzzles — a content treadmill rather than an emergent system |
| A squad of specialists | Coordination depth | Deadlock between your own units | Needs inter-program communication in the language core from day one |
| A factory / production system | Throughput | A bottleneck you cannot see | Agents become dumb; the "programming a unit" fantasy largely evaporates |

The deciding argument was where difficulty is *sourced*. A fleet sources it from
interaction between copies, which the sim generates for free once determinism
holds. A single unit sources it from authored content, which someone has to keep
writing.
