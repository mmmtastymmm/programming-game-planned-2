*Closed record — see [../README.md](../README.md). Not spec.*

# Q7 — What does a unit sense?

## Ruling

*Ruling (2026-09-07):* **two senses, vision and sound, each with a range per
unit kind, and everything any unit senses is shared with every unit of its
player.** Units come in kinds — `bot` is one, and buildings are several — and
each kind has a vision range and a hearing range, tuning constants in data.
Sensing is a **query a program polls**, computed at the moment it asks from
the world as it is; nothing is cached, so the state hash carries no sensing
data at all.

### The rules

Each is hash-affecting.

1. **Distance is squared Euclidean in `num`**, compared against a squared
   range. No square root exists, and a range modifier later multiplies a
   range before it is squared.
2. **Vision is state.** Unit `u` at `p` sees an entity at `q` if
   `dist²(p, q) ≤ vision(kind(u))²` **and** the line of sight from `p` to `q`
   is clear of blocking terrain. Which terrain blocks and the ray-walk that
   decides it are `docs/03`'s; the walk is one algorithm on every peer.
3. **Sound is a record of events.** An action that makes noise — moving,
   firing, building; `docs/02` lists the causes — emits a **sound** at the
   actor's position on the tick it happens, carrying its cause and a
   **loudness** from the cause's tuning row. Unit `u` hears a sound at `s` if
   `dist²(p, s) ≤ (hearing(kind(u)) + loudness)²`. **Sound is not blocked by
   terrain.**
4. **A hearing query returns the sounds of the previous tick**, a fixed
   window, so no unit keeps a "since I last listened" mark. A program that
   wants to hear everything listens every tick.
5. **All sensed data is shared.** A query from any of a player's units
   returns the union of what every unit of that player senses at that
   moment, each entity or sound once. A player's own units are always in the
   union, whatever the ranges.
6. **A sighting carries the sighted unit's attributes** — all of them, as
   `docs/02` defines them — plus its entity id and position. **A sound carries
   its cause, position, loudness and tick, and never its emitter**; if the
   emitter is also seen, that is a sighting.
7. **Order** (determinism rule 6): sightings by squared distance from the
   querying unit, ties by entity id; sounds by squared distance from the
   querying unit, then position, then cause. Every peer produces the same
   list.
8. **Cost** follows the game-builtin shape [`docs/01`](../../01-language/costs.md)
   fixed: a base per query plus `factor.traverse` per element returned, so
   a fleet that sees a lot pays to look.

### What each doc owns

The model above is spec now and moves to `docs/02` when it is written. `02`
names the two queries, lists the unit kinds with their ranges, the noise
causes with their loudness, and the attributes a sighting carries. `03` pins
what blocks sight and the ray-walk. Whether a sound can *also* interrupt a
program is Q16.

### Consequences

- **`docs/03` is unblocked**; `02` now waits on Q9 and Q16.
- **Sound is the first event-shaped thing in the design**, and it is ruled as
  a polled query. Q16, which decides whether world events push into handlers,
  starts from a sense that already exists in polled form.
- **Fleet communication is not a question**, because there is nothing to
  communicate: every unit already knows what any unit knows.
- **The fourth option, sensors as equipment, is not closed.** Ranges per kind
  in data are the hook it would use; per-equipment ranges are a widening
  under a new number.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, and
  the reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block; Q16's framing.
- **Task:** [T9](../../TASKS.md) — `03` is unblocked and Q7 leaves its list.
  [T7](../../TASKS.md) — no longer blocked on this question.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Omniscient within a radius | Simplest to specify and to sort. Removes scouting and information asymmetry as design material. |
| Line-of-sight, per unit | Makes terrain matter and exploration real. Visibility as per-unit hashed state would be a large addition to the state hash at fleet scale — avoided here by computing at query time. |
| **Shared fleet vision, with line of sight and a second sense** *(chosen)* | One visibility set per player rather than per unit, computed on demand. Terrain matters (vision), events carry (sound), and the fleet feels like one organism. Two senses to specify and two ranges per kind to tune. |
| Explicit sensors as equipment | Sensing becomes a build choice with costs and trade-offs. Most design surface, most tuning, most to get wrong. Left as a widening; the per-kind ranges are where it would attach. |

### How the answer took its shape

The register's four options were one axis, how much a unit knows. The
ruling split that into two axes and answered each: *what* a unit senses
(two senses with different physics, one blocked by terrain and one not) and
*who* knows it (everyone on the side). The first came from noticing that
"what is around me" and "what just happened" are different shapes of
information, and a programming game wants both. The second came from Q2's
fleet fantasy: fifty units running three programs are one organism, and an
organism does not keep secrets from itself.

Two smaller choices. **Squared distance in `num`** was preferred to an integer
metric so a range modifier can be a multiplier rather than a second table.
**Loudness on the sound** rather than on the hearer alone was preferred
because building kinds vary, and a model with one range per side needs
somewhere for "a shot carries farther than a footstep" to live.

**Computed at query time** is the choice that makes the rest cheap. Sensing
as stored state would put a visibility set per player into the hash and
require every world change to update it; sensing as a function of the world
costs a query what it returns and nothing else.
