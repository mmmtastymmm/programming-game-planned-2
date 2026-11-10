*Closed record — see [../README.md](../README.md). Not spec.*

# I7 — The starter's patrol stops scaling before the sim does

In the scaling benchmark (`crates/replay/benches/scale.rs`), no bot of
2,000 on a 98 by 98 map moved in 121 ticks, while every bot of 500 on 53
by 53 moved: the team's seen set is the whole map, `seen_tiles()` costs
one unit per tile returned, and the patrol's work per decision is linear
in the seen set under a 200-unit tick budget, so a decision takes longer
than the window. Fifty teams of ten bots — the shape a printer cap of ten
produces — moved 427 of 508 in 137 ticks on the same map, where two teams
of 250 moved all 500 in 200. The sim is not the limit here; the starter
is, and it is the program a new player reads first (DI8). A patrol bounded
by a box or a sample of the seen set, rather than the whole list, would
scale with the map instead of the team. Whether the starter should be
written for that scale at all, or whether the budget should make a
team-sized list unaffordable by design, is undecided.

## Outcome

- **Task:** [T37](../../TASKS.md) — the patrol walks straight and turns
  at the map's edge or an obstacle, one tile looked at and one step per
  tick, so it costs the same on any map. The budget question was not
  taken up: the budget already makes a team-sized list dear, which is
  what the stall was, and a starter that does not ask for one is the
  right lesson to ship.
