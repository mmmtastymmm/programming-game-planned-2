*Closed record — see [../README.md](../README.md). Not spec.*

# T14 — Determinism suite for the language

`crates/lang/tests/golden.rs` mirrors the sim's gate: stored fixtures with
their transcripts and per-tick hashes, regenerated only by
`UPDATE_GOLDEN=1` with the PR saying why; a cross-process comparison; and
an alive assertion per fixture so a test that fails to run cannot score
green. Three fixtures: `showcase`, a two-file program across the language;
`interrupts`, two scripted scenarios over one program; `wide`, the 256-bit
routine behind `num` at its boundaries, `**` step counts included.

The fixture the task named — a hook exhausting its budget on its last
operation — needed the interrupt model of `docs/01-language/execution.md`,
so it came with the task: hooks bound at load, hook budgets, delivery by
priority, escalation, abandonment mid-unwind, the fault record with the
exhausted kind and the bundle version. The sim (T7) raises the world kinds
through `Machine::raise` and reads `Machine::take_events`.

The wide fixture earned its bytes on its first run: `round` and `:.Nf` at
the range edge wrapped instead of raising, and `%` raised whenever `//` of
the same operands was out of range. Both fixed in the same commit.

Completed in `f3a3f78`.
