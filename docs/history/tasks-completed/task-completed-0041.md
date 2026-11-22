*Closed record — see [../README.md](../README.md). Not spec.*

# T41 — The state hash and the print step visit the map and the machines once, not once a team

From [I10](../inbox-triaged/inbox-triaged-0010.md). At a hundred
teams the state hash costs more than the step it follows, because it
walks every position on the map twice for every team, building the list
of positions each time, and counts each team's printers and bots by
scanning every machine. Every tile's plans, for every team that has any,
are in the tile, so one pass over the tiles gives each team its plans in
row-then-column order; a team's memory is its own table and needs no
list of positions to walk; and one pass over the machines counts every
team's bots, printers and prints in progress, for the hash's cap and for
the print step. The hash writes the same bytes in the same order, so no
replay hash moves, and the PR shows it: the golden fixtures untouched,
and `crates/replay/benches/scale.rs` at a hundred teams before and
after.

Completed in `7cc61a2`, the merge of #129; the task's own commit is
`64a5c82`. `World::rosters` gives every team its machines in id order
with its bots, printers and prints in progress from one pass, and
`Map::walk` gives every tile in the hash's order with its index and
builds no list. The hash, the print step and the out-teams step use
them. The power step, which since T31 scanned every machine for each
team three times over, was moved onto the rosters too, beyond what the
task named. No golden fixture moved. `crates/replay/benches/scale.rs`
at 10,000 bots that only wait on a 208 by 208 map: at a hundred teams
the step went from about 88 ms to 66 ms and the hash from about 91 ms to
60 ms; at two teams the step stayed at about 25 ms and the hash went
from 8.7 ms to 7.9 ms. What is left of the hash at a hundred teams is
each team's memory of the map, 51 of its 60 ms, which the hash's
definition has it write; the measurement is in the inbox as I12, open
when this closed. It was the one task under M12, Scale.
