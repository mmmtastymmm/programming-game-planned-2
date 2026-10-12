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

Q52, ruled in `06`'s Forks section. `sim.save()`, the match's full state at
a tick's end, and a fork built from one or from a re-run; positions and
their order, the step-4 damage position included; statements beginning on
their `op.statement` and the clauses that begin again; a fork recording the
match's commands it took per tick, applying them and then its own log, with
its faithful, ahead, stale and diverged flags (`SetSpeed` never stales it);
content hashes; `sim.step` taking a team's watches and reporting, per
watched machine and tick, the first beginning with its position and a
count, read-only and outside the hash; the language VM stopping at any
boundary mid-slice and resuming, and reporting frames, every run module's
globals, the record with action and progress, budget, deficit, pending
interrupts and handler state; the line, over, tick, continue and run steps,
the step limit as an argument, and every stop with its cause and position,
the match's end among them; saving and restoring a fork's state, which
saved states a fork may use, and the re-run fallback; edits at slice
positions or a tick's end, naming a frame by depth, with live values
counted as an assignment would; what-if deploys in the fork's log; a fork's
snapshot at any position; a new branch discarding later entries and states;
the PvP bound and refusal. Tests that an untouched fork reproduces the
golden stream hash for hash, that watching lines changes no hash, that a
re-run across an edit applies it, that a yield before a watched statement
is no hit, and that a fork run ahead goes stale when the match's log gains
a `Mark` but not a `SetSpeed`.

**T27 — The debugger in `render`, and replays on the start screen**

Q52, ruled in `07`'s Debugging section and The match. Gutter breakpoints on
the shown text, moved to a line where a statement begins, carried through
edits, watching that text's content hash, scoped to a machine or the file,
greyed while nothing runs it, kept with the tree; version windows, read-
only, for what a machine runs and what a replay ran; the driver saving the
match's state at once when debugging starts and at every multiple of
`debug_checkpoint_ticks`, keeping `debug_checkpoints_kept` and every state
an open fork or listed hit needs; a hit opening a fork at the tick's
earliest hit and, in a live single-player or PvE match, submitting a pause
unless the player's last speed is already `0`, a replay's playback pausing
instead; further hits listed, the last `debug_hits_kept`; the debugger
window, its stop causes and the fork banner with its state flags; the map
drawing the fork in the team's view, the mark tools resting and *show the
match* restoring them; the step keys and *run*'s speed; variable editing
outside PvP. Replays: *open a replay* on the start screen and `--replay
<path>`, its driver as `06` rules — sending no command, paced and paused by
the viewer, saving states from before tick 1, seeking by restoring one; the
saver's team in the file and its name `<map>-<tick>-<hash>-<team>.ron`;
version windows for the team's own deployed versions. The `debug_*`
constants in `data/interface.toml`.

**T28 — What-if**

Q52: *try in the fork* on each file window, beside deploy in a match and
alone on a replay, sending the file's deploys into the open fork for its
next tick; *take it* deploying the same text to the match; refused in a
live PvP match.

[P5](PROBLEMS.md) closes with whichever of T26, T27 and T28 lands last.

**T29 — A paused driver wakes only for a player's command**

Q52 rule 13, amending Q32, in `06`'s Decided entry and The driver. At speed
`0`, `render`'s driver runs to the next tick carrying a player's command,
through a `net` query that can tell one — `06`'s pseudocode names it
`next_tick_with_a_player_command`. `net`'s log counts a scripted team among
its senders (the driver's `local_peers` holds the player and every scripted
team), so the fix tells a player from a scripted team rather than filtering
by sender. No replay hash changes — the sim's ticks are the same. The
existing paused test, with the Fool loaded, is unaffected, since the Fool's
script has nothing after tick 0; a new test loads a script with an entry
after the pause and asserts the driver stops short of it. Fixes
[P6](PROBLEMS.md).
