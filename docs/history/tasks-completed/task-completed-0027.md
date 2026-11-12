*Closed record — see [../README.md](../README.md). Not spec.*

# T27 — The debugger in `render`, and replays on the start screen

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
`<map>-<tick>-<hash>-<team>.ron`, outside the replay's identity; the
warning `crates/replay` builds for a replay whose version is not this
build's or that records none, shown when it opens, and its report of a
final hash that differs from the recorded one, shown in the time bar when
the playback reaches the replay's last tick (Q60,
[T33](task-completed-0033.md)); version
windows for the team's own deployed versions. The `debug_*` constants in
`data/interface.toml`.

Completed in `080a8b9`, the merge of #118, the last of three. #116 brought
replays: *open a replay* and `--replay`, the playback on a fork, the
saver's team in the file and its name, and the version warning and the
differing final hash shown. #117 brought the debugger's core in the
driver: the watches, the saved states, a hit's fork and pause, the further
hits, and the fork's steps, run, back, selection, inspection and edits.
#118 brought the interface: gutter breakpoints and their scope, carried
through edits and renames and kept with the tree in `windows.toml`,
version windows, the debugger window, the fork banner, the map of a fork
and the step keys, `F4` to `F9`. Two things went into `sim` on the way,
neither part of the state hash: a machine's snapshot names the version of
the bundle it runs, and a fork says whether it would take an edit. No
replay hash moved. The interface's logic is tested headless; its drawing
was not looked at on a screen before the merge, the build machine's
display being asleep. Fixes [P9](../problems-fixed/problem-fixed-0009.md).
What-if is [T28](../../TASKS.md), and [P5](../../PROBLEMS.md) stays open
against it.
