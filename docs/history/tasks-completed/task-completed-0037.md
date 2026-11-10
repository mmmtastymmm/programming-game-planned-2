*Closed record — see [../README.md](../README.md). Not spec.*

# T37 — The starter's patrol costs the same on any map

From [I7](../inbox-triaged/inbox-triaged-0007.md). `lib/jobs.py`'s
patrol walked a loop round the seen part of the map, measuring it with
`seen_tiles()` on every step: a unit per seen tile, which on a big map was
most of the tick budget, and at two thousand bots on a 98 by 98 map no
bot moved in a hundred ticks. The patrol now walks straight until the
tile ahead is off the map, rock, water, a building or another bot, or the
leg has run `LEG_STEPS`, then turns: one tile looked at and one step,
whatever the map. A tile next to the bot that the team does not know is
off the map, since a bot always sees the tiles around it. `corners()` and
`INSET` go; `seen_bounds()` stays in `lib/sense.py` for players who want
it. `bot.py`'s header says what the patrol does now (DI8).

Completed in `0665332`, the merge of #101, which did the work and triaged the
inbox entry that asked for it in one PR; this file closes the task.
