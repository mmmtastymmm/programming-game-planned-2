*Closed record — see [../README.md](../README.md). Not spec.*

# I10 — At a hundred teams the state hash costs more than the step

`crates/replay/benches/scale.rs` at 10,000 bots that only wait, on a
hundred teams and a 208 by 208 map, after T38: 67 ms a step and 77 ms a
state hash, where two teams pay 17 ms and 7 ms. The sampler puts most of
it in `state_hash`, which for every team walks every position on the map
twice, once asking each tile for the team's plans and once writing the
team's memory of it, so the hash is teams by tiles a call the way the
vision pass was a tick. `Map::positions` builds a fresh list of every
position for each of those walks. `bot_cap`, `printers_of` and `bots_of`
each scan every machine for one team, and the hash and the print step
call them once a team, which is teams by machines. What is left of the
step itself is the vision pass writing every visible tile's memory anew
each tick, cloning its deposit amounts, paint and overlay, though on a
tick where nothing moved almost none of it differs from what is there.

Confirmed on reading `state_hash` in `crates/sim/src/tick.rs`: for each
team it calls `Map::positions` twice, which builds the list of every
position each time, once to collect the team's plans tile by tile and
once to write its memory; and `bot_cap` there, with `bots_of` and the
count of printing printers in the print step, each scan every machine
for one team. T31 has since added to the same walks, not changed their
shape: each tile's wires and each remembered tile's wires are written in
them. None of it is a rule: the same bytes can be written from one pass
over the tiles and one over the machines.

## Outcome

- **Task:** [T41](../../TASKS.md) — the state hash and the print step
  without a pass over the map, or over the machines, for each team, the
  hash's value unchanged.
