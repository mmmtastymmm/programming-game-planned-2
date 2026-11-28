*Closed record — see [../README.md](../README.md). Not spec.*

# T46 — Height and water in `render`

`07`'s Q65 entry, the screen table and Marks. Tiles drawn at their
heights with a cliff's face; water as a see-through sheet at its
surface, on a mesh of its own as `crates/render/CLAUDE.md` requires of a
translucent layer, following the slope of water that is moving; damp
ground as a tint; a spring's and a sink's mark; a bot
standing at its tile's height and stepping between heights, on a ramp
where they differ by a level, at the speed its pace gives, and shown
falling and bonked; the ground
row on `G` with *lower*, *raise*, *level* and its clear, a ground plan
as the tile's outline at the planned height, and its ghost only where
it would change what the team knows; the flooded mark; the inspector's
height, water, a spring's or a sink's rate, and `flooded`. Machines, wires, pylon lines and
marks all sit on the ground at its height.

Completed in `346dd5b`, the merge of #149, in four parts. Tiles at
their heights on a column of earth, and water as a sheet at its
surface, were `b580340`, the merge of #137. A spring's and a sink's
marks came with Q66's water in `dbaf768`, the merge of #138. The ground
row, a ground plan's outline, ramps, the flooded mark, damp ground and
a water sheet tipped to its slope were `eaa4448`, the merge of #143,
which also gave screenshots a camera, `RENDER_SCREENSHOT_LOOK`. Two
things in that part were not what `07` said, were noted as
[I13](../inbox-triaged/inbox-triaged-0013.md), and `07` was changed to
them: a plan to lower a tile draws its outline on the tile, hatched,
since the planned height is under the ground; and a stroke that asks
for the height a tile already has withdraws the plan it holds.

The last part, `fdef03a`, was the bot's own movement, once T43 had
built `03`'s Moving: a move drawn over the whole of its ticks, the ramp
ridden between heights, a drop drawn as a fall out over the edge, a
bonk's lunge and the knock it gives. The maths is plain functions in
`crates/render/src/motion.rs`, read from the two snapshots either side
of a tick with no state kept. The snapshot gained `moving` for it, read
off a machine's action as the snapshot is taken and no part of the
hash; no replay hash moved in any part of this task. Left undrawn: a
bonk of a single tick, which leaves nothing in a snapshot, and any
pitch of the body on a ramp. Ramps on a hill of one-level steps are a
wedge at every step, which is the rule and may be more than is wanted.
