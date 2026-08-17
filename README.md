# programming-game-planned-2

A lockstep-multiplayer programming game, in the design stage. The design docs
live in [docs/](docs/); start with [docs/00-overview.md](docs/00-overview.md)
and [docs/QUESTIONS.md](docs/QUESTIONS.md).

The corpus scaffolding and the determinism gate are ported from the predecessor
project (`../programming_game_planned`); [CLAUDE.md](CLAUDE.md) documents both
and is worth reading before the first commit.

## Getting set up

```sh
scripts/install-hooks.sh    # once per clone: pre-commit doc gate
scripts/ci.sh               # everything CI runs
scripts/ci.sh docs          # just the doc checks — seconds, no Rust build
scripts/ci.sh rust          # fmt, clippy, tests
```

Node 20+ is needed for the doc checks; the Rust toolchain is pinned in
`rust-toolchain.toml`.

## Layout

| Path | What |
|---|---|
| `docs/` | The design corpus, plus the question / problem / task registers |
| `crates/sim/` | Deterministic world simulation. Placeholder model, real determinism gate |
| `scripts/` | CI entry point and the four doc checks |
| `.githooks/` | Pre-commit doc gate, installed via `scripts/install-hooks.sh` |
