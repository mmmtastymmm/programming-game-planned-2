# 03 — The world

What the map is made of: tiles, the terrain on them, the deposits that
regrow, what blocks sight and how a line of sight is decided, and where a
match begins. This doc is what the world half of `crates/sim` is built from,
and it is normative: two implementers reading it must produce the same
tick-by-tick behavior (design-invariant DI7). Machines and what they do on
the world are [02-machines](02-machines.md); this doc says what they stand
on.

## Decided

This doc elaborates two rulings that live in [02-machines](02-machines.md)'s
Decided section and are not repeated here: **Q21**, which made the world a
grid of tiles that the colony remembers, and **Q22**, which made deposits
terrain that regrows. Everything below is this doc's own, and none of it
overrides those.

## Tiles

The world is a **rectangle of tiles**, `width` by `height`, both integral
`num` fixed by the map. A tile is addressed by `(x, y)`, with `x` from `0`
to `width - 1` running east and `y` from `0` to `height - 1` running south;
`(0, 0)` is the north-west corner. Nothing exists off the rectangle: a
position outside it is not a tile, a move onto it is a `ValueError`, and a
sighting can never report one.

Every tile holds exactly:

| Field | Holds |
|---|---|
| `terrain` | one of the terrain kinds below, fixed for the match |
| `deposit` | a `dict` of resource kind to amount, or `None`; only on `ore` terrain |
| `marks` | per team: a blueprint (a building model) or `None`, a paint (a deployment color) or `None`, and a set of layer labels — placed by the player through the command log (Q10), read by that team's programs, never sensed or remembered, and part of the state hash |
| the machine on it | at most one (`02`, Occupancy); not a field of the tile, but of the machine |

A tile's `terrain` never changes; its `marks` change only by command. The
layer labels are the closed list `mark.layers` in `data/world.toml`. A tile is never destroyed, so the second
cause of `death` that `02` reserves for this doc **does not occur**: a
machine dies of damage or not at all. Deposits change amount and nothing
else.

## Terrain

The terrain kinds are a closed list, one row each in
[data/world.toml](../data/world.toml). Each says whether a machine may stand
on it, whether a building may be built on it, and whether it blocks sight.

| Terrain | Passable | Buildable | Blocks sight | Holds |
|---|---|---|---|---|
| `ground` | yes | yes | no | nothing |
| `ore` | yes | no | no | a deposit |
| `rock` | no | no | yes | nothing |
| `water` | no | no | no | nothing |

- **Passable** terrain is what a bot may move onto (`02`, `move`). A
  building occupies its tile and is not terrain; a bot cannot move onto a
  building's tile because the tile is occupied, not because of its terrain.
- **Buildable** terrain is what `build` and `build_nearest` accept. Building
  on a deposit is refused so a deposit is never buried.
- **Blocks sight** is what the ray-walk below tests. A machine blocks
  nothing: bots and buildings are seen through.

Adding a terrain kind is a new question number and a row here; the four
above are what the first maps need.

## Deposits

A deposit is the `deposit` field of an `ore` tile: for each resource kind in
the resource table (`02`, Q22), an **amount**, a **cap** and a **regrowth**
per tick, the last two per tile from the map and bounded by the kind's
maxima in data.

- **Regrowth.** At the tick-end pass, after every machine's slice and before
  the vision pass (`06` places both), every deposit's amount becomes
  `min(amount + regrowth, cap)`. A deposit at zero regrows like any other,
  so no deposit is ever exhausted for good.
- **Picking** (`02`, `pick`) takes from the amount at the moment the action
  completes, `min(pick_rate, amount, free capacity)`, and never below zero.
- **What a program is told.** A visible `ore` tile's record carries its
  deposit's current amounts; a remembered one carries the amounts as of the
  tick it was last seen (Q21), which regrowth has since made stale in the
  colony's favour — the belief is a floor, not a ceiling.
- **Order.** Deposits, where a query lists them, sort as tiles do: by row
  then column.

## Line of sight

Q7 made vision depend on line of sight and left the algorithm here. It is
one algorithm on every peer, integer-only, over tile centres:

**A line of sight from tile `a` to tile `b` is clear** if no tile the ray
passes through, other than `a` and `b` themselves, has terrain that blocks
sight. The ray is walked with **Bresenham's line algorithm** in its integer
form, from `a` to `b`, with these fixings so that it is single-valued:

1. Let `dx = |bx - ax|`, `dy = |by - ay|`, `sx = +1` if `bx > ax` else `-1`,
   `sy = +1` if `by > ay` else `-1`, and `err = dx - dy`.
2. From `(x, y) = a`, repeat: if `(x, y) == b` stop; let `e2 = 2 × err`; if
   `e2 > -dy` then `err -= dy` and `x += sx`; if `e2 < dx` then `err += dx`
   and `y += sy`; the tile now at `(x, y)`, if it is not `b`, is a tile the
   ray passes through.
3. The walk is **not** symmetric in general — the ray from `a` to `b` may
   pass through different tiles than the ray from `b` to `a` — so sight is
   always evaluated **from the seer to the seen**, never the reverse.

A machine sees the tile it stands on and every adjacent tile regardless of
terrain, since a ray of length one passes through nothing. `rock` on `b`
itself does not block: a machine sees a rock tile it has a clear line to,
which is how walls are seen at all.

## The map

A **map** is a data file, one per match, that fixes everything this doc
calls "from the map": `width` and `height`, every tile's terrain, every
deposit's cap and regrowth, and the starting positions. The format is
[data/maps/](../data/maps/)'s, one TOML file per map, and the first map is
`data/maps/first.toml`. A map is part of the replay's inputs: `(map,
command log)` reproduces a match, and the map's bytes are hashed into the
replay's identity as the bundle's are (rule 7).

A map fixes:

- **the starting speed** (Q6), one of the steps in `data/world.toml`, the same
  for every team; and, for each team,
- **one starting tile**, on which the team's starting printer stands at
  tick 0 (Q9), with full health and an empty store, on the `printer`
  deployment; and
- nothing else — no bots, no ore held, no memory. The first slice belongs
  to the printer, which prints the first bot once the opening program set
  (Q9) has reached it.

A map is **valid** only if every starting tile is `ground`, no two teams
share one, and every team's starting tile can reach at least one `ore` tile
by passable terrain. The sim refuses an invalid map at load, as it refuses
an invalid bundle.

**Symmetry** is a map author's business and not a rule: a fair map places
teams and deposits so that no team's nearest deposit is nearer than
another's, and the first map does.

## The tick

`docs/06` owns the tick loop. This doc needs from it only the order of the
two passes it defines: after every machine's slice, first **regrowth**, then
the **vision pass** (Q21). That way a deposit's remembered amount is the
amount after this tick's growth, which is what a machine that saw it this
tick would have seen.

## Costs

Nothing in this doc is a builtin: the world is read through `02`'s queries
and changed through `02`'s actions, and their rows price it. The tick-end
passes cost no program anything.

## What this doc leaves open

- **Q25** decides whether anything in the world deals damage — terrain that
  hurts, an opposition that fires — and, if so, gains a column in the
  terrain table.
- **Map generation.** The first maps are hand-written. Generating one from
  a seed is a `docs/06` question when it is wanted: the seed is the sim's
  (rule 4), and generation is a deterministic function of it.
