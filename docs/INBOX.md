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

**I12 — After T41 the hash at a hundred teams is each team's memory of the map**

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
