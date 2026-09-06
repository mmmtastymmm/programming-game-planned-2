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

⚠HASH — this regenerates the golden fixture by definition. Blocked on Q9; the
shape of `Command` is blocked on Q10 and Q12.

Q3 admits mid-match program updates, so `Command` **is** an ordered per-tick log
— but its principal variant is a program deploy, not a unit order. The
placeholder's `Spawn` / `SetGoal` / `Despawn` variants command individual units,
which Q3 forbids outright. They are scaffolding, not a model: not a register
entry, since the placeholder never claimed to be one, but wrong to copy from.

The golden fixture must exercise a mid-match redeploy. Q11 made it an interrupt
delivered per unit at its next operation boundary, which is the hash-affecting
path most likely to differ between peers.

## M1 — First design pass

**T8 — Write `docs/01`**

Every question it waited on — Q8, Q11 and Q14, with Q17 amending Q8 — is
answered. Their files under
[history/questions-answered/](history/questions-answered/README.md) are the
source; the doc is what makes them spec.

The things `docs/01` must pin that no open question owns, recorded here so they
are not left to the implementation:

- **Every interpreter limit is spec, not a build default** — recursion depth,
  the per-tick operation budget, the size of every collection, including
  `range`, which Q13 made eager, the pending-interrupt set, which Q8 bounded
  at one entry per kind, and the total operation budget of each hook, which
  Q17 added. The language spike found Rhai's recursion limit
  differing between debug and release builds, a desync produced by a build flag,
  and owning the interpreter (Q5) removes the dependency, not the hazard.
- **`isinstance` is the one permitted type query.** Q13 admits it as a builtin
  and excludes introspection in the same ruling; `docs/01` states the line so
  the two cannot be read against each other.
- **The interrupt mechanism, as Q8, Q11 and Q17 ruled it** — the two modes;
  prologue, player code and unconditional epilogue; the four kinds in priority
  order and which have hooks; the escalation chain fault, dying, death;
  preemption abandoning the preempted handler; delivery points, including the
  boundary before the first operation; coalescing; the halt state; the swap as
  `redeploy`'s epilogue; and the exact shape of the value `on_fault` receives.
  The rules are in those three files; `docs/01` is where they become spec.
- **The number model, as Q14 ruled it** — the scale of `fixed`, stated once as
  spec; the literal grammar and what it rejects; the promotion, floor and
  rounding rules; what `**` accepts; the `str` format of a `fixed`, so printing
  is one string on every peer; and which numeric builtins exist, with a
  specified integer algorithm for any root or other function the game needs.
- **What Q13 named but did not pin** — the closed dunder set that `class`
  dispatches, the module resolution order for `import`, and the exact `match`
  pattern forms supported. Q13's file lists them; without this bullet they
  live only in history.

**T9 — Answer Q6, Q7, Q9, Q10, Q12 and Q15, then write the numbered docs they unblock**

Split a doc (doorway + parts directory) only once it actually outgrows one file —
the split has a real cost in cross-part invariants.

## M2 — Language implementation

Staged deliberately: Q13's boundary is materially larger than the procedural
core. **Staging does not narrow the spec** — `docs/01` specifies all of it — and
this note exists so the first shipped subset does not quietly become the
boundary.

**T10 — Lexer with significant indentation, then the procedural core** ⚠HASH

INDENT/DEDENT, then functions, control flow, `list`/`dict`/`set`, comprehensions,
f-strings, chained comparisons and `lambda`.

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

The determinism scan flags a Rust float literal anywhere on a line, string
literals included, so a player-program fixture containing `1.5` — a `fixed`
literal under Q14 — inside a Rust string will read as a violation once the scan
covers the language crate. The fixtures live outside Rust source, or the scan
learns to skip strings; either way the scan must still catch a real float in
the interpreter.

## M3 — Determinism assurance

**T15 — Cross-architecture determinism check in CI**

The language spike ran every process on one arm64 machine, which is not the
property lockstep needs. Owning the interpreter (Q5) does not grant it; it only
means the bug would be ours to fix. The workflow already exists to hang this on:
run the battery on `ubuntu-latest` and compare against a checked-in hash.
