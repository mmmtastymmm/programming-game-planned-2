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

## M8 — The debugger

**T26 — Forks in `sim`: watches, stopping mid-tick, steps, saved states, edits**

Q50, ruled in `06`'s Forks section. A fork from a saved state or a
re-run, applying the match's (or replay's) log and then its own, faithful
while its own log is empty; `sim.step` taking a team's watches — `(file,
content hash, line, deployment or any, machine or any)` — and reporting
the machines that begin a statement on a watched line, read-only and
outside the hash; the language VM stopping at an operation boundary
mid-slice and resuming, and reporting frames, every run module's globals,
the record with action and progress, budget, deficit, pending interrupts
and handler state; the line, over, tick and continue steps and every
stop and its cause; saving and restoring a state at a tick boundary;
edits from a literal or display of literals, checked against `01`'s
Limits, entered in the fork's log at (tick, machine, operation count);
what-if deploys in the fork's log; a new branch discarding later entries
and states; the PvP bound and refusal. Tests that an untouched fork
reproduces the golden stream hash for hash, that watching lines changes
no hash, and that a re-run across an edit applies it.

**T27 — The debugger in `render`, live and on replays**

Q50, ruled in `07`'s Debugging section. Gutter breakpoints on the shown
text, carried through edits, watching that text's content hash, scoped to
a machine or the file, greyed while nothing runs it, kept with the tree;
saving the match's and each fork's state every `debug_checkpoint_ticks`,
keeping `debug_checkpoints_kept`; a hit opening a fork and, outside PvP,
submitting one pause; further hits counted; the debugger window, its stop
causes and the fork banner; the map drawing the fork in the team's view;
the step keys; variable editing outside PvP; opening a fork by hand and
from a replay on the start screen, with a picker for the replay's file
versions; the saver's team recorded in a saved replay, outside its
identity; the three `debug_*` constants in `data/interface.toml`.

**T28 — What-if**

Q50: deploying a file into the open fork, for its next tick, instead of
the match; running the fork at a chosen speed; *take it* deploying the
same text to the match; refused in PvP. Together with T26 and T27,
finishing it fixes [P5](PROBLEMS.md).

**T29 — A paused driver counts only peers' commands**

Q50 rule 12, amending Q32, in `06`'s Decided entry and The driver. At
speed `0`, `render`'s driver (and `net`'s `next_tick_with_a_command`, or
a variant of it) runs to the next tick carrying a peer's command, not a
scripted team's. No replay hash changes — the sim's ticks are the same —
but the driver tests that pause with a card loaded change. Fixes
[P6](PROBLEMS.md).
