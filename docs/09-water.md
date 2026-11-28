# 09 — Water

Water that stands on the ground and runs over it: how much a tile holds,
how it flows to lower ground a tile at a time, what puts it on the map and
takes it off, what it does to a machine it covers, and how a team's
buildings pipe it. This doc is what
the water half of `crates/sim` is built from, and it is normative: two
implementers reading it must produce the same tick-by-tick behavior
(design-invariant DI7). The ground water stands on — every tile's height,
and the digging, filling and mining that change it — is
[03-world](03-world.md)'s.

## Decided

- **Water is small parts that flow over ground that has a height (Q65,
  amending Q21, Q28 and Q62, as Q66 amended).** `water` is no longer a kind
  of terrain: a tile is `ground` or `rock` at a height (`03`), and water
  is what stands on it, a whole number of **parts**, a thousand to a
  level as Q66 has it. Every tick each tile gives water to the lower
  tiles beside it, a fifth of the difference each as Q66 has it, so a
  flood has a front and a lake takes time to drain. Nothing else moves it: a channel dug from a lake fills because
  the lake runs into it. **Springs** put water on the map and **sinks**
  take it off, and both draw on one store of **ground water**, as does the
  thin water that dries, so that the water of a match is a fixed amount
  that only moves. A bot wades through a level of water and is
  **flooded** by more; a building is flooded by any water deep enough to
  count; a flooded machine is stopped and loses health every tick. Pumps,
  pipes, storage and what uses water are Q67's, below.
- **A tile gives a fifth, the flow runs in passes, a film dries only
  when nothing reaches it, a spring has a head and a sink a sill, and
  water is written in levels (Q66, amending Q65).** Q65 had a tile give
  each lower neighbour the difference over one more than the number of
  them, which in a channel is half, and half makes a row of tiles swing
  back and forth for ever; a tile now gives **a fifth**, however many
  are lower. The flow runs `water.passes` times a tick, so that a gentle
  slope carries a real stream. Thin water dries only on a tick in which
  it was given none, so that a stream can wet a floor and a channel
  fill. A spring may have a **head** it will not raise its own tile's
  water above, and a sink a **sill** it will not take its own tile's
  water below, so that a spring and a sink make a river that fills to
  its banks and runs over none of them. Water is counted a thousand
  parts to a level, and every amount a map, the tables or a program
  holds is written in levels, so that how finely water is counted is a
  number that changes nothing else.
- **Water is piped: buildings add to a network and take from it, pumps
  lift water into it, tanks hold it and spouts pour it back (Q67,
  amending Q10, Q27, Q37, Q42 and Q65).** Every building model has a
  need for water a tick in data, which may be nothing; a team's
  buildings that touch, or are joined by its **pipe** underground or its
  **aqueducts** overhead, are a **network**, served oldest first; and a
  building its network cannot cover is **dry**, and stops as a dark one
  does. A **pump** lifts water off a wet tile beside it while it is
  powered, a **tank** holds it, and a **spout** pours it onto the tile
  it faces, in a direction its program sets. Piped water is the map's
  own: what a pump lifts leaves a tile, what a spout pours lands on
  one, and what a building uses goes to the ground water, so the water
  of a match is still a fixed amount. A printer has a **well** that
  covers its own need, so a team that lays no pipe still prints. Pipes
  take no account of height, and bots carry no water.

## Water on a tile

Water is counted in **parts**, and `water.parts` of them in data, a
thousand, stand as deep as one level of height (`03`). Every amount of
water in the sim is a whole number of parts, and so is every tile's
`water`: the parts standing on it, zero or more.

Parts are how the sim counts and nothing more. A map, the tables and a
program each give an amount of water **in levels** — a whole number, or a
decimal — which must come to a whole number of parts; `film` alone
(below) is in parts, since it is the rounding it clears up. So
`water.parts` may be changed and no map, and no program, means anything
different.

- Only `ground` holds water. `rock` holds none at any height and is a
  wall to it, as the edge of the map is: water never leaves the map and
  never enters a rock.
- An **open** tile is a `ground` tile. The **surface** of an open tile,
  in parts, is its `height` times `water.parts`, plus its `water`: where
  a dry tile's ground is, and where a wet tile's water stands to.
- A tile is **wet** if its `water` is at least `water.wet` in data, a
  tenth of a level, and **damp** if it holds less than that and more than
  nothing. Damp ground is ground: only a wet tile counts as water for a
  step, for building and for flooding (below, and `03`).

Water does not block sight, and nothing about a tile but its `water`
changes when water comes or goes: its terrain, height, paint, overlay,
wires, deposit and plans are what they were.

## The water step

The water step runs once a tick, after regrowth and the network step
(The water network, below) and before the vision pass (`06`, step 7). It does five things in order, and nothing else in
the sim moves water.

**1. Springs run.** Each spring, in the order of its tile, moves onto its
tile as many parts as its rate, or all the ground water holds if that is
less, or, if it has a head, only what brings its tile's surface up to
the head, if that is less still (Springs and sinks, below).

**2. Water flows**, `water.passes` times over. In each pass every flow is
worked out from the water as the pass began, and all of them are then
applied together, so the order tiles are looked at in changes nothing;
the next pass begins from what that left.

For each open tile that holds water, its **lower neighbours** are the
adjacent open tiles (`02`) whose surface is below its own. It **offers**
each of them a fifth of the difference between their two surfaces,
rounded down: the tile and its four sides share, however many of the
sides are lower. If its offers together are more than the water it
holds, they are met lowest surface first — among equals in the order
`n`, `e`, `s`, `w` — each in full while its water lasts, and the one it
runs out on in part.

Then every tile's `water` becomes what it held, less what it gave, plus
what it was given.

**3. Sinks run.** Each sink, in the order of its tile, moves from its
tile into the ground water as many parts as its rate, or all the tile
holds if that is less, or, if it has a sill, only what stands above the
sill, if that is less still.

**4. Thin water dries.** Every tile that holds water, and no more than
`water.film` parts of it, four, and that was given none in this step —
by a spring or by a neighbour, in any pass — loses it all to the ground
water. A film that is still being fed stays, and builds. A spring's own
tile is not dried.

**5. `flooded` is set** on every machine (Wading and flooding, below).

**What the rules add up to.**

- **Water runs downhill, a tile a pass.** A tile only ever gives to the
  tiles beside it, so the front of a flood moves `water.passes` tiles a
  tick, and a lake cut open drains over many ticks.
- **Flow needs a slope.** What crosses between two tiles in a pass is a
  fifth of how far one stands above the other, so a stream of a given
  size needs a given slope, whatever the water's depth, and a long level
  channel carries little. More passes carry more on the same slope.
- **Level water stays put.** A tile gives a neighbour nothing until it
  stands five parts above it. A lake's surface can therefore hold a
  slope of up to four parts a tile — four thousandths of a level — and
  no more.
- **Nothing sloshes.** A fifth is small enough that no arrangement of
  tiles swings back and forth: a rise on a pool, or a row of tiles in a
  channel, only settles.
- **Water stays only where the ground holds it.** On open flat ground a
  spill spreads until its edge is a film and nothing more reaches it,
  the film dries, and the edge moves in: a puddle on a plain is gone in
  time. In a hollow it stays.
- **The total is fixed.** The parts on every tile and the ground water
  together never change in these five steps: springs, sinks and drying
  only move parts between the two.

An implementation must leave the state these steps leave, however it
finds it. Still water gives nothing, so a settled lake need cost nothing.

## Springs and sinks

The **ground water** is one whole number of parts for the whole map,
under every tile and belonging to none. It starts at the amount the map
gives, which may be zero, and it is world state, in the hash.

A **spring** is a feature of a tile the map places (`03`, The map), fixed
for the match, with a **rate**, in levels a tick. A **sink** is the same,
with a rate of its own. A spring brings ground water up onto its tile
and a sink takes water on its tile down, each at its rate and each only
as far as there is water to move (The water step, 1 and 3).

A spring may have a **head**: a height, in levels, above its own tile's.
It brings up nothing that would raise the surface of its tile past its
head. Water it feeds never stands higher than it does at the spring, so
where the ground beside that water is everywhere at or above the head,
none of it runs over. A sink may have a **sill**: a height at or above
its own tile's. It takes nothing that stands at or below its sill, so
the water beside it fills to the sill before the sink takes any.

- **A spring with a head and a sink with a sill under it make a river.**
  It fills until its surface stands between the two and then holds, the
  same tick after tick, with what the slope between them carries running
  from the one to the other: a loop that makes and loses nothing.
- A map with no sink loses no water but what dries, which its springs
  return. Its lakes come back to what they were.
- A map with a spring above a sink has a river between them for as long
  as the ground holds the water there.
- A spring does nothing while the ground water is empty, and so a match
  that begins with none sees its springs run only as water dries, is
  sunk, or — once something uses water (What this doc leaves open) — is
  given back.

A spring's or a sink's tile cannot be dug, filled or built on (`03`).

## Wading and flooding

- **Wading.** A bot may step onto a tile whose `water` is at most
  `water.wade` in data, one level (`03`, A step). Deeper water is not
  passable.
- **A flooded bot** is one standing on a tile whose `water` is above
  `water.wade`. A bot cannot walk into that; it happens when the water
  rises on it. **A flooded building**, or site, is one whose tile is
  wet.

Every machine has the attribute `flooded` (`02`), which the water step
sets, from the water as its step 4 left it, and which stands until the
next one.

- **A flooded bot cannot move**: its `move` and `move_to` are a
  `ValueError`. Every other builtin runs as before, so it can dig, fill
  or mine its way out, and a move it had begun completes.
- **A flooded building is dark** (`08`): its `powered` is `False` from the
  water step that floods it, and the power step leaves it dark whatever
  its grid holds — its generation is not pooled, its need is not met, and
  nothing enters or leaves its battery. It stays on its grid and still
  joins the buildings beside it. This is the one way a printer is dark:
  it begins no print, a print in progress holds, and it does not defend.
- **A flooded machine loses health**: in step 4 of every tick (`06`),
  each machine flooded as the last water step left it loses
  `water.flood_damage` in data, a bot, a building or a site alike, with
  that step's other damage, and `dying` and `death` follow as for any
  damage (`02`).

A machine is flooded or not by the water on its own tile and nothing
else: a bot wading beside deep water is unharmed.

## The water network

Water is taken off the map, held, used and put back by a team's
buildings, joined into **networks** (Q67). It is the same water: every
amount here is a whole number of parts, written in levels wherever a
table or a program holds it, and the parts on every tile, the ground
water and what every tank holds together never change.

| Model | Tank | Needs | Does |
|---|---|---|---|
| `printer` | yes, small | per tick, and more while printing; its well covers both | — |
| `pump` | no | nothing | lifts water off a wet tile beside it, while powered |
| `tank` | yes, large | nothing | holds water |
| `spout` | no | nothing | pours water onto the tile it faces |
| `aqueduct` | no | nothing | joins its team's network across its span; runs no program |
| every other building | no | nothing | joins what it touches |

Every number is a row of the model in
[data/machines.toml](../data/machines.toml), an amount of water or of
water a tick, in levels, and zero where the table says nothing: `tank`,
the most it holds; `water_drain`, and for a printer `print_water`, its
need; `well`; `pump_rate`; `pour_rate`. The table is today's data, not a
rule: the rules read the numbers, never the model's name. A machine
holds water in its **tank**: its `water` attribute, never above its
model's `tank`, which is its `tank` attribute. Every tank starts empty,
a map's printer's included. A bot has none of these and carries no
water.

**A need and a well.** A building's **need** in a tick is its model's
`water_drain`, plus `print_water` for a printer whose `printing` is
above zero, less its model's `well`, and nothing if that is below zero.
A well is water the building is taken to bring up and give back within
the tick, so it moves no parts and asks nothing of the ground water.
Data starts the printer's well at its `water_drain` plus its
`print_water`, so a team that lays no pipe prints as it always did;
bringing the well down is what makes pumps matter, and no check on data
holds it up.

### Networks

A team's **water conductors** are the tiles carrying its pipe (`03`) and
the tiles of its buildings, sites excluded. Two are **joined** when they
are adjacent, and an aqueduct's tile is joined besides to the tile of
every aqueduct and building of its team, sites excluded, within the
aqueduct's `span`, by squared distance, whatever stands between. A
**network** is a set of a team's water conductors joined to one another
that holds at least one building; its **label** is the least entity id
among its buildings, which is each of those buildings' `network`
attribute. This is `08`'s grid in every respect but what joins: wire and
a pylon's span join a grid and no network, pipe and an aqueduct's span a
network and no grid, and a building's own tile joins both. Pipes and
aqueducts take no account of height.

### The network step

In step 7 of the tick (`06`), after regrowth and before the water step,
for each team in order:

1. **The networks are traced**, as above.
2. **Each pump finds its source**, in ascending entity id: of the wet
   tiles beside it, the one holding the most water, the first of `n`,
   `e`, `s`, `w` among equals. A pump that is not powered (`08`), is
   flooded, or has no wet tile beside it has none, and lifts nothing
   this tick. A pump with a source may lift up to its `pump_rate` in the
   tick, and never more than its source holds at the moment it lifts.
3. **Each network is served**, networks in ascending label, three times
   over. Wherever water is **drawn**, it is lifted by the network's
   pumps in ascending entity id, each as far as step 2 allows, and what
   they cannot give is taken from its tanks in ascending entity id.
   1. *The needs.* Each building in ascending entity id — oldest first:
      one with no need is **watered**. Each other: if what the pumps may
      still lift and the tanks hold covers its need, the need is drawn,
      those parts go to the ground water, and it is watered; if not, it
      takes nothing and is **dry**.
   2. *The spouts.* Each spout that is pouring (below), in ascending
      entity id, draws up to its `pour_rate`, or as much as there is,
      and those parts land on the tile it faces.
   3. *What is left.* The tanks are filled in ascending entity id, each
      to its `tank`, by what the pumps may still lift. A pump lifts
      nothing that nothing takes.

   A **flooded** building takes no part: it is dry without being tried,
   and its tank neither gives nor is filled. It is still on its network.
4. **The figures are kept**: for each network its label and four sums
   over its buildings — `supply`, the `pump_rate` of every pump that had
   a source; `drain`, every need; `stored`, the water its tanks hold
   after the step; and `capacity`, the tanks' — as team state, in the
   hash, which `water()` returns.

A tile a pump lifts from or a spout pours onto is disturbed, and the
water step that follows in the same tick moves its water on. `watered`
and `network` are set here and hold until the next network step; the
match is built with its networks traced, every building watered and the
figures kept as they then stand. A building completed in step 3 is
watered and on no network until this step. A site is on no network and
is not watered, and neither matters.

### A dry building

A building the last network step left dry is stopped as a dark one is
(`08`, A dark building): its rules, its program and its senses stop, and
everything else stands — it keeps its tile, its health, its store, its
charge and its water, its tank still gives and is filled, it still joins
its network, and a dry printer still counts for its team's cap and for
the team not being out. A dry printer begins no print, and its print in
progress holds. Being dry and being dark are apart: the power step
serves a dry building as it does any other.

### Pumps, tanks and spouts

Each is a building a bot builds from a site (`02`), with a deployment of
its own and a program.

- **A pump** stands on dry ground like any building, and lifts only
  while a tile beside it is wet. What it drains is power, by its `drain`
  (`08`). Water that rises onto its own tile floods it, and it stops.
- **A tank** only holds. A building with a tank of its own, the printer
  today, is a tank to its network as well.
- **A spout** has the attribute `pouring`: `"n"`, `"e"`, `"s"`, `"w"` or
  `None`, which its program sets with `pour(dir)` and which is `None`
  when it is built. It pours onto the tile beside it in that direction
  if that tile is `ground` on the map, at whatever height, and nowhere
  if it is not. A dark or a dry spout pours nothing.

A building that leaves the world — deconstructed, dead, or its team
out — gives what its tank holds to the ground water.

### Pipes and aqueducts

**A pipe** is underground, and is `08`'s wire in every rule but what it
carries: a mark of the team's on a `ground` tile (`03`), planned with
the sixth plan kind, laid and removed by the team's own bots alone with
`pipe(x, y)` and `unpipe(x, y)`, each tile priced in ore at `pipe_cost`
and taking `pipe_ticks` or `unpipe_ticks`; it occupies nothing, shares
its tile with anything, a wire included, joins only by adjacency, and
every team sees and remembers it. **An aqueduct** is a building, as a
pylon is: built from a site, with health, no deployment and no program,
which any team's bot may attack or deconstruct, and that is how a rival
cuts a network. A team that is out has its pipes cleared with its
machines.

## The map's water

A map (`03`, The map) gives the water a match begins with by marking
tiles `~`: ground that starts under water. Each set of `~` tiles joined
by adjacency starts full to its rim: the surface of every tile in it is
the height of the lowest open tile beside the set that is not `~`, or,
where rock and the map's edge are all that is beside it, one level above
the set's own highest tile. A map in which a `~` tile's height is not
below that is refused. A map also gives its springs and sinks — each a
tile and a rate, a spring's head and a sink's sill if they have one —
and the ground water it begins with, every amount in levels. The
match's first water step is tick 1's.

## What a program reads

A tile's record (`02`, `tile`) carries its `height`; its `water`, how
deep it stands, a `num` in levels; and its `spring` and its `sink`, each
the rate in levels a tick or `None` — as the
team sees them, or as it last saw them on a remembered tile (Q21) — and
`pipes`, the sorted team numbers with a pipe on it (Q67). A machine's
record carries `flooded`, and its `water` and `tank`, in levels; a
building's, `watered` and `network`; a spout's, `pouring`. `water()`
returns the team's networks as the last network step kept them, as
`power()` (`02`) does its grids, each a record of `network`, `supply`,
`drain`, `stored` and `capacity`; `pipes()`, the tiles carrying the
team's pipe, as `wires()` does. `pour(dir)` is a spout's alone and takes
no ticks. The ground water is read by no program. `pipe`, `unpipe`,
`pour`, `water` and `pipes` each have a row in the `[game]` cost table
and in `[power.cost]` (`02`, `08`).

## Data

`[water]` in [data/world.toml](../data/world.toml) holds `parts`, how
finely water is counted; `wet`, `wade` and `max_rate`, the most a map
may give a spring or a sink, each in levels; `film`, in parts; `passes`;
and `flood_damage` and `cushion`, the water that breaks a fall (`03`,
Moving). Every key is hash-affecting. The sim refuses data in which
`parts` or `passes` is nothing, an amount in levels does not come to
whole parts, `film` is not below `wet`, or `wet` is above `wade`.

## What this doc leaves open

- **Pipes that mind height.** A network lifts water as high as it
  likes for nothing but its pump's power.
- **A spout's direction given with its plan**, where today only its
  program sets it; and a spout's rate set by its program.
- **Which buildings need water.** Only the printer's need is above zero
  today, and its well covers it: every number of the network is a
  placeholder until the game is played.
- **A bot carrying water**, and a rival cutting a pipe.
- **Rain.** Springs are the only way water reaches the ground.
- **Water with momentum.** Flow here is in proportion to slope, so a
  level river carries next to nothing; a flow that builds up along a
  channel, and carries a river on a gentle slope, is a new question
  number.
  difference each tick; a cap on what crosses between two tiles in a
  tick is a number that could be added.
