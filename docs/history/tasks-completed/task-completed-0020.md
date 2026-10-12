*Closed record — see [../README.md](../README.md). Not spec.*

# T20 — The mark strip, drag marking, the armed tool and the plan preview

Q37 built, as amended by Q44. In `sim`: the snapshot gives each team its
own plans on the tiles that carry any, a read in no hash, and
`Sim::markable` lists the values a Mark of each kind may name — the
building models a plan may name (`printer` and `depot`, Q42), the paint
colors and the overlay labels — which a test holds equal to what
validation accepts.

In `render`: the tools window is the strip, a row per kind from that list,
each ending in *clear*; `B`/`P`/`O` pick a row and `1`–`9` an entry, and
a tool stays armed until `Esc` or a second pick. Armed, a press marks the
tile and a drag every tile the cursor crosses, each once, without
panning; a press with `Shift` held anchors a rectangle that fills on
release (Q44); *clear* withdraws the team's own plan of its kind where one
stands and plans the slot emptied elsewhere (Q44). The player's plans draw
as a translucent version of their effect — the paint's color, the
overlay's marker and label, a ghost building — and a clear plan as the
slot hatched; the armed tool shows the same ghost under the cursor or over
the rectangle. The driver's `snapshot_plan` peek is gone.

Two defects found on the way were fixed with it: flat marks sat inside a
ground or water slab, so neither paint nor a paint plan showed there, and
a machine replayed its last turn every tick it stood still, which read as
spinning. Smoke-tested in a running window. Nothing touched a hash.

Completed in `5445f3b`, the merge of #69; finishing it fixed
[P4](../problems-fixed/problem-fixed-0004.md).
