# Tasks

What is left to build. **This file holds open tasks only** — completing one moves
it to [history/tasks-completed/](history/tasks-completed/README.md) as its own
file, with the commit that finished it. Anything here is open by definition, so
there are no checkboxes to keep in sync.

Numbering is stable — **append new tasks, never renumber**, and never reuse a
number after it moves to history. Entries are written as `**T<n> — <title>**` on
their own line, which is how `scripts/check-registers.mjs` finds them; it rejects
a number that is open and completed at once, and any gap in the sequence.

Milestones below are groupings, not entries. A milestone is finished when every
task under it has left the file.

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md).
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

**T8 — Answer Q14, then write `docs/01`**

Q14 changes what the parser accepts, so it cannot be discovered during
implementation. It is the last thing `docs/01` waits on.

**T9 — Answer Q6–Q12 and write the numbered docs they unblock**

Split a doc (doorway + parts directory) only once it actually outgrows one file —
the split has a real cost in cross-part invariants.

**Q11 must clear variables on hot-swap, or Q13's boundary reopens.** The
dependency is recorded in both rulings; it is repeated here so the sequencing is
not discovered late.

## M2 — Language implementation

Staged deliberately: Q13's boundary is materially larger than the procedural
core. **Staging does not narrow the spec** — `docs/01` specifies all of it — and
this note exists so the first shipped subset does not quietly become the
boundary.

**T10 — Lexer with significant indentation, then the procedural core**

INDENT/DEDENT, then functions, control flow, `list`/`dict`/`set`, comprehensions,
f-strings, chained comparisons and `lambda`.

**T11 — `class` with single inheritance and a closed dunder set** ⚠HASH

**T12 — `match`/`case`** ⚠HASH

**T13 — `import` over a closed module set** ⚠HASH

Program identity becomes the hash of every file's bytes in sorted name order;
circular imports are rejected at load.

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
