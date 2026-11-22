*Closed record — see [../README.md](../README.md). Not spec.*

# T40 — A deficit is carried until it is paid

`01`'s Metering, the tick budget, which the interpreter does not yet
follow for a deficit above one tick's budget
([P10](../problems-fixed/problem-fixed-0010.md), from
[I11](../inbox-triaged/inbox-triaged-0011.md)). When a slice
closes, what the machine owes is what it owed as the slice opened, plus
what the slice spent, less one tick's budget, and never below zero: so a
deficit of several budgets is paid down one budget a tick, by slices
that open at zero and run no operation, and a held slice — a flat bot's,
a dark building's — pays it down the same. A slice a debugger stopped
and resumed still takes the deficit off once. Tests: an operation
costing many budgets sits out that many ticks, less the one it ran in;
a deficit below a budget is carried as before; a `fault` and a
`redeploy` clear none of it. Any fixture that overspends by more than a
budget changes, and the PR says why. Fixes [P10](../problems-fixed/problem-fixed-0010.md).

Completed in `113d1d0`, the merge of #124; the task's own commit is
`2165995`. When a slice closes, the machine owes what it owed as the
slice opened, plus what the slice spent, less one tick's budget, and
never less than nothing; a deficit below a budget is carried exactly as
it was. The task was marked as changing hashes, and it does for any
program with an operation costing more than two budgets, but no stored
fixture has one, so the golden replay and the language fixtures did not
move. Three tests in `crates/lang/tests/programs.rs` hold the rule: a
deficit of many budgets paid down a budget a tick, a redeploy and a
fault clearing none of it, and a slice stopped and resumed at every
boundary paying it down once. It was the one task under M9, the
language as ruled, and fixes
[P10](../problems-fixed/problem-fixed-0010.md).
