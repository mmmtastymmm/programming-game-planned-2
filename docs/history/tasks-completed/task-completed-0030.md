*Closed record — see [../README.md](../README.md). Not spec.*

# T30 — A `while` header charges `op.statement` on every pass

`01`'s costs part, the operations table's first row, built. In `lang`, the
compiler's `While` arm now starts the loop at the header's own
`Instr::Stmt`, so the back edge and `continue` charge it: a header is
charged each time its condition is tested, once per pass and once more for
the test that ends the loop. The task's "three passes charge it three
times" was loose: the condition is tested four times for three passes, and
"a statement, each time it executes" fits only four, since a loop that
never runs still tests once. The costs row was made exact in the same PR.

A test in `crates/lang/tests/programs.rs` zeroes every cost but
`op.statement` and finds the least budget that reaches a `wait()`: a loop
that never runs costs one more than no loop, three passes six more than
none, and three passes with a `continue` nine more. It fails without the
fix. The golden replay was regenerated, every robot in it looping with
`while`. The language fixtures did not move, their transcripts being blind
to a cost change this small, which is
[I5](../../INBOX.md).

Completed in `9151bc0`, the merge of #88; finishing it fixed
[P7](../problems-fixed/problem-fixed-0007.md).
