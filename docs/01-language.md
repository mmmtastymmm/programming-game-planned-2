# 01 — The unit language

The language every unit program is written in: a purpose-built interpreter for
a Python-shaped language, deterministic by construction, whose surface a Python
programmer recognises on sight and whose semantics are entirely this document's.
This is what the language crate is built from. It is normative: two
implementers reading it must produce the same bit-for-bit behavior on every
tick, because a difference is a desync (design-invariant DI7).

This doc is split Rust-module style: this file is the doorway and holds only
what crosses its parts; each part owns its subject in full, and each part that
elaborates a ruling carries it in its own **Decided** section. The reasoning
behind those
rulings lives one file per ruling under
[history/questions-answered/](history/questions-answered/README.md) and is not
repeated here.

## Parts

| Part | Owns |
|---|---|
| [syntax](01-language/syntax.md) | The program bundle and its identity; lexical rules; the boundary — every statement, expression, type, builtin and method that exists; classes and the closed dunder set; `match` patterns; `import` and module resolution; exceptions; the complete divergence list |
| [execution](01-language/execution.md) | A unit's program lifecycle; metering and the cost model; the interrupt model — kinds, handlers, hooks, delivery, preemption, escalation; every limit |
| [numbers](01-language/numbers.md) | `num`, the one number type; literals; every operator's rounding; conversions; the numeric builtins; the faults arithmetic can raise |
| [costs](01-language/costs.md) | One row per operation, builtin and method with its cost formula over named constants; the shape of a game builtin's row; the values live in `data/language/costs.toml` |

## Invariants that cross the parts

1. **Nothing is inherited from the host.** Every evaluation order, iteration
   order, rounding, limit and failure is stated in these parts. Where a part is
   silent, the behavior is Python 3.12's — and a difference from it that the
   [divergence list](01-language/syntax.md#divergences-from-python-in-full)
   does not name is a defect in this doc, not a question: it is filed in
   [PROBLEMS.md](PROBLEMS.md) and the list is amended.
2. **Every failure is a fault.** Anything the language cannot do — an
   unrepresentable number, an exhausted limit, a name that does not exist —
   raises an exception, and an exception nothing catches is a `fault` interrupt
   located at the operation that raised it — or, if an interrupt cuts off its
   unwinding, a fault record written as if it had escaped. Nothing is silent,
   nothing wraps, nothing is undefined behavior.
3. **Nothing resumes across an interrupt.** A `fault` restarts main flow, a
   `redeploy` swaps it, `dying` and `death` end it. The only resumption the
   language has is metering: main flow or a hook paused at an operation
   boundary when the tick's budget runs out continues from that boundary next
   tick, in the same program, with the same state.
4. **Every limit and every cost is spec, and its value is data.** Each limit —
   call depth, budgets, collection sizes — is named in
   [execution](01-language/execution.md) with what exhausting it does; each
   cost has a row in [costs](01-language/costs.md) saying what it depends on.
   The numbers are tuning constants that live in data files —
   `data/language/costs.toml` and `data/language/limits.toml` — and are
   hash-affecting like every tuning constant. The scale of
   `num` is the one number that is spec rather than tuning: it decides which
   literals parse, so it is stated once in [numbers](01-language/numbers.md)
   and changes only under a new question number.
5. **A program is its bytes.** A bundle of named files, byte-exact and never
   normalised, identified by the hash [syntax](01-language/syntax.md#the-bundle)
   defines (determinism rule 7). Which version each unit is running is state,
   and the state hash sees it.
6. **Every rule here is hash-affecting.** Changing any of them changes the
   golden-replay fixtures, and a PR that does so says why (CLAUDE.md).

Where this doc stops, each part says so at the point of deferral — naming the
doc that owns the rest, or the open question it waits on — and the overview's
table is the map from question to doc.
