# Tasks

What is left to build. **This file holds open tasks only** — completing one moves
it to [history/tasks-completed/](history/tasks-completed/README.md) as its own
file, with the commit that finished it. Anything here is open by definition, so
there are no checkboxes to keep in sync.

Numbering is stable — **append new tasks, never renumber**, and never reuse a
number after it moves to history. Entries are written as `**T<n> — <title>**` on
their own line, which is how `scripts/check-registers.mjs` finds them; it rejects
a number that is open and completed at once, and any gap *below* the highest
number recorded. Deleting the highest-numbered entry outright shrinks the range
and goes unnoticed — the same honest limit
[history/questions-answered/](history/questions-answered/README.md) states.

Milestones below are groupings, not entries. A milestone is finished when every
task under it has left the file.

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md). **No other register carries it** — CLAUDE.md and
  `.claude/design-invariants.md` define it, and `docs/history/` may narrate it,
  but no entry outside this file is marked. An open question is not work, and
  nearly every open question would qualify while the sim is unbuilt, so a marker
  on all of them selects nothing. Where a *ruling* cannot be discovered during
  implementation, the question says so in words.
- **Decided-but-unbuilt** work — a ruling the code has not caught up to — is
  tracked here *and* as an entry in [PROBLEMS.md](PROBLEMS.md). The register owns
  the gap; this file owns the work. A task is only lag once it is actually
  buildable; before that it is merely pending.

## M0 — Scaffolding

**T7 — Replace the placeholder sim in `crates/sim` with the real world model**

⚠HASH — this regenerates the golden fixture by definition. Blocked on Q9, the
last question `docs/02` waits on; the shape of `Command` is
blocked on Q10 and Q12.

Q3 admits mid-match program updates, so `Command` **is** an ordered per-tick log
— but its principal variant is a program deploy, not a unit order. The
placeholder's `Spawn` / `SetGoal` / `Despawn` variants command individual units,
which Q3 forbids outright. They are scaffolding, not a model: not a register
entry, since the placeholder never claimed to be one, but wrong to copy from.

The golden fixture must exercise a mid-match redeploy. Q11 made it an interrupt
delivered per unit at its next operation boundary, which is the hash-affecting
path most likely to differ between peers.

## M1 — First design pass

**T9 — Answer Q6, Q9, Q10, Q12 and Q15, then write the numbered docs they unblock**

`docs/03` waits on nothing now that Q7 is ruled, and Q7's ruling — held in the
overview's Decided section meanwhile — moves into `docs/02` when that doc is
written.

Split a doc (doorway + parts directory) only once it actually outgrows one file —
the split has a real cost in cross-part invariants.

## M2 — Language implementation

Staged deliberately: Q13's boundary is materially larger than the procedural
core. **Staging does not narrow the spec** — `docs/01` specifies all of it — and
this note exists so the first shipped subset does not quietly become the
boundary.

**T10 — Lexer with significant indentation, then the procedural core** ⚠HASH

INDENT/DEDENT, then functions, control flow, `list`/`dict`/`set`, comprehensions,
f-strings, chained comparisons and `lambda`. The evaluator charges every
operation by its row in `docs/01-language/costs.md`, loading the values from
`data/language/costs.toml`, and every limit from `data/language/limits.toml`
— a row that names a value with no key, or a key with no row, is a load error
in either file, so
the docs and the data cannot drift silently. `num` (Q19) is a scaled i128 —
`docs/01-language/numbers.md` states the scale — so multiply and divide need
a 256-bit intermediate: a
hand-written wide-arithmetic routine, bit-identical on every target, which T14
pins with a fixture at its boundary cases.

**T11 — `class` with single inheritance and a closed dunder set** ⚠HASH

**T12 — `match`/`case`** ⚠HASH

**T13 — `import` over a closed module set** ⚠HASH

Program identity and circular imports follow **determinism rule 7** (CLAUDE.md),
which Q13 extended when it ruled `import` in; this task is what makes the sim
honour it. The rule is not restated here — one canonical statement, per
[design-invariant DI1](../.claude/design-invariants.md).

**T14 — Determinism suite for the language**

Mirroring `crates/sim`'s: golden fixtures for program execution, a cross-process
check, and the guard the language spike needed — **a test that fails to run must
not score green.**

Two fixtures this suite owes that the spec names: the boundary cases of the
256-bit wide-arithmetic routine behind `num` (T10) — the products and
quotients nearest the range, the floor at each sign, the `**` procedure's
count of steps — and the interrupt paths the golden replay is least likely to
cover, a fault escaping mid-unwind and a hook exhausting its budget on its
last operation.

The determinism scan already strips string literals before looking for float
literals, so a `num` literal like `1.5` inside a player-program fixture is
safe in an ordinary Rust string; the one form it cannot strip is a raw string
with hashes (`r##"…"##`), so fixtures avoid that spelling or live outside Rust
source.

## M3 — Determinism assurance

**T15 — Cross-architecture determinism check in CI**

The language spike ran every process on one arm64 machine, which is not the
property lockstep needs. Owning the interpreter (Q5) does not grant it; it only
means the bug would be ours to fix. The workflow already exists to hang this on:
run the battery on `ubuntu-latest` and compare against a checked-in hash.
