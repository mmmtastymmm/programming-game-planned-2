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

**T42 — The hash reads no memory of a tile a team has never seen, and the vision pass copies nothing unchanged**

From [I12](history/inbox-triaged/inbox-triaged-0012.md). After T41, at
a hundred teams on a 208 by 208 map, 51 of the state hash's 60 ms are
the teams' memory tables: 4.3 million entries, 3.4 million of them
unknown, each of which the hash reads a wide record to learn and then
writes one byte for. Two changes, and neither moves a hash. Each team
keeps, beside its memory, which tiles it knows, written where the vision
pass first sees a tile, since that pass is the only writer of a tile's
state; the hash writes an unknown tile's byte from that and reads the
memory of the known ones only, and may fold a run of unknown tiles into
one step, FNV-1a over a zero byte being one multiplication. And the
vision pass, for a tile the team sees, sets `seen_at` and the state and
replaces a field of its memory only where it differs from the tile, so
a tick on which nothing changed copies no deposit, paint, overlay, wire
set or building record. A test holds the index to the memory's states
across walking bots, a fork's copy and a team that goes out, and the
existing one that holds the pass to a walk of the whole map still
passes. The PR shows the golden fixtures untouched and
`crates/replay/benches/scale.rs` at a hundred teams and at two, before
and after. Whether the hash should stop writing the known tiles in full
is `Q64`, and no part of this task.
