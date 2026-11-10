*Closed record — see [../README.md](../README.md). Not spec.*

# I9 — The vision pass costs every team a pass over every tile, each tick

`crates/replay/benches/scale.rs` at 10,000 bots that only wait: 19 ms a
tick on two teams, 219 ms on a hundred, on the same 208 by 208 map. The
sampler puts the difference in step 8: for each team the pass walks every
position on the map and rewrites that team's memory for the tile —
cloning its paint, overlay and deposit amounts when visible, demoting it
when not — so the pass is teams by tiles, 4.3 million tile memories a
tick there, whatever the teams see. Only a tile that is visible now, or
was at the last pass, can change; a team's visible set from the previous
pass, kept on the team, would bound the work to what it sees plus what it
just stopped seeing. The sight cache (T35) has a second cost at that
scale: its basis check compares every machine on every query, so ten
thousand `see()` calls over ten thousand machines is a hundred million
comparisons a tick, 5% of the sample; a generation counter bumped at
every move, placement and removal would make the check constant, at the
price of every such site having to bump it. For the record, the starter
at 10,000 bots runs at 3 ticks a second and takes 33 to 37 GB of memory,
nearly all of it each bot's `see()` list of every machine its team sees,
which is the cost model I6 describes and not this entry's.

Confirmed on reading `vision_pass` in `crates/sim/src/tick.rs`: a tile
that is neither visible now nor `Visible` in the team's memory is left as
it stands, so the pass bounded to those two sets writes the same memory
the full walk does, and no replay hash can move. The sight cache's basis
check in `crates/sim/src/sense.rs` is a full comparison of every machine's
id, position, model and team on every `ensure_sight`, which is every
`see()`, every attack and every team's vision pass.

## Outcome

- **Task:** [T38](../../TASKS.md) — the vision pass bounded to each
  team's visible tiles and the tiles its memory still holds as visible,
  and the sight cache told of each change by a generation counter rather
  than finding it by comparison.
