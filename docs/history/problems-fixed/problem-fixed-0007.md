*Closed record — see [../README.md](../README.md). Not spec.*

# P7 — A `while` header is charged once per loop, not once per pass

`01`'s costs part charges a statement each time it executes, a `while`
header once per pass, and `06`'s Forks has a `while` begin again on each
pass for that reason. The compiler charged the header's `op.statement`
once, before the loop's start (`crates/lang/src/compile.rs`, the `While`
arm), so a loop of a hundred passes paid for its header once, and every
program with a `while` loop ran cheaper than ruled — the starter's and the
Fool's robots among them. Found in review of Q55, whose statement rule
would otherwise have been built on the lagging charge. The work was
[T30](../tasks-completed/task-completed-0030.md).

Fixed in `9151bc0`, the merge of #88, which finished T30: the loop starts
at the header's statement, so it is charged each time its condition is
tested.
