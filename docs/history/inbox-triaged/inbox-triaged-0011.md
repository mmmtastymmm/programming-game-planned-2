*Closed record — see [../README.md](../README.md). Not spec.*

# I11 — A deficit larger than one tick's budget is forgiven after one tick

Found while building T39, and not T39's. `01`'s Metering says a deficit
reduces each following tick's budget "by what remains of it until it is
paid". `Machine::close_slice` in `crates/lang/src/vm.rs` sets the new
deficit to `spent - slice_budget`, saturating, and the slice's budget is
the tick budget less the deficit, saturating at zero: so a deficit above
one tick's budget opens a slice of zero, which spends nothing and closes
with a deficit of zero. A program whose one operation costs 3,006 units
against a budget of 200 was seen to carry 2,806, sit out one tick, and run
at full budget on the next, where the doc has it sit out fourteen. A
deficit below the tick budget is carried correctly. Fixing it moves the
golden hashes of any replay that overspends by more than a tick.

The user was asked which way to settle it, the code brought to the rule
or the rule to the code, and chose the rule: a cap of one lost tick on
any overspend would let a program do unbounded work for a fixed price,
which is what the budget exists to refuse.

## Outcome

- **Problem:** [P10](../../PROBLEMS.md) — the interpreter forgives what
  `01`'s Metering says is carried.
- **Task:** [T40](../../TASKS.md) — the deficit carried in full until it
  is paid.
