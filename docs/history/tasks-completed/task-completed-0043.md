*Closed record — see [../README.md](../README.md). Not spec.*

# T43 — Height in `sim`: a height on every tile, steps, pace, falls and bonks, `dig`, `fill` and `mine`, earth, the ground plan

`03`'s Height and Changing the ground, `02`'s rows and `06`'s steps. A
`height` on every tile, read from a map's grid of digits or from
`height.default`; `[height]` in `data/world.toml` with `max`, `default`,
`lake_depth`, `step` and `earth_per_level`. A step: ground, heights at most `step` apart,
for `move_to`, where a printed bot appears, and a map's check that a
start reaches a deposit; a drop, for `move` alone. `03`'s Moving: a
bot's `pace`, a move's ticks fixed as it begins from `move_ticks`,
`pace_max`, `up_ticks` and `down_ticks`, pace gained on completing and
lost as the slices end; a fall and its `fall_damage` in step 4; and a
bonk, with `bonk_ticks`, its sound, the bot it puts a tile onward and
that bot's action ended. Water does not break a fall until T45. The actions `dig`, `fill` and `mine`
with their ticks, cycle costs, charge costs and sounds in data; a tile
that is clear; `earth` in the resource table with a bot's and a depot's
capacity for it. The ground plan: a fifth plan kind in `Mark` and
`Unmark` and on the wire, what drops one on its tick, and what consumes
it. A tile's record, a team's memory of it, the snapshot and the state
hash carry its height. `water` stays a terrain kind until T45, so this
task alone leaves a match playable. Tests for each rule — a bot that keeps moving and one that pauses, a
turn, a climb and a descent, a fall of two levels and of four, a bonk
onto level ground, off a cliff and against a wall, a bonk from a
standstill refused, and a bonked bot's action ended — and the golden
replay gains a dug, a filled and a mined tile, a ground plan carried
out, a fall and a bonk, and the PR says why every hash moved.

Completed in `24803d1`, the merge of #147, in three parts. A height on
every tile and the step came first, in `a29936b`, the merge of #135:
`Height` and `Slope` in the map, a map's grid of digits, `World::steps`
for `move_to`, a printed bot's tile and a start's reach, and the height
in the record, the memory, the snapshot and the hash. `dig`, `fill` and
`mine`, earth and the ground plan were `f9648c0`, the merge of #139,
with `pick` taking only from a deposit that holds the kind asked for.

The last part, `521d5f3`, was `03`'s Moving, in
`crates/sim/src/moving.rs`: pace, with a move's ticks fixed as it
begins and `move_to` going by it as `move` does, which `02`'s row had
against `03` and now agrees with; a fall for `move` alone, its damage
summed in step 4 and broken by water as deep as `water.cushion`; and
a bonk. Twenty-two tests in `crates/sim/tests/moving.rs`. The numbers
the user kept: `pace_max` 2, `up_ticks` 1, `down_ticks` 1,
`bonk_ticks` 2, `fall_damage` 2, `water.cushion` 0.5, and `move_ticks`
left at 2 — so pace only ever speeds a bot up, to a tile a tick on
level ground, where Q65's worksheet had a standstill move slower than
before; and a bot at that pace uses 0.9 charge a tick, so one printer
no longer keeps ten that never rest charged. Readings the build made:
a bot in a bonk is not in a move and loses its pace as that tick's
slices end; a bonk still needs the tile to be a step or a drop from
the bonker's; a caught `ValueError` followed by a move in the same
slice keeps pace; a shove takes no account of `flooded`.

The golden replay gained a dug and a filled tile, a ground plan
carried out and a mined rock, a two-level fall and a bonk, each
asserted, across the three parts and [T45](task-completed-0045.md)'s
fixture; every hash moved in each, and each PR said why. Water did
break a fall from this task's own merge, T45 having landed first.
