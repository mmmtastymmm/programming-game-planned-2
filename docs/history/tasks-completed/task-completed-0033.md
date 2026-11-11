*Closed record — see [../README.md](../README.md). Not spec.*

# T33 — A replay records the version that wrote it

Q60, ruled in `07`'s Q36 entry and The match and `06`'s command log. In
`crates/replay`'s RON, the game's version string and the final state hash
beside the saver's team, outside the replay's identity; `render` writes
both when a match ends or desyncs. Opening a replay whose version is not
this build's, or that records none, shows a warning naming the version
that wrote it and plays anyway; when the playback reaches the replay's
last tick, a state hash that differs from the recorded one is reported in
the time bar. Tests: a replay round-trips its version and final hash, and
a replay whose recorded final hash is altered reports the difference.
Fixes [P9](../../PROBLEMS.md).

Completed in `dd405d6`, the merge of #109, short of the two things it
says are shown. The replay's RON holds the version and the final hash as
optional fields, so a file older than Q60 still parses and the golden
replay's stored file did not change; `render` writes both into every
replay it saves; and `crates/replay` builds the warning, for a version
that is not this build's or is absent, and the report of a final hash
that differs from the recorded one, each with the tests the task names.
Showing the warning on opening and the report in the time bar had no
place to land: nothing in `render` opens a replay until
[T27](../../TASKS.md), which needs T26's forks. That display moved into
T27's text, and P9 stays open against T27 rather than closing here.
