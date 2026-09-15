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

## M6 — The match's edges

**T19 — The start and end screens, and the saved replay**

Q36: a start screen listing `data/maps` and `data/opposition`, taking the
programs directory, offering play, host or join, and building the driver
as `main.rs` does; an end screen with the winner or draw, the tick, the
hash and the replay's path, *again* and *quit*; the replay written as
`replays/<map>-<tick>-<hash>.ron` inside the programs directory whenever a
match ends or desyncs (T21 saves on a desync too). Every command-line flag
stays and skips the screen. Nothing touches a hash.

**T20 — The mark strip, drag marking, the armed tool and the plan preview**

Q37: the tools window as three rows from data, each ending in a clear,
with `B`/`P`/`O` and `1`–`9`; a tool armed until `Esc`; a drag marking
every tile it crosses once and not panning; `Shift`+click an `Unmark` of
the armed kind; the player's own plans drawn as a translucent version of
their effect and the armed tool as a ghost under the cursor. The snapshot
gains the player's team's plan values per tile — a read the renderer
makes, in no hash — replacing the driver's `snapshot_plan` peek.

**T21 — The stall banner with resign, and the desync freeze**

Q38: a banner over the map past `stall_report_ticks` naming the peer, the
tick and the wait, with a resign beside it and a note of when the resign
lands; on a desync, a red banner with the report and the replay's path,
the replay written at once (T19's writer), and *leave* to the end screen.
Nothing touches a hash.
