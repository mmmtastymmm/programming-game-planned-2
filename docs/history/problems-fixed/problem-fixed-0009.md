*Closed record — see [../README.md](../README.md). Not spec.*

# P9 — The code lags Q60: nothing shows a replay's version warning

Q60 ruled that a saved replay records the game version that wrote it and
its final state hash, that a replay from another version opens with a
warning, and that a playback whose final hash differs says so; `06` and
`07` state it as current.
[T33](../tasks-completed/task-completed-0033.md) put both fields in
the file, has `render` write them, and has `crates/replay` build the
warning and the report of a differing final hash. Neither is shown,
because the renderer cannot open a replay at all, so there is still no
screen an old replay could warn on. The work left is in
[T27](../tasks-completed/task-completed-0027.md), which opens replays;
this entry closes with the commit that finishes it.

Fixed in `080a8b9`, the merge of #118, which finished T27. The display
itself came with #116, the first of T27's three: a replay opens from the
start screen or `--replay`, its warning is shown as it opens, and a final
hash that differs from the recorded one is reported in the time bar when
the playback reaches the replay's last tick.
