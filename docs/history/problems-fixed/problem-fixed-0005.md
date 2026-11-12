*Closed record — see [../README.md](../README.md). Not spec.*

# P5 — The code lags Q55: no forks, breakpoints or what-if

Q55 ruled that a program is debugged on a fork, and `06`'s Forks section
and `07`'s Debugging section and The match state it as current: a saved
state of the match, watched lines passed to `sim.step`, forks that stop
mid-tick, step, save and restore, editable variables, what-if, and
replays opened from the start screen, recording their team.
[T26](../tasks-completed/task-completed-0026.md) built the `sim`
half: the saved state, the watches, the fork and everything it does, and
a replay's playback on one.
[T27](../tasks-completed/task-completed-0027.md) built the
renderer's half but what-if: replays opened from the start screen and
recording their team, breakpoints, version windows, the debugger window
and its steps, and editable variables. Nothing sends a try into a fork
or takes one to the match. The work left is
[T28](../tasks-completed/task-completed-0028.md); this entry closes
with the commit that finishes it.

Fixed in `07dafa5`, the merge of #120, which finished T28: a file window
sends a try into the open fork and takes one to the match. With T26 and
T27 before it, everything Q55 ruled is built.
