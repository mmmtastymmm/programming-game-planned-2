*Closed record — see [../README.md](../README.md). Not spec.*

# Q21 — Fog of war: does the colony remember what it no longer senses, and how?

## Ruling

*Ruling (2026-09-07):* **the colony remembers tiles, not units.** The world
is a grid of tiles, and for each player every tile is in one of three
states: **unknown**, never seen by any of the player's units; **visible**,
seen by one of them now; or **remembered**, holding the tile as it was on
the tick it was last seen. A remembered tile holds the terrain and any
building on it — never a bot, which is only ever a live sighting — and the
tick. Memory has no expiry: the table is bounded by the map, and a remembered
tile stays remembered until it is seen again. Sound is never remembered.

This is the first sensing data in the state hash, which amends the
consequence [Q7](question-answered-0007.md) drew from computing sensing at
query time: the live list is still never stored, and the memory table is.

### The rules

Each is hash-affecting.

1. **The world is tiles.** Every position is a tile, terrain is per tile,
   and a unit occupies a tile. `docs/03` owns what a tile is; this ruling
   assumes only that tiles exist and have positions.
2. **Visibility is Q7's vision, evaluated against tiles.** A tile is visible
   to a player if any of the player's units is within its vision range of the
   tile's position and has line of sight to it.
3. **A tick-end vision pass.** After every unit's slice and before the tick's
   state hash — `docs/06` owns the tick loop and places this step in it — the
   sim computes each player's visible tile set and refreshes the remembered
   snapshot of every visible tile: the terrain, the building on it with its
   attributes as `docs/02` defines them, and the tick. A tile that leaves
   vision therefore keeps the snapshot of its last visible tick, without any
   "was visible" bookkeeping.
4. **Bots are never remembered.** A snapshot holds no bot. A bot is a live
   sighting (Q7) and nothing else; out of sight, it is gone from every list.
5. **Memory belongs to the player**, not to any unit. It survives every
   unit's death and every redeploy, and nothing clears it until the match
   ends.
6. **What a program is told.** A tile query returns the tile's state and,
   for a visible tile, the tile as the world is at the moment of the query
   (Q7's rule, so a bot that moved this tick is where it is now); for a
   remembered tile, its snapshot and `seen_at`; for an unknown tile, nothing
   — not even its terrain. `docs/02` names the queries and their fields, and
   `docs/03` says what terrain a snapshot carries.
7. **A remembered building that has since been destroyed is still
   remembered**, until the tile is seen again. That is the fog: what the
   colony believes, not what is true.
8. **Order** (determinism rule 6): tiles by position, row then column.
   Sightings keep Q7's order.
9. **Cost**: a query over tiles pays the game-builtin shape, a base plus
   `factor.traverse` per tile returned. The tick-end pass costs no program
   anything; it is the sim's.
10. **The memory table is world state.** Per player, per tile: the state and
    the snapshot. It is hashed with everything else, and a replay reproduces
    it.

### What each doc owns

The model is spec now and moves to `docs/02` with Q7's. `02` names the tile
queries and the building attributes a snapshot carries; `03` says what a tile
and its terrain are and what a snapshot of terrain holds; `06` places the
tick-end pass in the tick loop and the memory table in the state hash. How
fog is drawn is the renderer's, once Q15 picks one.

### Consequences

- **`docs/02` waits on Q9 alone**, and `docs/03` has its first ruling to
  build from: the world is tiles.
- **Communication stays unasked.** Everything a unit sees is shared live
  (Q7), everything the colony has seen of the map is shared as memory, and
  nothing a unit could tell another is left over.
- **Q7's "no sensing data in the hash" is narrowed to the live list.** The
  memory table is the cost the register foresaw for fog of war, taken on
  deliberately, and bounded by the map rather than by an expiry rule.
- **Ghost trails are avoided by design.** Remembering tiles rather than
  units means a scout seen crossing ten tiles leaves no scout behind on
  any of them; remembering buildings means a base seen once is a target
  until it is seen gone.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, the Q7
  bullet's hash claim narrowed, and the reserved-docs table.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T9](../../TASKS.md) — Q21 leaves its list. [T7](../../TASKS.md)
  — its note on what `02` waits on.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| No memory — programs keep what they can in variables | Nothing to specify. Every restart forgets everything, and no unit can tell another anything; scouting is worth exactly one look. |
| Colony memory of sightings — last known position of everything ever seen, with an expiry | The classic fog of war for *units*. A per-player table of units, an expiry so it does not grow forever, and ghost trails unless each unit is remembered once. |
| **Colony memory of tiles — terrain and buildings as last seen, no expiry** *(chosen)* | Bounded by the map, so no expiry rule; refreshed by one pass per tick; bots never remembered, so no ghosts. The first sensing data in the state hash, sized map times players. |
| A shared, writable scratch space — programs write what they want remembered | Maximum expressiveness and the smallest sim, and every player builds their own fog of war, badly at first; a shared store is a race between fifty copies of one program unless writes are ordered by rule. |

### How the answer took its shape

The question was opened as memory-of-sightings, and the first draft of an
answer was a per-unit table with an expiry. Two things moved it to tiles.
The first was the expiry: a unit table grows with everything ever seen and
needs a rule for forgetting, while a tile table is the map and needs none.
The second was ghosts: "the last seen version" of a tile, taken literally,
remembers the bot that stood on it, and a bot seen crossing ten tiles would
be remembered on all ten. Remembering only what does not move — terrain and
buildings — is what real fog of war does, and it is what makes a remembered
tile *mean* something: the colony's belief about a place, wrong until it
looks again.

**The tick-end pass** is the cost. Q7 avoided storing sensing data by
computing it on demand, and memory cannot be computed on demand — the sim
has to notice a tile stop being seen. Refreshing every visible tile's
snapshot every tick is the simplest form of noticing: it needs no memory of
what was visible last tick, and it makes "last seen version" exactly the
snapshot left behind when the refreshing stops.
