*Closed record — see [../README.md](../README.md). Not spec.*

# I12 — After T41 the hash at a hundred teams is each team's memory of the map

`crates/replay/benches/scale.rs` at 10,000 bots that only wait, on a
hundred teams and a 208 by 208 map, after T41: about 66 ms a step and
60 ms a state hash, where two teams pay 26 ms and 8 ms. Timed by
section, 51 of the hash's 60 ms are the teams' part, which is nearly all
each team's memory: a hundred teams by 43,264 tiles is 4.3 million
entries, 876,004 of them known. A known entry costs about 30 ns to
write, as it does at two teams; the 3.4 million unknown ones write one
byte each and still cost about 7 ns apiece, which looks like the read
of one field out of a wide record a tile. The tiles themselves take
1 ms and the machines 4 ms. In the step, what I10 said is left is still
there: the vision pass writes every visible tile's memory anew each
tick.

Confirmed on reading `state_hash` and `vision_pass` in
`crates/sim/src/tick.rs`. The hash reads the state of every team's
memory of every tile to learn that most are unknown, and a memory is a
wide record, the building's whole record among its fields. The vision
pass builds a new memory for every visible tile on every tick, copying
the tile's deposit amounts, paint, overlay and wires and the building's
record, where on most ticks only `seen_at` differs from what is there.
Neither is a rule: the same bytes can be hashed, and the same memory
kept, without either. What cannot be had that way is the cost of the
tiles a team does know, which the hash's definition in `06` has it
write in full on every tick; the user chose to open both the work that
moves no hash and the question that would.

## Outcome

- **Task:** [T42](../../TASKS.md) — the hash without a read of the
  memory of a tile a team has never seen, and the vision pass without a
  copy of what has not changed, no hash moving.
- **Question:** [Q64](../../QUESTIONS.md) — how the state hash covers a
  team's memory.
