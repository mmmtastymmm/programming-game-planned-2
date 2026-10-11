*Closed record — see [../README.md](../README.md). Not spec.*

# P4 — The code lags Q37: the tools are a list, not the mark strip

Q37 ruled the mark tools a strip of three rows from data — building
models, paint colors, overlay labels — each ending in a clear, picked with
`B`/`P`/`O` and `1`–`9`, armed until `Esc`, with drag marking and a plan
preview; `07` states it as current. The renderer still offers a short
fixed list of tools on keys `1`–`6`, marks one tile per click, and draws no
preview. The work is
[T20](../tasks-completed/task-completed-0020.md), open since before Q42;
this entry records the gap the registers require beside it, and closes
with the commit that finishes T20.

Fixed in `5445f3b`, the merge of #69, which finished T20: the tools
window is the strip, a drag marks every tile it crosses, and plans
preview on the map, as Q37 amended by Q44 rules.
