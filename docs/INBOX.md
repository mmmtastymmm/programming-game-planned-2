# Inbox

Raw observations, in whatever words came out. **This file is never spec and holds
no rulings.**

The point of an inbox is that writing something down must be cheaper than
deciding where it goes. Dump it here; triage it later. An entry left here is a
review finding, not a backlog item — a doc pass that walks past this file has
missed something.

**This file holds untriaged entries only.** Triaging one moves it to
[history/inbox-triaged/](history/inbox-triaged/README.md) as its own file,
declaring where it went. Numbering is stable — **append new entries, never
renumber**, and never reuse a number. Entries are written as
`**I<n> — <short title>**` on their own line, which is how
`scripts/check-registers.mjs` finds them.

Triage means turning an entry into at least one of the outcomes below — two is
common, since an observation that opens a task often gets a doc fix at the same
time. The triaged file's `## Outcome` declares which, in these spellings and no
others:

- `- **Question:**` — a numbered question in [QUESTIONS.md](QUESTIONS.md),
  something undecided;
- `- **Problem:**` — a numbered entry in [PROBLEMS.md](PROBLEMS.md), decided
  text that is wrong;
- `- **Task:**` — a numbered task in [TASKS.md](TASKS.md), work to do;
- `- **Docs:**` — a documentation fix made on the spot, with the edits linked;
- `- **Dropped:**` — nothing, because it turned out to be a misreading.

That last outcome is the one this register has and the others do not, and it is
deliberate: an inbox that cannot absorb a false alarm stops being cheap to write
to, and then people stop writing to it. A dropped entry still gets a file and
still has to say why.

## Open

**I6 — The state hash costs far more than the tick, and the renderer pays it twice**

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

**I7 — The starter's patrol stops scaling before the sim does**

In the same benchmark, no bot of 2,000 on a 98 by 98 map moved in 121
ticks, while every bot of 500 on 53 by 53 moved: the team's seen set is
the whole map, `seen_tiles()` costs one unit per tile returned, and the
patrol's work per decision is linear in the seen set under a 200-unit
tick budget, so a decision takes longer than the window. Fifty teams of
ten bots — the shape a printer cap of ten produces — moved 427 of 508 in
137 ticks on the same map, where two teams of 250 moved all 500 in 200.
The sim is not the limit here; the starter is, and it is the program a
new player reads first (DI8). A patrol bounded by a box or a sample of
the seen set, rather than the whole list, would scale with the map
instead of the team. Whether the starter should be written for that scale
at all, or whether the budget should make a team-sized list unaffordable
by design, is undecided.

**I8 — Per-bot tick cost rises seven-fold between 500 and 2,000 bots**

The same benchmark's step-only series: about 45 µs per bot-tick at 22,
102 and 502 bots, then 308 µs at 2,001, alongside a twelve-fold larger VM
state per bot (200 KB to 2.5 MB). The tick budget is 200 units a machine
regardless of bot count, so an interpreter whose work tracked its budget
would hold the per-bot cost flat; something scales with the state or the
map instead. Not profiled — candidates are the vision pass over a team
with a thousand seers, a builtin that does linear work for a fixed charge,
or freeing a seen-tile list of ten thousand records. Worth a profile
before any tuning rests on the 500-bot numbers.
