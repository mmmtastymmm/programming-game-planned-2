*Closed record — see [../README.md](../README.md). Not spec.*

# T45 — Water in `sim`: parts that flow, springs, sinks and the ground water, wading and flooding

All of `09`, as Q66 amended it: a fifth to each lower tile, the flow in
`water.passes` passes, a film that dries only on a tick when nothing
reaches it, a spring's head and a sink's sill, a thousand parts to a
level, and every amount in a map, the tables and a tile's record in
levels. `water` leaves the terrain kinds; a tile holds its parts.
The water step in step 7, in `09`'s order: springs, the flow worked out
from one state and applied together, sinks, drying, and `flooded`. The
ground water, in the hash where `06` puts it. A map's `~` tiles full to
their rims, its springs and sinks, the ground water it begins with and
the checks on all of them; `[water]` in data with the checks `09` gives.
Still water is skipped, since it gives nothing, and the result is the
one a walk of every tile gives: a test holds the two together over
random ground, random digging and filling, and springs and sinks. Wading
in a step; a flooded bot's moves refused, a flooded building dark from
the water step and through the power step, and flood damage in step 4.
A tile's record, memory and the snapshot carry `water`, `spring` and
`sink`. Tests: a tile's offers for one, two and four lower neighbours,
and when they outrun its water; a small rise on a flat pool that does
not slosh; a channel dug from a lake filling a tile a tick; a rim cut
open and a lake draining; a spill on a plain drying to nothing and one
in a hollow staying; a spring and a sink making a river; a lake that a
spring brings back; the total of the tiles and the ground water never
changing; a tile filled in under water; and a flooded printer that
prints nothing and dies. The golden replay gains a lake, a spring, a
sink, a channel and a flooded building. `crates/replay/benches/scale.rs`
gains a map with a lake and a river and says what the water step costs.

Completed in `bee91dd`, the merge of #141, in four parts. The flow, the
springs and sinks, the ground water, drying and wading came first, in
`9410e48`, the merge of #136, with water out of the terrain kinds and
the test that holds the skipping of still water to a walk of every
tile. Q66 then changed the rules the first part had built — a fifth to
each lower tile, the flow in passes, a film that dries only unfed, a
head and a sill, a thousand parts to a level and every amount written
in levels — and the code followed it in `dbaf768`, the merge of #138.
Flooding was `60d56f1`, the merge of #140: `flooded` set at the end of
the water step, a flooded bot's `move` and `move_to` refused, a flooded
building dark from that step and skipped by the power step, its
battery neither giving nor filled, and `water.flood_damage` in step 4,
at 0.2 a tick as a first guess; eight tests in
`crates/sim/tests/flood.rs`, among them the flooded printer that prints
nothing, does not defend and takes its team out. A flooded generator
still counts in its grid's `generation` figure, since `08` defines the
figure as a sum over the grid's buildings. The same merge gave
`crates/replay/benches/scale.rs` its `--water`, a river loop and a lake
at rest, with the water step timed alone on a copy of the world: 0.02
to 0.12 ms a tick on maps from 28 by 28 to 98 by 98.

The last part, `a3f6347`, was the fixture. The golden replay runs on
the first map with a spring and a sink in its south-west lake, added by
the test itself until T47 gives the map its own; its ninth bot digs a
hollow beside the lake, builds a pylon in it and cuts the channel that
floods it, and the pylon dies of the flood fifty ticks on. The alive
test asserts each, and fails with the channel cut too late, the spring
and sink switched off, or the pylon planned before the hollow. Every
hash moved in each of the four parts, and each PR said why.

One key `09` gives `[water]` is not in data: `cushion`, the water that
breaks a fall, which has nothing to read it until falls are built and
goes in with them, under T43. The flooded mark on the map is T46's, and
a starter bot that digs its way out of a flood is T47's.
