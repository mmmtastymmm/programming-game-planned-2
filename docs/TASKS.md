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
shape of `Command` is blocked on Q10 and Q12; hot-swap semantics on Q11.

Q3 admits mid-match program updates, so `Command` **is** an ordered per-tick log
— but its principal variant is a program deploy, not a unit order. The
placeholder's `Spawn` / `SetGoal` / `Despawn` variants command individual units,
which Q3 forbids outright. They are scaffolding, not a model: not a register
entry, since the placeholder never claimed to be one, but wrong to copy from.

The golden fixture will need to exercise a mid-match redeploy once Q11 lands,
that being the hash-affecting path most likely to differ between peers.

## M1 — First design pass

**T8 — Answer Q11 and Q14, then write `docs/01`**

Neither can be discovered during implementation; the reason for each is with
the question, in [QUESTIONS.md](QUESTIONS.md), and is deliberately not repeated
here.

Q11 falls inside T9's sweep, so that much of T9 lands first — the sequencing is
recorded here because reading T8 alone once suggested Q14 was the only thing in
the way.

Three things `docs/01` must pin that no open question owns, recorded here so
they are not left to the implementation:

- **Every interpreter limit is spec, not a build default** — recursion depth,
  the per-tick operation budget, the size of every collection, including
  `range`, which Q13 made eager, and the pending-interrupt set, which Q8 bounded
  at one entry per kind. The language spike found Rhai's recursion limit
  differing between debug and release builds, a desync produced by a build flag,
  and owning the interpreter (Q5) removes the dependency, not the hazard.
- **`isinstance` is the one permitted type query.** Q13 admits it as a builtin
  and excludes introspection in the same ruling; `docs/01` states the line so
  the two cannot be read against each other.
- **The interrupt mechanism, as Q8 ruled it** — the two modes; prologue, player
  code and unconditional epilogue; the priority order over the closed kind set;
  preemption abandoning the preempted handler; delivery points; coalescing; the
  halt state; and the exact shape of the value `on_fault` receives. The rules
  are in Q8's file; `docs/01` is where they become spec.

**T9 — Answer Q6, Q7, Q9–Q12 and Q15, then write the numbered docs they unblock**

Split a doc (doorway + parts directory) only once it actually outgrows one file —
the split has a real cost in cross-part invariants.

**Q11 and Q13's boundary are coupled**, so answer Q11 before M2 leans on that
boundary. [QUESTIONS.md](QUESTIONS.md) states the dependency and Q13's ruling
records it; what belongs here is only the sequencing, which is easy to discover
late.

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

## M3 — Determinism assurance

**T15 — Cross-architecture determinism check in CI**

The language spike ran every process on one arm64 machine, which is not the
property lockstep needs. Owning the interpreter (Q5) does not grant it; it only
means the bug would be ours to fix. The workflow already exists to hang this on:
run the battery on `ubuntu-latest` and compare against a checked-in hash.
