*Closed record — see [../README.md](../README.md). Not spec.*

# Q65 — Should every tile have a height that bots change, with water that stands on it and flows?

## Ruling

*Ruling (2026-11-24):* **yes. Every tile has a height; bots dig it down,
fill it up and mine rock away; and water is small parts that stand on
the ground and flow to lower ground a tile a tick, brought up by springs
and taken down by sinks from one store of ground water, flooding what
they cover.** Made under a new
number because every ruling it touches is a closed record, and opened and
answered in one commit. It is the first half of a water system the user
set out to design; pumps, pipes, storage and what uses water are left for
the next question, and the user chose to settle the ground and the water
on it first.

It amends [Q28](question-answered-0028.md) on the terrain kinds, which
become two: `water` is no longer one. It amends
[Q21](question-answered-0021.md) on what a tile is and what a team
remembers of it, and withdraws the rule that a tile's terrain never
changes; [Q22](question-answered-0022.md) on resources, which gain
`earth`; [Q7](question-answered-0007.md) on line of sight, which height
now blocks and stretches; [Q10](question-answered-0010.md),
[Q27](question-answered-0027.md), [Q37](question-answered-0037.md) and
[Q49](question-answered-0049.md) on plans and the strip, which gain the
ground plan; and [Q62](question-answered-0062.md) on the printer, which
is dark when flooded though never for want of power.

### The rules

Rules 1 to 14 are the sim's and are hash-affecting; rule 15 is the
renderer's.

1. **Every tile has a height**, a whole number of levels from the map,
   rock included.
2. **A bot steps at most one level.** A move needs ground, heights at
   most one apart, and water no deeper than a bot wades.
3. **Bots change the ground.** `dig` lowers an adjacent tile a level,
   `fill` raises one a level, `mine` turns rock to ground at its height.
   Not a tile with a building, a site, a deposit, a spring or a machine.
   Any team's bot may do any of them.
4. **Earth is carried.** Digging puts earth in the bot's load and filling
   takes it; it is a second resource, held in depots like ore. Mining
   yields nothing.
5. **The player plans a height.** A ground plan is a fifth kind of plan,
   on any tile the team has seen without a deposit or a spring; on rock
   it means mine it. The team's own bot that brings the tile there
   consumes it.
6. **Height blocks sight**: a tile standing above the straight line
   between the ground of the seer's tile and the ground of the seen one,
   in integers.
7. **Height stretches sight**: a machine sees one tile further for each
   level it stands above the tile looked at, and one less for each below.
8. **Water is not terrain.** A tile is ground or rock, and water is a
   whole number of parts standing on ground, a hundred to a level. Rock
   and the map's edge are walls to it.
9. **Water flows a tile a tick.** In a water step after regrowth and
   before the vision pass, every tile offers each lower tile beside it
   its share of the difference between their surfaces, counting itself
   among those sharing, and every flow is applied together.
10. **Level water stays put.** Surfaces within a part of each other
    exchange nothing, and a share that rounds to nothing is not given.
11. **Springs and sinks.** A spring moves ground water onto its tile and
    a sink moves water on its tile into the ground water, each at its
    rate.
12. **Thin water dries** into the ground water: a tile holding four
    parts or fewer loses them, a spring's own tile aside.
13. **The total is fixed.** The parts on the tiles and the ground water
    together never change; water only moves.
14. **Flooding.** A bot wades one level and is flooded by more; a
    building or a site is flooded on a wet tile, one holding a tenth of
    a level or more. A flooded bot cannot move; a
    flooded building is dark; every flooded machine loses health each
    tick.
15. **The map shows it.** Tiles at their heights, water at its surface,
    damp ground tinted, springs and sinks marked, a ground row in the
    strip, a mark over a flooded machine.

### Consequences

- **The ground is a tool.** A ramp, a wall, a moat and a sump are all
  dug or filled; a cliff two levels high stops a bot.
- **Water can be moved without a pump.** A channel dug from a lake
  fills as the lake runs into it; a rim cut open drains a lake to the
  sill, over time, and the flood runs downhill with a front that can be
  watched and raced.
- **Flooding is a weapon and a hazard.** A base in a hollow below a lake
  can be drowned by one bot with a load of patience, and by its owner's
  own digging.
- **A printer can be dark.** Power never darkens one; water does.
- **Hills are worth holding.** A machine on high ground sees further and
  over what stands below it.
- **Water stays only where the ground holds it.** A spill on a plain
  spreads, thins and dries back into the ground water; a hollow keeps
  it.
- **Moving water costs work every tick**, and a lake's surface can hold
  a slope of about a part a tile.
- **Every replay hash moves**, and the map gains heights.

## Outcome

- **Docs:** [09-water.md](../../09-water.md), new — the whole of water.
  [03-world.md](../../03-world.md) — a Q65 Decided entry and the Q49
  entry, the tile's fields, the terrain table, new sections Height and
  Changing the ground, the line of sight's height rules, the map and the
  tick. [02-machines.md](../../02-machines.md) — a Q65 Decided entry, the
  Q22 entry, `flooded`, the `tile` and `plans` queries, the sound causes,
  `move`, the actions `dig`, `fill` and `mine`, and the causes of damage.
  [06-architecture.md](../../06-architecture.md) — the tick's steps 3, 4,
  7 and 8, what is dropped on its tick, the snapshot and the state hash.
  [07-interface.md](../../07-interface.md) — a Q65 Decided entry, the
  screen table and Marks. [08-power.md](../../08-power.md) — a flooded
  building in the power step and the Decided entry's printer.
  [00-overview.md](../../00-overview.md) — the Decided listing and the
  table of docs. [CLAUDE.md](../../../CLAUDE.md) — the list of docs.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T43](../../TASKS.md) — height, a step, the three actions,
  earth and the ground plan in the sim.
- **Task:** [T44](../../TASKS.md) — sight over height in the sim.
- **Task:** [T45](../../TASKS.md) — water in the sim.
- **Task:** [T46](../../TASKS.md) — height and water in the renderer.
- **Task:** [T47](../../TASKS.md) — the starter library and the shipped
  maps.
- **Problem:** [P11](../../PROBLEMS.md) — the docs state Q65 as current
  and none of it is built.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| what bots change | **Dig, fill, drain by pumping, mine rock** *(all chosen)* | The ground is no longer fixed; sight, paths and maps all follow. |
| how water moves | Level by body *(drafted first)* | Cheap and exact; no flow to watch. |
| how water moves | Spreading from sources *(chosen first, then set aside)* | Water is never scarce and does not behave like water. |
| how water moves | A conserved volume in whole levels, one level a body, settled at once *(chosen second, and drafted in full)* | Bodies, remainders and four moves; and water short of a level had to soak away or hide in dry ground. |
| how water moves | **Small parts that flow a tile a tick** *(chosen)* | Work every tick on moving water; a lake's surface can slope by a part a tile. |
| height | A few depths below flat ground | No hills; sight and movement unchanged. |
| height | No depth at all | A draining lake has no order to dry in. |
| height | **A full height map** *(chosen)* | Maps, movement, sight and the renderer all change. |
| how fine | Ten parts to a level | A lake's slope shows. |
| how fine | **A hundred** *(chosen)* | A film spreads wider before it dries. |
| what counts as wet | Any water | A film a part deep floods a building. |
| what counts as wet | **A tenth of a level** *(chosen)* | Damp ground does nothing. |
| how fast | **The whole share each tick** *(chosen)* | A front moves a tile a tick. |
| how fast | A cap on what crosses in a tick | Slower floods; one more number. |
| a tile's share | Half the difference to each lower neighbour | A small rise on a flat pool sloshes for ever. |
| a tile's share | **The difference over its lower neighbours and itself** *(chosen)* | Specks of up to four parts can be too small to share. |
| thin water | Stays as a film | A spill coats a plain and never goes. |
| thin water | Dries at exactly one part *(the user's first thought)* | Specks of two to four parts never get there. |
| thin water | **Dries at four parts or fewer** *(chosen)* | A lake a few parts over its rim loses them. |
| the supply | Springs with a head, and soaking as the only loss | Water is made and destroyed. |
| the supply | **Springs and sinks on one store of ground water** *(chosen)* | The total is fixed; a spring is idle while the ground is empty. |
| a flooded machine | Disabled until dry | Flooding stops a base without destroying it. |
| a flooded machine | Damaged each tick | Flooding kills. |
| a flooded machine | **Both** *(chosen)* | A flooded base is lost unless drained quickly. |
| wading | Never | Any channel is a barrier. |
| wading | **One level deep** *(chosen)* | Only deep water blocks; a bot is flooded by more than a level. |
| dig and fill | Dig is time, fill is ore | No new resource. |
| dig and fill | **Earth is carried** *(chosen)* | A second resource in every load, and earth to haul. |
| dig and fill | Both time only | Terrain is cheap to reshape. |
| sight range | **By height difference, up and down** *(chosen)* | Looking up from a pit reaches little. |
| sight range | Gain only | Pits do not hurt. |
| what blocks a line | Higher than both ends | A machine on a peak sees through a ridge in front of its target. |
| what blocks a line | **Above the straight line between the two** *(chosen)* | One multiplication a tile; a ridge hides what is behind it. |
| rock and water | **Rock is a wall to water** *(chosen)* | Mining a rock beside a lake can release it. |
| the map's edge | **A wall** *(chosen)* | Water never leaves the map. |
| the ground plan | Lower or raise by one | Many strokes for a deep cut. |
| the ground plan | **A height to reach** *(chosen)* | One mark; bots dig or fill until the tile is there. |

### How the answer took its shape

The user asked to design the water system: buildings that consume water,
a network, pumps beside water, and storage. They wanted the terrain to
change too, saw that this needed rules for terrain and for how water
flows, and chose to do those first. Four rounds of questions followed.
The first settled that bots dig, fill, drain and mine, that land has a
full height map, and that water is finite and refilled; for how water
moves the user first chose spreading from sources, and in the second
round, shown that a finite supply did not fit it, chose a conserved
volume instead. The same round fixed one level a step, sight that height
blocks and stretches, and rock as a kind of terrain. The third took
whole levels, running downhill, springs with a head and settling at
once; the fourth chose both harms for a flooded machine, wading one
level deep, earth that is carried, and range by height difference.

That design was written out in full: bodies of water with one level
each, a remainder for what made no level, and four moves by which a
body owed, dropped, ran over and rose until all were at rest. Reading it
back, two things stood out to the user: a shallow pond that digging
beside destroyed, and a body's last level vanishing when it fell, both
from the rule that water short of a level soaked away. Asked how to
change that rule, the user instead changed the model: water as small
parts that flow. It was close to what they had first pictured, and it
removed bodies, remainders, the moves and soaking together, since water
that is too thin simply stops.

Two things were then the user's own: that the thinnest water should dry
so that no film stays, and that a sink should be a spring's opposite, so
that the water of a map is by default a fixed amount. The drafting made
the first "four parts or fewer", since the share rule can leave specks
that never reach one part; and read the second as one store of ground
water that sinks and drying fill and springs draw on.

The calls that were the drafting's, put to the user and accepted, still
stand: that a bot is flooded only by more than it wades; that water is
no terrain kind; that rock and the map's edge are walls to water; that a
line is blocked by what stands above the straight line between its
ends; that the player marks a height and bots carry it out; and the
limits on what may be dug.
