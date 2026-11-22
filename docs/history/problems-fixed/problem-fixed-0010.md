*Closed record — see [../README.md](../README.md). Not spec.*

# P10 — A deficit above one tick's budget is forgiven after a tick

`01`'s Metering has one statement of the rule: an operation that
completes past the tick budget leaves a deficit, each following tick's
budget is reduced by what remains of it until it is paid, and nothing
clears it, "so a large traversal costs its full price however the ticks
fall". The interpreter carries a deficit below one tick's budget
correctly and forgives the rest of a larger one: `Machine::close_slice`
(`crates/lang/src/vm.rs`) sets the new deficit to what the slice spent
past the budget it opened with, and a slice that opens at zero spends
nothing, so it closes owing nothing. A program whose one operation cost
3,006 units against a budget of 200 sat out one tick, where the rule has
it sit out fourteen. Every program with an operation that large runs
cheaper than ruled.
From [I11](../inbox-triaged/inbox-triaged-0011.md). The work was
[T40](../tasks-completed/task-completed-0040.md).

Fixed in `113d1d0`, the merge of #124, which finished T40: a slice that
closes leaves the machine owing what it owed, plus what the slice spent,
less one tick's budget, so a deficit of several budgets is paid down a
budget a tick and nothing clears it.
