*Closed record — see [../README.md](../README.md). Not spec.*

# T44 — Sight over height in `sim`

`03`'s Line of sight, as Q65 extends it and Q68 amends it. A tile on
the ray blocks when it is higher than the seer's ground and stands
above the straight line between that ground and the seen tile's, by
the integer test `03` gives; a machine's range to a
tile is its vision plus its height less the tile's, nothing below
zero; `attack` gains the blocking and keeps its range; hearing is
unchanged. A change of height, or a mined rock, is a change to what a
team sees, so it bumps the count the sight cache holds (T38), and the
gates seed one that forgets. Tests: a ridge between two machines, a
machine on a hill and one in a pit, level ground as before, and the
walk from each end, since the ray is not symmetric.

Completed in `8100872`, the merge of #145; the task's own commit is
`2ad66b6`. `sense::clear` tests each tile of the ray by `03`'s
integers, and range is the seer's vision plus its height less the seen
tile's; `attack` took the blocking through the same function with no
change of its own, and hearing is held unchanged by a test. Built
first to Q65's letter, the rule hid the bed of a lake or a river from
everything not on its bank — on `falls` each printer lost the river in
front of it — and the user amended it in the same merge as
[Q68](../questions-answered/question-answered-0068.md): ground no
higher than the seer never blocks. `World::set_height` became the one
place a height changes, and it and a mine's landing tell the sight
cache through `ground_changed`. Both already bumped the count; what
was missing was a way to catch one that forgot, since a dig lands in a
tick in which other bots move and bump it anyway. Debug builds now
keep each tile's terrain and height as they are told of them, and the
Rust gates seed a `set_height` that does not tell. Eleven tests in
`crates/sim/tests/sight.rs`. Every hash of the golden replay moved,
because range now counts height and the player's printer sees the
lake's bed from its first tick; the scenario did not change. The
scaling benchmark's step was the same within noise before and after.
