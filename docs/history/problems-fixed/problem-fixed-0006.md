*Closed record — see [../README.md](../README.md). Not spec.*

# P6 — A pause runs on to a scripted team's next command

Q55 amended Q32 so a paused driver runs only to the next tick carrying a
player's command, a scripted team's not counting. `render`'s driver still
took the next tick carrying any command, through `net`'s
`next_tick_with_a_command`, and the driver puts a scripted team's whole
script into the log when it is built, a script being a command log written
in advance (`04`). So a pause in PvE, a player's or a debugger's, ran on
without the clock to the script's next entry. It was latent only because
the Fool's script has nothing after tick 0. The work was
[T29](../tasks-completed/task-completed-0029.md).

Fixed in `120e793`, the merge of #105, which finished T29: the query is
`next_tick_with_a_player_command`, taking the scripted teams and skipping
their commands, and a test with a script entry after the pause holds the
driver short of it.
