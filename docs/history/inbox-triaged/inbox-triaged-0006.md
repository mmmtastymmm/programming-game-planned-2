*Closed record — see [../README.md](../README.md). Not spec.*

# I6 — The state hash costs far more than the tick, and the renderer pays it twice

`crates/replay/benches/scale.rs` (`cargo bench -p replay --bench scale`)
times `Sim::step` and `Sim::state_hash` apart. With the starter tree
patrolling on two teams, the hash costs 5 ms against a 1 ms tick at 22
bots, 35 ms against 5 ms at 102, 239 ms against 37 ms at 502, and 2 s
against 77 ms at 2,000. The tick alone scales linearly at about 45 µs per
bot-tick to 500 bots; the hash is the ceiling. It walks every bot's
`state_bytes`, and a starter bot's state is 150 KB to 2.5 MB because
`seen_tiles()` hands each bot its own copy of the team's whole seen-tile
list and the patrol keeps it live, so the hash reads every byte of every
copy of the same team-wide data. `render` pays it twice: the driver hashes
once per tick for the exchange (`driver.rs`), and the status bar in
`ui.rs` calls `state_hash()` again every frame for its hex readout — at
100 bots, 35 ms a frame. The per-frame call could read the tick's stored
hash; the per-tick cost is a design matter — hashing team-shared data once,
or a bot's state incrementally — and which it should be is undecided.

After [T37](../../TASKS.md) took the seen-tile list out of the patrol,
the hash fell to 0.2 ms at 22 bots, 4.5 ms at 99 and 121 ms at 502. What
is left is each bot's `see()` list, held on its stack while the build
step walks it looking for sites, which is the language's cost model at
work — a unit per record returned — not a defect: a team of 250 sees 500
machines, so each bot's state is a 500-record list, and FNV-1a reads
every byte of every copy at about a nanosecond each. Cutting that means
changing `docs/06`'s hash — a per-machine composition, a faster function
— which is a question, not a fix, and none is opened: the design's scale
is ten bots a printer, two teams of 250 is twenty-five printers each, and
the benchmark stays to say when a target scale needs it.

## Outcome

- **Task:** [T36](../../TASKS.md) — the status bar reads the hash the
  driver stored when the tick ran, so a frame costs no hash. The driver's
  own per-tick hash, which the exchange needs, is unchanged.
