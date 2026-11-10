*Closed record — see [../README.md](../README.md). Not spec.*

# I8 — Per-bot tick cost rises seven-fold between 500 and 2,000 bots

The scaling benchmark's step-only series: about 45 µs per bot-tick at 22,
102 and 502 bots, then 308 µs at 2,001, alongside a twelve-fold larger VM
state per bot (200 KB to 2.5 MB). The tick budget is 200 units a machine
regardless of bot count, so an interpreter whose work tracked its budget
would hold the per-bot cost flat; something scales with the state or the
map instead. Not profiled — candidates are the vision pass over a team
with a thousand seers, a builtin that does linear work for a fixed charge,
or freeing a seen-tile list of ten thousand records.

Profiled, with the macOS sampler on the bench at 2,000 bots. With the
starter running, 93% of the time was `sense::team_sees`, which every
`see()` call recomputed from scratch: every seer of the team against every
machine, with a line of sight for each pair in range — a thousand seers
by two thousand machines, once per bot per tick. With bots that only
wait, 80% was the vision pass's search of every machine for a building on
every visible tile, once per team. Neither was the VM state's size, which
was a symptom of the same seen-tile lists [I7](inbox-triaged-0007.md)
names.

## Outcome

- **Task:** [T35](../../TASKS.md) — a sight cache on the world, checked
  against the machines on every use, so a team's field of view is
  computed once a tick however many of its bots look; and the vision
  pass indexing buildings by tile. The queries answer as before and no
  replay hash moves.
