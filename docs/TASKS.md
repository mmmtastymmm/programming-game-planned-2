# Tasks

What is left to build, grouped into milestones. Completed milestones move to
[history/tasks-completed.md](history/tasks-completed.md).

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md).
- **Decided-but-unbuilt** work — a ruling the code has not caught up to — is
  tracked here *and* as an entry in [PROBLEMS.md](PROBLEMS.md). The register
  owns the gap; this file owns the work.

## M0 — Scaffolding (done except where noted)

- [x] CI harness: `scripts/ci.sh`, GitHub Actions, pre-commit doc gate.
- [x] Doc checks: links, registers, doc layout, mermaid.
- [x] Determinism kit: FNV-1a state hash, named seeded RNG streams, replay
      artifact, golden fixture, cross-process replay check, syntactic scan for
      floats / hash iteration / wall clock.
- [ ] Replace the placeholder sim in `crates/sim` with the real world model.
      ⚠HASH — this regenerates the golden fixture by definition. Blocked on Q5
      and Q9; the shape of `Command` is blocked on Q10 and Q12; hot-swap
      semantics on Q11.

## M1 — First design pass

- [x] Answer the framing questions. Q1–Q4 are ruled; `00-overview.md` is real
      rather than a placeholder and carries its Decided section.
- [x] Answer Q5 (the language). Ruled: our own Python-shaped interpreter. The
      spike behind it is [../spikes/lang-determinism](../spikes/lang-determinism/README.md).
- [x] Answer Q13 (the subset boundary). Broad: procedural Python plus `class`,
      `set`, `match` and `import`; no generators, no reflection.
- [ ] Answer Q14 (the number model), then write `docs/01`. It changes what the
      parser accepts, so it cannot be discovered during implementation.
- [ ] Answer Q6–Q12 and write the numbered docs they unblock.
- [ ] **Q11 must clear variables on hot-swap, or Q13's boundary reopens.** The
      dependency is recorded in both; this line exists so the sequencing is not
      discovered late. Split a doc
      (doorway + parts directory) only once it actually outgrows one file — the
      split has a real cost in cross-part invariants.

## Decided-but-unbuilt

- **The language crate does not exist yet (Q5).** Blocked on Q13 and Q14, so it
  is not yet lag — but it becomes lag the moment those land.
- **Cross-architecture determinism is unverified.** The spike ran every process
  on one arm64 machine, which is not the property lockstep needs. Owning the
  interpreter (Q5) does not grant it; it only means the bug would be ours. The
  cheap fix is a CI job on `ubuntu-latest` comparing against a checked-in hash,
  and the workflow already exists to hang it on.

Note for whoever builds M0's last item: Q3 admits mid-match program updates, so
`Command` **is** an ordered per-tick log — but its principal variant is a program
deploy, not a unit order. The placeholder's `Spawn` / `SetGoal` / `Despawn`
variants command individual units, which Q3 forbids outright; they are
scaffolding, not a model. Not a register entry — the placeholder never claimed to
be the model — but wrong to copy from.

The golden fixture will need to exercise a mid-match redeploy once Q11 lands,
since that is the hash-affecting path most likely to differ between peers.

## M2 — Language implementation (staged; Q13 is the target, not the first ship)

Q13's boundary is materially larger than the procedural core, so the build is
staged. **Staging does not narrow the spec** — `docs/01` specifies all of it —
and this section exists so that the first milestone does not quietly become the
boundary.

- [ ] Lexer with significant indentation (INDENT/DEDENT), then the procedural
      core: functions, control flow, `list`/`dict`/`set`, comprehensions,
      f-strings, chained comparisons, `lambda`.
- [ ] `class` with single inheritance and a closed dunder set. ⚠HASH
- [ ] `match`/`case`. ⚠HASH
- [ ] `import` over a closed module set, program identity as the hash of every
      file's bytes in sorted name order, circular imports rejected at load. ⚠HASH
- [ ] Determinism suite mirroring `crates/sim`'s: golden fixtures for program
      execution, a cross-process check, and the guard the language spike needed —
      **a test that fails to run must not score green.**
