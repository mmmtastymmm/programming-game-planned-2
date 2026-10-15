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

Q55, ruled in `06`'s Forks section. `sim.save()`, the match's full state at
a tick's end, and a fork — a type of its own wrapping a world, holding
every entry point below, none of which the match's sim has — built from one
or from a re-run, standing at that tick's end stopped as *opened*;
positions and their order, the step-4 damage position included; statements
beginning on their `op.statement` and the clauses that begin again, a
docstring never, a `while` each time its condition is tested, as [T30](history/tasks-completed/task-completed-0030.md) charges it;
a fork recording the match's commands it took per tick, applying them and
then its own log, with its faithful, ahead, stale and diverged flags
(`SetSpeed` never stales it); content hashes; `sim.step` taking a team's
watches and reporting, per watched machine and tick, the first beginning
with its position and a count, read-only and outside the hash; the language
VM stopping at any boundary mid-slice and resuming, and reporting frames,
every run module's globals, the record with action and progress, budget,
deficit, pending interrupts and handler state; the line, over, tick,
continue and run steps, *tick* reporting watched beginnings as `sim.step`
does and stopping at none, *run* paused at a tick's end, a stop at the
tick's end for a machine step 5 removes with its team, *back* not offered
at a first stop, the step limit as an argument, and every stop with its
cause and position, the match's end among them; saving and restoring a
fork's state, which saved states a fork may use, and the re-run fallback,
taking the match's log only before the fork's first tick and its own record
after; edits at slice positions or a tick's end, never of a machine at its
own death, each entry naming its machine and a frame by depth, with live
values counted as an assignment would; what-if deploys in the fork's log; a
fork's snapshot at any position, the team's own machines as they stand and
everything else as at the last completed tick's end; a new branch
discarding later entries and states; what-if deploys validated as a
submission is; the PvP bound and refusal. `06`'s forks-and-watches gate: a
golden replay watched on every line gives its stream unchanged, and a fork
stopped and resumed at every boundary, one restored from each saved state,
and a replay's playback each reproduce it — with mutations in
`scripts/lib/rust-gates.mjs` that make a watch charge budget and a resumed
slice re-charge its deficit, each of which the gate must catch. Tests that
a re-run across an edit applies it to its machine, that a yield before a
watched statement is no hit, that a fork run ahead goes stale when the
match's log gains a `Mark` but not a `SetSpeed`, and that a stale fork's
re-run takes none of the commands that staled it.

**T27 — The debugger in `render`, and replays on the start screen**

Q55, ruled in `07`'s Debugging section and The match. Gutter breakpoints on
the shown text, moved to a line where a statement begins, carried through
edits, watching that text's content hash, scoped to a selected machine of
the team that runs that text or else to the team, greyed while nothing runs
it, kept with the tree; version windows, read-only, one per version of a
file a machine runs or a replay ran, opened from a file window, the
inspector or the stack, and never another team's code; the driver saving
the match's state once per ask, between ticks, when debugging starts and at
every multiple of `debug_checkpoint_ticks`, keeping
`debug_checkpoints_kept` and every state an open fork or listed hit needs;
a hit opening a fork at the tick's earliest hit and, in a live
single-player or PvE match, submitting a pause unless the speed once every
`SetSpeed` in flight lands is already `0`, a replay's playback pausing
instead; a fork opened by hand at the latest tick or a replay's; further
hits listed, the last `debug_hits_kept`, reopening one replacing the open
fork after asking if it is diverged; a version window's breakpoints kept
while its match or replay is open; the debugger window, its stop causes and
the fork banner with its state flags; the map drawing the fork in the
team's view, the mark tools resting and *show the match* restoring them;
the step keys, *run*'s speed and its pause; variable editing outside PvP,
never in step 4 and never of a machine at its own death. Replays: *open a
replay* on the start screen and `--replay <path>`, played on a fork as `06`
rules — sending no command, stepped a tick at a time at the viewer's pace
and paused by the viewer, reporting hits, saving its own states from before
tick 1 and keeping them spread over the replay, halving them and doubling
the interval when full, seeking by restoring one or by running on from
where it stands, without watches, and standing for the match when the
debugger forks from it; the saver's team in the file and its name
`<map>-<tick>-<hash>-<team>.ron`, outside the replay's identity; version
windows for the team's own deployed versions. The `debug_*` constants in
`data/interface.toml`.

**T28 — What-if**

Q55: *try in the fork* on each file window, beside deploy in a match and
alone on a replay, sending the file's deploys into the open fork for its
next tick, at any stop but one in step 4, a bundle that fails to load
refused with its error; *take it* deploying to the match the bundles the
last try sent, offered only while the tree still holds every file of them
as tried; refused in a live PvP match.

[P5](PROBLEMS.md) closes with whichever of T26, T27 and T28 lands last.

**T29 — A paused driver wakes only for a player's command**

Q55 rule 13, amending Q32, in `06`'s Decided entry and The driver. At speed
`0`, `render`'s driver runs to the next tick carrying a player's command,
through a `net` query that skips a scripted team's commands — `06`'s
pseudocode names it `next_tick_with_a_player_command(tick, scripted)`.
`net`'s log records only senders and cannot tell a scripted one; the driver
can, having loaded the scripts, so it passes the query the scripted teams.
Its `local_peers` is no such test: in single-player it also holds every
team neither the player's nor scripted. No replay hash changes — the sim's
ticks are the same. The existing paused test, with the Fool loaded, is
unaffected, since the Fool's script has nothing after tick 0; a new test
loads a script with an entry after the pause and asserts the driver stops
short of it. Fixes [P6](PROBLEMS.md).

## M10 — Power

**T31 — Power in `sim`: the grid, powered machines, wires and the solar panel** `⚠HASH`

Q58, ruled in `02`'s Power section, `03`'s tile and `06`'s tick. The data:
`power_supply` and `power_draw` on every model, the bot's `wire_cost`,
`wire_ticks` and `unwire_ticks`, and a `solar` model with its cost, build
and deconstruct ticks, health and ranges. The tile's per-team `wires`, on
`ground` only, sensed and remembered like paint; the wire plan as a fourth
plan kind in `Mark` and `Unmark`, only on seen ground; the `wire` and
`unwire` actions, taking `wire_cost` from the load on completion. Step 6
as power and printing: each team's grid traced from its printers through
its wires and buildings, sites excluded; supply from buildings on the grid
above zero health; consumers powered oldest first while they fit; an
unpowered machine's main flow and main-flow action held, a printer's print
and a site's construction held, handlers always run; a print begun only
with spare power for its bot. The attributes `powered` and `on_grid`; the
builtins `power()` and `wires()` and every new builtin's `[game]` cost row;
the `solar` deployment and the starter's `robots/solar.py`; the state hash
over the tiles' wires. Tests: a grid cut by unwiring drops a panel's
supply, the newest consumers stop first, a stopped bot's move resumes when
power returns, a `dying` unpowered machine still reaches `death`, and a
printer with no spare power begins no print. The golden fixtures change,
and the PR says why. Fixes [P8](PROBLEMS.md) with [T32](TASKS.md).

**T32 — Power in `render`: wires, the wire tool, dark lights, the panel**

Q58, ruled in `07`'s Decided entry, Machines on the map, the inspector and
Marks. Wires drawn as lines in their team's color joining wired neighbours
and buildings, two teams side by side, the player's own off-grid wires
dimmed; the strip's wire row and `W`, its plan's see-through line and its
ghost only on seen ground; lights dark on an unpowered or off-grid machine;
the solar panel's art, a low tilted plate with a lamp; the inspector's
supply, demand and spare for the player's own team, and the selected
machine's `powered` and `on_grid`. [P8](PROBLEMS.md) closes with whichever
of T31 and T32 lands last.
