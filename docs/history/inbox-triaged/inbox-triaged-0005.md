*Closed record — see [../README.md](../README.md). Not spec.*

# I5 — The language's golden fixtures cannot see a small cost change

T30 made every `while` pass cost one more `op.statement`, and the
`showcase` and `interrupts` fixtures in `crates/lang/tests/golden` did not
move: their transcripts record calls, events and each tick's slice result,
so a cost change shows only if it moves one of those to another tick.
`showcase` has no `while` at all, `interrupts`' loops of 40 passes finish
in the same tick either way, and its 600-pass hook exhausts a fixed hook
budget at the same tick regardless. Only the match replay caught it. A
fixture whose transcript carried each tick's spent budget and deficit, or
the machine's `state_bytes` hash, would see any cost change. Worth deciding
whether the language fixtures are meant to guard costs, since the
determinism rules treat a cost as hash-affecting.

## Outcome

- **Task:** [T34](../../TASKS.md) — the language fixtures record each
  tick's spent budget and deficit, so a cost change moves them, with a
  mutation in `scripts/lib/rust-gates.mjs` that proves the gate sees one.
  The determinism rules treat a cost as hash-affecting, so the gate that
  guards the language should see one.
