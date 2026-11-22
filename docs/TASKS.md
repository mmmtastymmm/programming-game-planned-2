# Tasks

What is left to build. **This file holds open tasks only** — completing one moves
it to [history/tasks-completed/](history/tasks-completed/README.md) as its own
file, with the commit that finished it. Anything here is open by definition, so
there are no checkboxes to keep in sync.

Numbering is stable — **append new tasks, never renumber**, and never reuse a
number after it moves to history. Entries are written as `**T<n> — <title>**` on
their own line, which is how `scripts/check-registers.mjs` finds them; it rejects
a number that is open and completed at once, and any gap *below* the highest
number recorded. Deleting the highest-numbered entry outright shrinks the range
and goes unnoticed — the same honest limit
[history/questions-answered/](history/questions-answered/README.md) states.

Milestones below are groupings, not entries. A milestone is finished when every
task under it has left the file.

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md). **No other register carries it** — CLAUDE.md and
  `.claude/design-invariants.md` define it, and `docs/history/` may narrate it,
  but no entry outside this file is marked. An open question is not work, and
  nearly every open question would qualify while the sim is unbuilt, so a marker
  on all of them selects nothing. Where a *ruling* cannot be discovered during
  implementation, the question says so in words.
- **Decided-but-unbuilt** work — a ruling the code has not caught up to — is
  tracked here *and* as an entry in [PROBLEMS.md](PROBLEMS.md). The register owns
  the gap; this file owns the work. A task is only lag once it is actually
  buildable; before that it is merely pending.

## M12 — Scale

**T41 — The state hash and the print step visit the map and the machines once, not once a team**

From [I10](history/inbox-triaged/inbox-triaged-0010.md). At a hundred
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
