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

**I9 — The vision pass costs every team a pass over every tile, each tick**

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
