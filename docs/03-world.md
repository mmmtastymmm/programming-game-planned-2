# 03 — The world

What the map is made of: tiles, the terrain on them and its height, how
bots change the ground, the deposits that regrow, what blocks sight and
how a line of sight is decided, and where a match begins. This doc is what the world half of `crates/sim` is built from,
and it is normative: two implementers reading it must produce the same
tick-by-tick behavior (design-invariant DI7). Machines and what they do on
the world are [02-machines](02-machines.md); this doc says what they stand
on. The water that stands on the ground is [09-water](09-water.md)'s.

## Decided

This doc elaborates two rulings that live in [02-machines](02-machines.md)'s
Decided section and are not repeated here: **Q21**, which made the world a
grid of tiles that the colony remembers, and **Q22**, which made deposits
terrain that regrows. Everything below is this doc's own, and none of it
overrides those.

- **Every tile has a height, and bots change the ground (Q65, amending
  Q7, Q10, Q21, Q22, Q27, Q28, Q37 and Q49).** A tile's `height` is a whole
  number of levels from the map. A bot climbs at most one level a step,
  slower up and faster down, and may drop any number, which is a fall
  that hurts unless it lands in water. A bot gathers **pace** as it keeps
  moving, and one with pace that runs into another bot **bonks** it a
  tile onward. A bot `dig`s a tile down a level, `fill`s
  one up a level and `mine`s rock away, which leaves ground at the rock's
  height; digging puts **earth**, a second resource, in the bot's load,
  and filling takes it. The player plans a height for a tile, a fifth
  kind of plan, and bots carry it out. Higher ground blocks a line of
  sight, and a machine sees one tile further for each level it stands
  above the tile it looks at and one less for each level below. `water`
  is no longer a terrain kind: it stands on ground (`09`). The rule that
  a tile's terrain never changes is withdrawn.
- **Paint, overlays and wires are planned only on ground the team has seen
  (Q49, as Q62 extends it to wires).** A `Mark` of a paint, an overlay or a
  wire plan, a value or a clear, applies only on a `ground` tile the
  sending team has seen — visible or remembered — and is dropped on its
  tick otherwise: never on rock, and never on a tile still unknown
  to the team, which is tested first so that marking cannot scout. A wire
  plan that asks for what the tile already has — to lay where the team's
  wire lies, or to clear where none does — is dropped on its tick too, and
  the team's `wire` or `unwire` that makes a tile match its plan consumes
  it (`02`), so a wire plan is always one a bot can carry out. A building
  plan may go anywhere, and an `Unmark` always applies.

## Tiles

The world is a **rectangle of tiles**, `width` by `height`, both integral
`num` fixed by the map. A tile is addressed by `(x, y)` with the **origin at
the centre**: `(0, 0)` is the tile `floor(width / 2)` columns from the west
edge and `floor(height / 2)` rows from the south edge; `x` increases
**east** and `y` increases **north**, as on a map, so a 24 by 16 map runs
from `(-12, -8)` to `(11, 7)` and `move("n")` (`02`) increases `y`. Nothing
exists off the rectangle: a position outside it is not a tile, a move onto
it is a `ValueError`, and a sighting can never report one.

**Row then column**, wherever a query sorts tiles, means north to south and
then west to east: `(x, y)` sorts before `(x', y')` if `y > y'`, or `y == y'`
and `x < x'`.

Every tile holds exactly:

| Field | Holds |
|---|---|
| `terrain` | one of the two terrain kinds below; `rock` becomes `ground` when mined, and nothing else changes it (Q65) |
| `height` | a whole number of levels from zero to `height.max` in data, from the map; changed a level at a time by `dig` and `fill` (Q65) |
| `water` | the parts of water standing on it, a whole number, a hundred to a level, zero on rock; changed only by the water step (Q65, `09`) |
| `spring`, `sink` | each the rate of the spring or the sink on the tile, or `None`; from the map and fixed (Q65, `09`) |
| `deposit` | a `dict` of resource kind to amount, or `None`; on any passable tile the map puts one on |
| `paint` | the tile's one paint, a color from `deployment.colors` in data or `None`; influences nothing |
| `overlay` | the tile's one overlay, a label from `mark.overlays` in data or `None`; influences bots through the programs that read it |
| `wires` | the set of teams with a wire on the tile, empty on anything but `ground`; each team's wire is underground, occupies nothing, and joins that team's grid (Q62, `08`) |
| `plans` | **per team**, five plans — paint, overlay, building, wire (Q62), ground (Q65) — each `None` or a value (a color, a label, a model, `True` for a wire, a height for the ground; or a plan to clear, which the ground plan has none of); placed and withdrawn by the player through `Mark` and `Unmark` (Q10, Q27) — paint, overlay and wire only on ground the team has seen (Q49, Q62) — read by that team's programs only, realised only by a bot's action, which consumes the plan; never sensed or remembered |
| the machine on it | at most one (`02`, Occupancy); not a field of the tile, but of the machine |

**Paint and overlay are the tile's, not a team's** (Q28): every team's bots
see them in a sighting and remember them in a snapshot (Q21), and a program
on any team reads them. **A wire is its team's** (Q62), and each team has
its own on a tile, so one team's wire never blocks another's; every team
sees and remembers every wire, as it does paint. **Plans are a team's**,
and private. All of it is world state, in the hash.

A tile's `terrain`, `height`, `paint`, `overlay`, `wires` and
building change only by a bot's action — a team's wire only by its own
bots' `wire` and `unwire` (`02`), never by another team's, or cleared with
its team when the team is out (`06`, step 5) — and **a slot is emptied
before it is filled**: a bot `paint`s only a tile whose paint is `None`,
`overlay`s only one whose overlay is `None`, and `build`s only one with no
building; `unpaint`, `unoverlay` and `deconstruct` (`02`) empty them, and a
plan with a value does the emptying first. A tile is never destroyed, so
the second cause of `death` that `02` reserves for this doc **does not
occur**: a machine dies of damage or not at all. Deposits change amount and
nothing else. A tile's `water` changes only in the water step (`09`).

## Terrain

The terrain kinds are a closed list, one row each in
[data/world.toml](../data/world.toml). Each says whether a machine may stand
on it and whether it blocks sight.

| Terrain | Passable | Blocks sight |
|---|---|---|
| `ground` | yes | no |
| `rock` | no | yes |

- **Passable** terrain is what a bot may move onto (`02`, `move`), where
  the step itself is allowed (A step, below). A
  building occupies its tile and is not terrain; a bot cannot move onto a
  building's tile because the tile is occupied, not because of its terrain.
- **Buildable** is not a terrain property: a tile is buildable if its terrain
  is `ground`, it has no deposit, no spring and no sink, it is not wet
  (`09`), and it has no building. A deposit is therefore never buried.
- **Blocks sight** is what the ray-walk below tests, with height. A machine
  blocks nothing: bots and buildings are seen through, and so is water.

Adding a terrain kind is a new question number and a row here. `water` was
a third kind until Q65 made it something that stands on ground.

## Height

Every tile, rock included, has a **height**: a whole number of levels from
zero to `height.max` in data. The map gives it, and only `dig` and `fill`
change it. Height does three things: it limits a step, it steers water
(`09`), and it decides what is seen (Line of sight, below).

**A step.** A bot may **step** from the tile it is on to an adjacent tile
only if that tile is `ground`, is higher or lower than its own by at most
`height.step` in data, one level, and holds water of at most `water.wade`,
one level (`09`). Wherever a rule asks whether a tile is passable *for a
bot on a given tile* — `move_to`, where a printed bot appears (`02`), a
map's check that a start reaches a deposit — it asks this.

**A drop.** `move(dir)` (`02`), and it alone, may also take a bot onto an
adjacent `ground` tile that is *lower* by more than `height.step`, with
water of at most `water.wade`: a drop, which ends in a fall (below). No
bot climbs more than `height.step`.

A bot that the ground or the water has changed under keeps its tile.
Nothing moves a bot but its own move and a bonk.

## Moving

How long a move takes, what a bot gains by keeping going, and what happens
when it leaves a cliff or meets another bot (Q65). A bot is on both tiles
of a move, for occupancy, from when the move begins until it completes
(`02`).

**Pace.** A bot has a `pace` (`02`), a whole number from zero to
`pace_max` in its model's data, zero when it is printed. A move's ticks
are fixed as it begins: the model's `move_ticks`, less the bot's pace,
plus `up_ticks` if the tile it moves onto is higher, less `down_ticks` if
it is lower, and never fewer than one. When a move completes, the bot's
pace rises by one, by two if the move went down a level, and not at all
if it went up, to at most `pace_max`. The direction does not matter: a
bot keeps its pace through a turn.

At the end of every tick's slices (`06`, step 2), a bot that is not in a
move has its pace set to zero. So a bot keeps its pace only by beginning
its next move in the slice after the last completed; a wait, any other
action, a failed move, or being out of power loses it.

**A fall.** A move onto a tile lower by more than `height.step` is a
drop. It takes the ticks of a move down. When it completes the bot has
**fallen** as many levels as the two heights differ by: its pace is zero,
and in this tick's damage step (`06`, step 4) it loses `fall_damage` in
data for each level fallen beyond `height.step` — unless the tile it
landed on holds at least `water.cushion` parts of water (`09`), which
takes all of it.

**A bonk.** `move(dir)` by a bot whose pace is above zero, onto a tile
that holds another bot — any team's — that is not itself in a move, does
not fail as a move onto an occupied tile otherwise does. It begins as a
bonk: it takes `bonk_ticks`, during which the bot is on its own tile
alone, and makes the sound `"bonk"`. When it completes, the bot's pace is
zero, and if the other bot is still on that tile and not in a move, and
the tile beyond it in the same direction is one it could step or drop
onto and no machine is on it or moving onto it, the other bot is put on
that tile at once. Its action in progress ends with nothing done, its
pace is zero, and if the tile was a drop it has fallen. Otherwise nothing
happens to it. A bonk does no damage; the fall may. A building and a site
are never bonked, and a bot with no pace bonks nothing: for both the move
is the `ValueError` it always was.

## Changing the ground

Three bot actions change a tile (`02` has their rows). Each acts on an
adjacent tile, takes its ticks, and takes effect when it completes, if the
tile then still meets what it met when the action began; otherwise
nothing happens. Any team's bot may do any of them on any tile.

A tile is **clear** if it holds no building, no site, no deposit, no
spring and no sink, and no machine is on it or moving onto it.

- **`dig(x, y)`** needs a clear `ground` tile whose height is above zero,
  and room in the bot's load for `height.earth_per_level` of `earth`. On
  completion the tile's height falls by one and the earth joins the load.
- **`fill(x, y)`** needs a clear `ground` tile whose height is below
  `height.max`, and `height.earth_per_level` of `earth` in the bot's load. On
  completion the tile's height rises by one and the earth leaves the
  load.
- **`mine(x, y)`** needs a `rock` tile. On completion its terrain is
  `ground`, at the height it had; nothing is gained.

A tile under water is dug and filled like any other: its water stays on
it, and flows by `09`'s rule from the surface it then has. Paint, an overlay and wires stay on a tile whose ground changes.

**Earth** is a resource kind (`02`, Q22), the second: a row of the
resource table with its capacities, carried, dropped into a depot and
picked from one like `ore`. No deposit holds it; digging is its only
source and filling its only use.

**The ground plan** (Q10, Q27) is a height the player wants a tile brought
to. A `Mark` of one applies only on a tile the sending team has seen that
holds no deposit, no spring and no sink, and is dropped on its tick if the tile is
already `ground` at that height. On rock it asks for the rock to be mined
and the ground beneath brought to that height. The plan is consumed when
a `dig`, `fill` or `mine` by one of the team's own bots completes and
leaves the tile `ground` at the planned height. It has no plan to clear:
`Unmark` withdraws it.

## Deposits

A deposit is the `deposit` field of a tile, on any passable tile the map
puts one on: for each resource kind in the resource table (`02`, Q22), an
**amount**, a **cap** and a **regrowth** per tick, the last two per tile from
the map and bounded by the kind's maxima in data.

- **Regrowth.** At the tick-end pass, after every machine's slice and before
  the vision pass (`06` places both), every deposit's amount becomes
  `min(amount + regrowth, cap)`. A deposit at zero regrows like any other,
  so no deposit is ever exhausted for good.
- **Picking** (`02`, `pick`) takes from the amount at the moment the action
  completes, `min(pick_rate, amount, free capacity)`, and never below zero.
- **What a program is told.** A visible tile's record carries its deposit's
  current amounts; a remembered one carries the amounts as of the tick it
  was last seen (Q21), which regrowth has since made stale in the colony's
  favour — the belief is a floor, not a ceiling.
- **Order.** Deposits, where a query lists them, sort as tiles do: by row
  then column.

## Line of sight

Q7 made vision depend on line of sight and left the algorithm here. It is
one algorithm on every peer, integer-only, over tile centres:

**A line of sight from tile `a` to tile `b` is clear** if no tile the ray
passes through, other than `a` and `b` themselves, has terrain that blocks
sight or stands above the line between the two (Q65, below). The ray is walked with **Bresenham's line algorithm** in its integer
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
terrain and height, since a ray of length one passes through nothing.
`rock` on `b` itself does not block: a machine sees a rock tile it has a
clear line to, which is how walls are seen at all.

**Height on the line (Q65).** Let the walk from `a` to `b` take `n` steps
of rule 2, so that the tile reached at step `k`, for `k` from 1 to
`n − 1`, is a tile the ray passes through. That tile stands above the
line, and blocks it, if

`height(tile) × n > height(a) × (n − k) + height(b) × k`

— if its ground is above the straight line from the ground of `a` to the
ground of `b`, in integers. A tile exactly on the line does not block, so
level ground sees across level ground as before. Water adds nothing to a
height here.

**Height and range (Q65).** A machine on tile `a` with vision `v` sees
tile `b` if the line is clear and the squared distance between them is at
most `r × r`, where `r = v + height(a) − height(b)`, and nothing if `r` is
below zero. Every rule that tests sight from a machine to a tile uses
this `r` — the vision pass, a sighting, the range of what a team sees —
and `attack` (`02`) keeps its own range and only gains the blocking rule.
Hearing is not sight and height does not change it.

## The map

A **map** is a data file, one per match, that fixes everything this doc
calls "from the map": `width` and `height`, every tile's terrain and its
height, the water it starts with and its springs (`09`), every deposit's
cap and regrowth, and the starting positions. The format is
[data/maps/](../data/maps/)'s, one TOML file per map, and the first map is
`data/maps/first.toml`. A map is part of the replay's inputs: `(map,
command log)` reproduces a match, and the map's bytes are hashed into the
replay's identity, length-prefixed as rule 7 hashes a bundle.

A map fixes:

- **the starting speed** (Q6), one of the steps in `data/world.toml`, the same
  for every team; and, for each team,
- **one or more starting tiles**, on each of which a printer stands at
  tick 0 (Q9, Q42), with full health and an empty store, on the `printer`
  deployment — every other printer the team will have, its bots build
  (`02`); and
- nothing else — no bots, no ore held, no memory. A starting printer's
  battery is full (`08`). The printers begin the
  first prints in tick 1's power and print step (`02`, Printing), whatever
  the opening program set (Q9) holds; the bots run the `bot` bundle once it
  arrives.

**Heights, water and springs (Q65).** A map gives every tile's height as
a grid of digits beside its terrain; a map that gives none has every tile
at `height.default` in data and every `~` tile `height.lake_depth` below
that. `~` in the terrain is ground that starts under water, and `09` says
how deep. A spring and a sink are each a tile and a rate, and the map
gives the ground water it begins with.

A map is **valid** only if every starting tile is buildable, no two are the
same tile, every team has at least one, every starting tile can reach
at least one deposit by steps a bot may take as the match begins, every
spring and sink is on a clear `ground` tile of its own that is no
starting tile, with a rate above zero and at most `water.max_rate`, and
its `~` tiles are as `09` requires. The sim refuses an invalid map at load, as it refuses
an invalid bundle.

**Symmetry** is a map author's business and not a rule: a fair map places
teams and deposits so that no team's nearest deposit is nearer than
another's, and the first map does.

## The tick

`docs/06` owns the tick loop. This doc needs from it only the order of the
passes at a tick's end: after every machine's slice, first **regrowth**,
then the **water step** (`09`), then the **vision pass** (Q21). That way a
deposit's remembered amount is the amount after this tick's growth, and a
tile's remembered water is the water after this tick's flow, which is
what a machine that saw it this tick would have seen.

## Costs

Nothing in this doc is a builtin: the world is read through `02`'s queries
and changed through `02`'s actions, and their rows price it. The tick-end
passes cost no program anything.

## What this doc leaves open

- **Map generation.** The first maps are hand-written. Generating one from
  a seed is a `docs/06` question when it is wanted: the seed is the sim's
  (rule 4), and generation is a deterministic function of it.
