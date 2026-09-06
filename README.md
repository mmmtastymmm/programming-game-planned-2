# programming-game-planned-2

A lockstep-multiplayer programming game, in the design stage. The design docs
live in [docs/](docs/); start with [docs/00-overview.md](docs/00-overview.md)
and [docs/QUESTIONS.md](docs/QUESTIONS.md).

The corpus scaffolding and the determinism gate are ported from the predecessor
project (`../programming_game_planned`); [CLAUDE.md](CLAUDE.md) documents both
and is worth reading before the first commit.

## Getting set up

```sh
scripts/install-hooks.sh    # once per clone: pre-commit gate
scripts/ci.sh               # everything CI runs
scripts/ci.sh docs          # just the fast checks — seconds, no Rust build
scripts/ci.sh rust          # fmt, clippy, tests
```

Node 20+ is needed for the `check-*.mjs` checks; the Rust toolchain is pinned
in `rust-toolchain.toml`.

## Layout

| Path | What |
|---|---|
| `docs/` | The design corpus, plus the four registers — questions, problems, tasks, inbox |
| `crates/sim/` | Deterministic world simulation. Placeholder model, real determinism gate |
| `scripts/` | CI entry point, the eight checks (`check-*.mjs`) and the file-size gate |
| `docs/history/` | Closed records: one file per answered question, fixed problem, completed task, triaged note |
| `spikes/` | Standalone experiments answering a design question; not part of the build |
| `.githooks/` | Pre-commit gate: file size, then the fast checks. Installed via `scripts/install-hooks.sh` |
| `.claude/` | [design-invariants.md](.claude/design-invariants.md) — properties the corpus must have, and how to check each |
