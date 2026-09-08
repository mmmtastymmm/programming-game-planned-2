*Closed record — see [../README.md](../README.md). Not spec.*

# Q28 — Amend Q26 and Q27: realised marks belong to the tile, and a slot is emptied before it is refilled

## Ruling

*Ruling (2026-09-08):* **a tile has one paint, one overlay and one building,
as physical things any team can see and any bot can change; plans stay per
team and private; and every slot must be emptied by one action before it is
filled by another.** This amends [Q26](question-answered-0026.md) and
[Q27](question-answered-0027.md), which made realised paint and overlay
per-team and invisible to other teams, and adds the deconstruct rule. Made
under a new number because both files are closed records; Q27's plans,
actions and reads otherwise stand.

Two changes to `docs/03` ride with it and are recorded here because they
touch what earlier rulings said: deposits are a field of a tile rather than
a terrain kind, and the map's origin is its centre with north and east
positive.

### The rules

Each is hash-affecting.

1. **A tile's realised marks are the tile's.** One `paint` (a color or
   `None`), one `overlay` (a label or `None`), and at most one building. They
   are not per team: every team's bots see them in a sighting and remember
   them in a snapshot (Q21), and a program reads them through the tile record
   whatever team it is on. `painted(color)` and `overlaid(label)` return
   every tile so marked that the team can currently see or remembers.
2. **Plans are per team and private**, as Q27 ruled: three slots per team
   per tile, placed by `Mark`, withdrawn by `Unmark`, read through
   `plans(kind)` and the tile record's `plans`, never sensed or remembered.
3. **A slot is emptied before it is filled.** A bot may `paint` only a tile
   whose paint is `None`, `overlay` only a tile whose overlay is `None`, and
   `build` only a tile with no building. Three actions empty them:
   `unpaint(x, y)`, `unoverlay(x, y)` and `deconstruct(x, y)`, on an adjacent
   tile, taking `unpaint_ticks`, `unoverlay_ticks` and the model's
   `deconstruct_ticks`. Any bot of any team may empty any slot.
4. **A plan with a value does the emptying first.** A bot realising a paint
   plan on a painted tile performs the unpaint and then the paint, as one
   action of both durations; likewise an overlay plan on an overlaid tile,
   and a building plan on a built tile, which deconstructs and then places
   the site. A plan whose value is `None` is the emptying alone.
5. **Deconstruction is destruction.** A deconstructed building returns
   nothing: its store is lost, and if it was a printer its team's cap and
   deployments update as they would on its death (Q24). A site is
   deconstructed like a building and its store is lost too. A machine
   standing on a building's tile cannot exist, so no occupancy question
   arises.
6. **Deposits are a field, not a terrain.** A tile of any passable terrain
   may carry a deposit; "buildable" is ground with no deposit and no
   building. The terrain kinds are `ground`, `rock` and `water`.
7. **The origin is the centre.** `(0, 0)` is the tile at `floor(width / 2)`
   columns from the west edge and `floor(height / 2)` rows from the south
   edge; `x` increases east and `y` increases north, as on a map, so
   `move("n")` increases `y`. Row-then-column order is north to south, then
   west to east: `(x, y)` sorts before `(x', y')` if `y > y'`, or `y == y'`
   and `x < x'`.

### Consequences

- **Marks are contestable.** A rival can unpaint a tile or deconstruct a
  depot, at the cost of walking there and spending the ticks; that is the
  first way one team can hurt another, and it costs no damage and needs no
  Q25. Defending a mark is defending a tile.
- **The tile record is the same for every team** except its `plans` and its
  memory `state`, which simplifies `02`'s queries.
- **Three actions join `02`**, each with a duration and a cost row, and the
  mermaid tree grows.
- **`03`'s terrain table loses a row** and its map gains a signed
  coordinate system; the first map's starting tiles are restated in it.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — the Q10 bullet, held
  until `docs/06` exists. [02-machines.md](../../02-machines.md) — the
  actions, the reads, the tree, adjacency. [03-world.md](../../03-world.md)
  — tiles, terrain, deposits, the map. [QUESTIONS.md](../../QUESTIONS.md) —
  the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Realised marks per team, as Q26 and Q27 ruled | Two teams' paint on one tile, neither seeing the other's. A mark is a private note, not a thing in the world, which sits oddly with a bot having walked there to make it. |
| **Realised marks are the tile's; plans are per team** *(chosen)* | One paint, one overlay, one building per tile, seen by everyone. A rival can undo them, at the price of a trip and the emptying action. |
| Overwrite in place — a new paint replaces the old in one action | Simpler by three actions. Loses the cost of contest: repainting a rival's tile would be as cheap as painting an empty one. |

### How the answer took its shape

Q27 made every mark a bot's work, and the user drew the conclusion Q26 had
missed: a thing a bot made is in the world, so it is the tile's and not the
team's. The deconstruct rule follows from that — if a tile has one paint,
replacing it means removing the old one, and removing is work — and the
plan doing both steps at once is what keeps the player's side of it a
single request. The deposit and origin changes were `03` corrections the
user made in the same pass.
