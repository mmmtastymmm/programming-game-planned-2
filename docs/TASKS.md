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

**T26 — Forks in `sim`: positions, watches, stopping mid-tick, steps, saved states, edits**

Q50, ruled in `06`'s Forks section. `sim.save()`, the match's full state
between ticks, and a fork built from one or from a re-run; positions;
which statements and clauses begin, for hits and steps; a fork recording
the match's commands it took per tick, applying them and then its own log,
and knowing whether it is faithful, ahead, stale or diverged; `sim.step`
taking a team's watches — `(file, content hash, line, deployment or any,
machine or any)` — and reporting every hit with its position, read-only
and outside the hash; the language VM stopping at any boundary mid-slice
and resuming, and reporting frames, every run module's globals, the record
with action and progress, budget, deficit, pending interrupts and handler
state; the line, over, tick and continue steps and every stop with its
cause and position; saving and restoring a fork's state, and which saved
states a fork may use; edits from a literal or display of literals, a
`num` with an optional `-`, checked against `01`'s Limits, at positions;
what-if deploys in the fork's log; a new branch discarding later entries
and states; the PvP bound and refusal. Tests that an untouched fork
reproduces the golden stream hash for hash, that watching lines changes
no hash, that a re-run across an edit applies it, and that a fork run
ahead goes stale when the match's log gains a command.

**T27 — The debugger in `render`, and replays on the start screen**

Q50, ruled in `07`'s Debugging section and The match. Gutter breakpoints on
the shown text, moved to a line where a statement begins, carried through
edits, watching that text's content hash, scoped to a machine or the file,
greyed while nothing runs it, kept with the tree; the driver saving the
match's state every `debug_checkpoint_ticks` while the debugger is in use,
keeping `debug_checkpoints_kept`; a hit opening a fork at the tick's first
hit and, outside PvP, submitting one pause; further hits listed by
position; the debugger window, its stop causes and the fork banner with
faithful, ahead, stale or diverged; the map drawing the fork in the team's
view, the match's own controls still acting on the match; the step keys;
variable editing outside PvP; opening a fork by hand. Replays: *open a
replay* on the start screen and `--replay <path>`, playback with speed,
pause and seeking, the saver's team recorded in the file and in its name
`<map>-<tick>-<hash>-<team>.ron`, a picker for the versions its log
deployed. The three `debug_*` constants in `data/interface.toml`.

**T28 — What-if**

Q50: deploying a file into the open fork, for its next tick, instead of
the match; *run*, continue drawn at a chosen speed with a pause; *take it*
deploying the same text to the match, absent on a replay; refused in PvP.

[P5](PROBLEMS.md) closes with whichever of T26, T27 and T28 lands last.

**T29 — A paused driver wakes only for a player's command**

Q50 rule 13, amending Q32, in `06`'s Decided entry and The driver. At
speed `0`, `render`'s driver runs to the next tick carrying a player's
command. `net`'s log counts a scripted team among its senders (the
driver's `local_peers` holds the player and every scripted team), so the
fix tells a player from a scripted team rather than filtering by sender.
No replay hash changes — the sim's ticks are the same. The existing paused
test, with the Fool loaded, is unaffected, since the Fool's script has
nothing after tick 0; a new test loads a script with an entry after the
pause and asserts the driver stops short of it. Fixes [P6](PROBLEMS.md).
