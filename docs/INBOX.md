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

**I10 — At a hundred teams the state hash costs more than the step**

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

**I11 — A deficit larger than one tick's budget is forgiven after one tick**

Found while building T39, and not T39's. `01`'s Metering says a deficit
reduces each following tick's budget "by what remains of it until it is
paid". `Machine::close_slice` in `crates/lang/src/vm.rs` sets the new
deficit to `spent - slice_budget`, saturating, and the slice's budget is
the tick budget less the deficit, saturating at zero: so a deficit above
one tick's budget opens a slice of zero, which spends nothing and closes
with a deficit of zero. A program whose one operation costs 3,006 units
against a budget of 200 was seen to carry 2,806, sit out one tick, and run
at full budget on the next, where the doc has it sit out fourteen. A
deficit below the tick budget is carried correctly. Fixing it moves the
golden hashes of any replay that overspends by more than a tick.
