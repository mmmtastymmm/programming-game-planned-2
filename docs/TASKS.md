# Tasks

What is left to build. **This file holds open tasks only** — completing one moves
it to [history/tasks-completed/](history/tasks-completed/README.md) as its own
file, with the commit that finished it. Anything here is open by definition, so
there are no checkboxes to keep in sync.

Numbering is stable — **append new tasks, never renumber**, and never reuse a
number after it moves to history. Entries are written as `**T<n> — <title>**` on
their own line, which is how `scripts/check-registers.mjs` finds them; it rejects
a number that is open and completed at once, and any gap *below* the highest
number recorded. Deleting the highest-numbered entry outright shrinks the range
and goes unnoticed — the same honest limit
[history/questions-answered/](history/questions-answered/README.md) states.

Milestones below are groupings, not entries. A milestone is finished when every
task under it has left the file.

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md). **No other register carries it** — CLAUDE.md and
  `.claude/design-invariants.md` define it, and `docs/history/` may narrate it,
  but no entry outside this file is marked. An open question is not work, and
  nearly every open question would qualify while the sim is unbuilt, so a marker
  on all of them selects nothing. Where a *ruling* cannot be discovered during
  implementation, the question says so in words.
- **Decided-but-unbuilt** work — a ruling the code has not caught up to — is
  tracked here *and* as an entry in [PROBLEMS.md](PROBLEMS.md). The register owns
  the gap; this file owns the work. A task is only lag once it is actually
  buildable; before that it is merely pending.

## M13 — Height and water

Q65, in the order to build it: each task stands on the one before, and
[P11](PROBLEMS.md) closes with the last.

**T43 — Height in `sim`: a height on every tile, steps, pace, falls and bonks, `dig`, `fill` and `mine`, earth, the ground plan** `⚠HASH`

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

**T44 — Sight over height in `sim`** `⚠HASH`

`03`'s Line of sight, as Q65 extends it. A tile on the ray blocks when
it stands above the straight line between the seer's ground and the
seen tile's, by the integer test `03` gives; a machine's range to a
tile is its vision plus its height less the tile's, nothing below
zero; `attack` gains the blocking and keeps its range; hearing is
unchanged. A change of height, or a mined rock, is a change to what a
team sees, so it bumps the count the sight cache holds (T38), and the
gates seed one that forgets. Tests: a ridge between two machines, a
machine on a hill and one in a pit, level ground as before, and the
walk from each end, since the ray is not symmetric.

**T45 — Water in `sim`: parts that flow, springs, sinks and the ground water, wading and flooding** `⚠HASH`

All of `09`. `water` leaves the terrain kinds; a tile holds its parts.
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

**T46 — Height and water in `render`**

`07`'s Q65 entry, the screen table and Marks. Tiles drawn at their
heights with a cliff's face; water as a see-through sheet at its
surface, on a mesh of its own as `crates/render/CLAUDE.md` requires of a
translucent layer, following the slope of water that is moving; damp
ground as a tint; a spring's and a sink's mark; a bot
standing at its tile's height and stepping between heights, on a ramp
where they differ by a level, at the speed its pace gives, and shown
falling and bonked; the ground
row on `G` with *lower*, *raise*, *level* and its clear, a ground plan
as the tile's outline at the planned height, and its ghost only where
it would change what the team knows; the flooded mark; the inspector's
height, water, a spring's or a sink's rate, and `flooded`. Machines, wires, pylon lines and
marks all sit on the ground at its height.

**T47 — The starter library and the shipped maps know height and water**

`07`'s starter tree and `data/maps`. The first map gains heights, a
lake with a spring and ground worth digging, kept fair between its
teams. The starter's `nav` steps only where a step is allowed, never
drops by accident, keeps a bot moving so that it keeps its pace, and
treats deep water as closed; `jobs` gains a ground step that carries
out the team's ground plans — mining, digging and filling, taking earth
to a depot or to a fill and fetching it when a fill needs it — and
`act` its guarded calls; a flooded bot digs or fills its way out, or
waits. The Fool keeps working on a map with heights. The starter tests
run a match in which a channel is planned and dug and no bot faults.
