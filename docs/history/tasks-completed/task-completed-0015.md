*Closed record — see [../README.md](../README.md). Not spec.*

# T15 — Cross-architecture determinism check in CI

`scripts/determinism-battery.sh` runs every golden gate against its
checked-in fixture and emits each stream's final hash as one line. The
workflow runs it on an x86-64 and an arm64 runner and a third job diffs the
two files; each side checks the fixtures first, so a desync on either
architecture fails before the diff. On the closing run both agreed with each
other and with the arm64 machine that generated the fixtures, so every
stream is pinned on three architectures. `docs/06`'s testing table names
the script and the jobs.

The compare step is a plain `diff` with no seeded mutation; the golden
tests on each side carry the alive assertions. Extending the
check-on-the-checks to the Rust gates stays under T7.

With T15 closed, milestone M3 — determinism assurance — is finished: every
task under it has left the file.

Completed in `eeb3d6d`.
