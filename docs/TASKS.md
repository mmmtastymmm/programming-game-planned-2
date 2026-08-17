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
- [x] Doc checks: links, register counts, doc layout, mermaid.
- [x] Determinism kit: FNV-1a state hash, named seeded RNG streams, replay
      artifact, golden fixture, cross-process replay check, syntactic scan for
      floats / hash iteration / wall clock.
- [ ] Replace the placeholder sim in `crates/sim` with the real world model.
      ⚠HASH — this regenerates the golden fixture by definition. Blocked on the
      design, which is the point of the next milestone.

## M1 — First design pass

- [ ] Answer the ordering questions in [QUESTIONS.md](QUESTIONS.md) well enough
      to write `docs/00-overview.md` as something other than a placeholder.
- [ ] Write the first numbered docs. Split them (doorway + parts directory) only
      once a file actually outgrows itself — the split has a real cost in
      cross-part invariants.
