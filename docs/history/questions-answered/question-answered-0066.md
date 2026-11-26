*Closed record — see [../README.md](../README.md). Not spec.*

# Q66 — How should water share out a difference, dry, and be fed and drained, now that it has been watched running?

## Ruling

*Ruling (2026-11-26):* **a tile gives each lower tile beside it a fifth of
the difference between them; the flow runs several passes a tick; thin
water dries only on a tick when nothing reaches it; a spring may have a
head and a sink a sill; and water is counted a thousand parts to a level
and written everywhere in levels.** Made under a new number because
[Q65](question-answered-0065.md), which it amends, is a closed record,
and opened and answered in one commit. Every part of it came from
running Q65's water on a map made to watch it and seeing what it did.

### The rules

All of them are the sim's and are hash-affecting.

1. **A fifth.** A tile's lower neighbours are the open tiles beside it
   whose surface is below its own, and it offers each a fifth of the
   difference between their surfaces, rounded down, however many of them
   there are. Q65's share, the difference over one more than the number
   of lower neighbours, is withdrawn.
2. **Passes.** The flow runs `water.passes` times in a tick, four, each
   pass from the water as the last left it. Springs, sinks and drying
   still run once.
3. **A film dries only unfed.** A tile holding `water.film` parts or
   fewer loses them to the ground water only on a tick in which it was
   given none, by a spring or a neighbour, in any pass.
4. **A spring's head.** A spring may have a head, a height above its own
   tile's, and brings up nothing that would raise its tile's surface
   past it.
5. **A sink's sill.** A sink may have a sill, a height at or above its
   own tile's, and takes nothing that stands at or below it.
6. **A thousand parts to a level.** `water.parts` is a thousand, where
   it was a hundred.
7. **Levels everywhere.** A map's rates, heads, sills and ground water,
   the tables' `wet`, `wade` and `max_rate`, and a tile's `water`,
   `spring` and `sink` as a program reads them are all in levels; each
   must come to a whole number of parts. `film` alone is in parts.

### Consequences

- **Water in a channel settles.** With Q65's share a row of tiles in a
  channel swapped back and forth for ever, and nothing downstream of it
  ever filled.
- **A spring and a sink make a river.** With a head at the height of the
  banks and a sill just under it, a cut fills to its banks and holds,
  the same tick after tick, with nothing running over.
- **A stream can wet a floor and a channel can fill.** Under Q65 a few
  parts a tick dried as they landed, and a lake beside a trench it could
  not fill quickly leaked away through its leading edge.
- **Flow needs a slope.** What a stretch of water carries is in
  proportion to how steeply its surface falls, whatever its depth. A
  level river carries next to nothing, and passes are the only thing
  that carries more on the same slope.
- **A lake's surface may slope by four thousandths of a level a tile**,
  since tiles less than five parts apart exchange nothing.
- **How finely water is counted is one number.** No map and no program
  changes when `water.parts` does.

## Outcome

- **Docs:** [09-water.md](../../09-water.md) — a Q66 Decided entry and
  the Q65 entry, Water on a tile, the water step's four steps and what
  they add up to, Springs and sinks, the map's water, what a program
  reads, Data, and what it leaves open.
  [03-world.md](../../03-world.md) — the tile's `water`, `spring` and
  `sink`, and the map's water and its checks.
  [06-architecture.md](../../06-architecture.md) — the tick's step 7.
  [00-overview.md](../../00-overview.md) — the Decided listing.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T45](../../TASKS.md), amended — water in the sim is these
  rules.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| a tile's share | The difference over its lower neighbours and itself *(Q65)* | In a channel that is half, and a row of tiles then swaps back and forth for ever. |
| a tile's share | **A fifth, always** *(chosen)* | Tiles under five parts apart exchange nothing; water moves more slowly in a pass. |
| a stream too big for its slope | Tune each map's spring and bed | A long level channel still carries little, which will matter once bots dig canals. |
| a stream too big for its slope | **Several passes a tick** *(chosen)* | Moving water costs that many times the work. |
| a stream too big for its slope | Water with momentum | Closest to real water; more state, and water that surges. Kept in mind. |
| thin water | Dries whenever it is thin *(Q65)* | A stream dries as it lands, and a lake leaks away through a front it cannot push on. |
| thin water | **Dries only when nothing reached it that tick** *(chosen)* | A spill spreads further and takes longer to go. |
| a spring that spills | Left to the map | Whether a river spills depends on its length and its bed. |
| a spring that spills | **A head it will not raise its water past** *(chosen)* | A spring with one runs slower than its rate once its channel is full. |
| a river that stays thin | A weak sink | The river fills, but how full depends on tuning the two rates. |
| a river that stays thin | **A sill the sink will not drain below** *(chosen)* | One more number on a sink. |
| how finely | A hundred parts to a level *(Q65)* | A full river kept a slope of most of a level along its length. |
| how finely | **A thousand** *(chosen)* | Still water takes somewhat longer to come to rest. |
| how finely | A hundred thousand | Lakes take far longer to go quiet, and a film a few parts thick spreads much wider before it dries. |
| what amounts are written in | Parts | Every map changes when the count does. |
| what amounts are written in | **Levels** *(chosen)* | A decimal must come to whole parts, and is written as a string. |

### How the answer took its shape

Q65's water was built and drawn, and a map, `falls`, made to watch it: a
spring at the head of a stepped bed and a sink in a basin at its foot.
The user asked whether the basin was meant to fill. It was not filling
even with a weaker sink, and looking at it tick by tick showed the river
swapping between two states: Q65's share, in a channel, is exactly the
amount that never settles. A fifth cured that, and showed the next
thing, that a thin stream dried as it reached the basin's floor; drying
only what nothing had reached that tick cured that in turn, and stopped
a lake leaking through a stalled front.

The user then saw the water piling up at the spring and running over the
banks, and asked whether it was decaying away from the spring. It was a
straight ramp: flow is in proportion to slope, so a spring's stream
needs a slope the banks did not allow. Of tuning the map, running the
flow several times a tick, and giving water momentum, the user chose
passes. They then said what they wanted of a spring and a drain: a
perfect loop with no spills. A head on the spring gave that; and when
the loop ran thin, and they said a river that fills right up would feel
more natural, a sill on the sink gave that. Seeing the full river still
step down along its length, they asked for a level surface: part of the
drop was the head standing a level above the sill, and part was
rounding, and counting a thousand parts to a level with a sill set a
tenth of a level under the head made both too small to see. Asked
whether a hundred thousand would be better still, the drafting advised
against it for the work it costs and for what it does to drying, and
proposed instead that every amount be written in levels, so that the
number can be changed later with nothing else to touch; the user kept a
thousand.
