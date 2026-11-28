*Closed record — see [../README.md](../README.md). Not spec.*

# T48 — The water network in `sim`

`09`'s The water network. The models `pump`, `tank`, `spout` and
`aqueduct`, with their rows in `data/machines.toml` and deployments for
the first three; `tank`, `water_drain`, `print_water`, `well`,
`pump_rate` and `pour_rate` on a model, in levels, and a tank on the
printer. The pipe: a sixth plan kind in `Mark` and `Unmark` and on the
wire, what drops one on its tick and what consumes it, `pipe` and
`unpipe` with their cost and ticks, and a tile's `pipes`. Networks
traced as grids are, by pipe, adjacency and an aqueduct's span. The
network step in step 7 before the water step: each pump's source, the
needs, the spouts and the tanks, in `09`'s order, with every part
lifted, poured or used accounted for, and the figures kept. `watered`
and a dry building stopped as a dark one is, a print held and none
begun; `pouring` and `pour`; `water()` and `pipes()`; a tank's water
given to the ground water when its building leaves the world. Every
new attribute in the record, the snapshot and the hash. Tests: a
network joined each way and cut by a deconstructed aqueduct; a need
met by a pump, by a tank, and by neither; oldest served first when
short; a pump with no wet tile, one unpowered and one flooded; two
pumps on one tile; a spout pouring each way, onto rock and off the
map; a pond pumped dry and a hollow filled from it uphill; the tiles,
the ground water and the tanks never changing in sum; a dry printer
that prints nothing; and a team that lays no pipe playing as before.
The golden replay gains a pump, a pipe, a tank and a spout that fills
a hollow.

Completed in `9702578`, the merge of #151; the task's own commit is
`b85d470`. The network is `crates/sim/src/network.rs`, and it is traced
by the code that traces grids, made general over what joins:
`trace_joined`, given wire and the pylon or pipe and the aqueduct. A
dry building is `Machine::is_dry`, and `is_stopped`, dark or dry, is
read everywhere dark was: the slice, a held action, a print begun or
counting down, a printer's defence, recharging from the building, its
sight and its hearing. Water a model holds or moves is kept in whole
parts, so the machines' table is parsed against `water.parts`.
`World::remove` and `remove_team` give a tank's water to the ground
water, and `water_total` counts tanks. Twenty-six tests in
`crates/sim/tests/network.rs`; of twenty-three defects seeded by hand
into the rules and the hash, twenty were caught, the rest being a dry
check that is made twice and two in a hash test since rewritten.

Readings the build made, which `09` now states: a dry pump still
lifts; a spout's dry is the same step's; two pumps on one tile count
its water once. Others it left unstated: a pour is held to what a tile
can count; with no well a printer still begins the one print the
match's first tick allows, the match being built watered, and holds
it; and a pond pumped dry is one whose tile is no longer wet, a tenth
of a level staying as damp.

The numbers are placeholders: a printer's tank 2 levels, its drain
0.002 a tick and 0.008 more while printing, its well their sum; a pump
lifting 0.05 a tick and draining 1 power; a tank of 20; a spout
pouring 0.05; pipe at wire's cost and ticks; an aqueduct's span 4.

`crates/render` was given stand-in looks for the four models, for T49
to replace. `crates/lang`'s check that every cost row is named by the
docs reads `09` as well as `02`, where Q67's rows are until T51 moves
them.

The golden replay gained a pump, a pipe, a tank and a spout that fills
a hollow, each asserted, and to fit them the scenario was changed: it
runs 540 ticks where it ran 480, its `bot` redeploy moved from tick 440
to 500, a second redeploy at tick 386 brings the program that builds
the waterworks, and the fixture map's spring runs at 0.1 where it ran
0.05, so that the lake keeps its slope with a pump on it. No other
assertion of the alive test changed. Every hash moved.
