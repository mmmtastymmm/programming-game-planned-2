*Closed record — see [../README.md](../README.md). Not spec.*

# T34 — The language fixtures see every cost change

From [I5](../inbox-triaged/inbox-triaged-0005.md). The
`crates/lang/tests/golden` transcripts record calls, events and each
tick's slice result, so a cost change that moves nothing to another tick
— T30's extra `op.statement` per `while` pass — leaves them unchanged.
Each transcript gains, per tick, the budget the slice spent and the
deficit it left, so any cost change moves the transcript and its hash.
The fixtures' hashes change once, and the PR says why;
`scripts/lib/rust-gates.mjs` gains a mutation that changes one cost
constant and asserts the language gate goes red.

Completed in `eb491e9`, the merge of #107: every tick line of `showcase`
and `interrupts` carries `spent=` and `deficit=`, read from two new
accessors on the machine; the two fixtures were regenerated once for the
new fields and nothing else in them moved; and the gates check seeds
`op.statement = 2` into `data/language/costs.toml` and asserts the
language gate reports the drift.
