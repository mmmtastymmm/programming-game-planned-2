*Closed record — see [../README.md](../README.md). Not spec.*

# T21 — The stall banner with resign, and the desync freeze

Q38 built. The driver's reports became structured: a stall names the
tick, the peers awaited and the seconds waited; a desync names the tick
the peers disagree on, this peer's hash for it when the disagreement is
over a hash, and the report. The driver also says which tick a
submission made now is agreed for, and submits on it.

In `render`: past `stall_report_ticks` an amber banner over the map names
the peers, the tick and the wait, with a resign and the tick it lands on,
then where it landed once pressed; nothing else changes. A desync no
longer goes straight to the end screen: the match freezes under a red
banner with the report and the replay's path, the replay written at once,
and *leave* goes on to the end screen. The time bar keeps a short word for
each.

A desync's replay is named for, and runs to, the tick the peers disagree
on with this peer's hash there, since the driver may have run past that
tick before the other's hash arrived. Checked across two real processes:
a frozen joiner stalled the host, and a joiner lying about its hash froze
the host on the lying tick, its replay running headless to that hash.
Nothing touched a hash.

Completed in `d6dee23`.
