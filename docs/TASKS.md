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

**T7 — Replace the placeholder sim in `crates/sim` with the real world model, and split out `lang`, `net` and `replay`**

⚠HASH — this regenerates the golden fixture by definition. The world model
has its rulings — Q7, Q9, Q21 through Q25, Q30 and Q31 — in `docs/02` and its
world in `docs/03`, so it is buildable, and `docs/06` fixes the tick's seven steps,
the command's fields, the snapshot's parts and the state hash's coverage and
order. The crate split `06` draws — `lang`, `sim`, `net`, `replay` — is this
task's too; `render` is T16.

Q3 admits mid-match program updates, so `Command` **is** an ordered per-tick log
— but its principal variant is a program deploy, not a machine order. The
placeholder's `Spawn` / `SetGoal` / `Despawn` variants command individual machines,
which Q3 forbids outright. They are scaffolding, not a model: not a register
entry, since the placeholder never claimed to be one, but wrong to copy from.

The golden fixture must exercise a mid-match redeploy. Q11 made it an interrupt
delivered per machine at its next operation boundary, which is the hash-affecting
path most likely to differ between peers.

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

*Progress (2026-09-09):* `crates/lang` exists — lexer, parser, compiler to a
resumable bytecode, the metered evaluator, `num` with its wide routine, the
language builtins and methods, the cost and limit tables with the row↔key
check in both directions, and a program-level test suite plus a cross-process
trace check. Game builtins resolve by their `[game]` row and go to a `Host`
trait the sim will implement (T7). Hook budgets and the interrupt kinds
landed with T14. (`key=` taking only a builtin was a gap here until T11's
dispatcher let a builtin re-enter a `def`.)

*Progress (2026-09-10):* the two remaining gaps closed — source nesting
past `depth.nesting` is a parse error at load, for brackets and blocks
alike, and the live-values limit is measured exactly, walked only when a
running bound on growth says the limit could have been crossed. Nothing
remains open under this task.

## M3 — Determinism assurance

**T15 — Cross-architecture determinism check in CI**

The language spike ran every process on one arm64 host, which is not the
property lockstep needs. Owning the interpreter (Q5) does not grant it; it only
means the bug would be ours to fix. The workflow already exists to hang this on:
run the battery on `ubuntu-latest` and compare against a checked-in hash.

## M4 — Renderer

**T16 — The `render` crate: Bevy, the driver, the snapshot, the mark tools**

Q15's shape: a crate depending on `sim` and Bevy, never the reverse; the
driver as a Bevy system owning the sim as a resource, applying the speed
(Q6) and the peer wait (Q12); every other system reading the completed-tick
snapshot `docs/06` defines; interpolation in floats, never read back; input
becoming `Deploy`, `Mark`, `Unmark`, `SetSpeed` and `Resign` commands (Q10)
that the renderer submits and forgets. The review rule — flag any arrow from
`render` to `sim` — starts applying with this crate's first commit.
