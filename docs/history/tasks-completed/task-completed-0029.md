*Closed record — see [../README.md](../README.md). Not spec.*

# T29 — A paused driver wakes only for a player's command

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
short of it. Fixes [P6](../problems-fixed/problem-fixed-0006.md).

Completed in `120e793`, the merge of #105: `net`'s query takes the scripted
teams, the driver keeps the teams it loaded scripts for and passes them,
and the new test's script plans a depot at tick 30 that a pause at tick 15
does not run to, while a player's command at tick 18 is run to exactly.
