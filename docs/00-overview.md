# 00 — Overview

A lockstep-multiplayer programming game. The player writes a small number of
programs and a fleet of identical units runs them — and when a program turns out
to be wrong, the player rewrites it while the match is still running. Watching a
plan meet the world, and patching it under fire, is the game.

Unlike the other numbered docs, `00-overview.md` stays a single file — it is
short by construction, and splitting a pitch defeats it. It owns the
pitch-level rulings below and nothing more specific; as `01`–`06` get written,
rulings that belong to them move there.

## Architecture, fixed before the design

Two things were settled before any design question was asked, and neither is
reopened by this corpus:

1. **The game is lockstep multiplayer.** Every peer runs the same simulation and
   exchanges only ordered input. This is the constraint that generates
   CLAUDE.md's determinism rules, and it is expensive to reverse — a sim built
   without it does not become deterministic later.
2. **`sim` is renderer-free plain Rust.** Whatever renders the game reads sim
   state and never writes it.

```mermaid
flowchart LR
  seed["map seed"] --> sim["sim: deterministic world"]
  progs["opening program set"] --> log["ordered command log"]
  edit["mid-match program update"] --> log
  peer["remote peer's updates"] --> log
  log --> sim
  sim --> hash["state hash per tick"]
  sim --> render["renderer: floats fine here"]
  hash --> desync{"hashes agree?"}
  desync -->|no| report["replay: seed + command log"]
```

The diagram is why the crate boundary is worth its inconvenience: the arrow from
`render` back to `sim` does not exist, and any commit that draws one is the
architecture violation to catch in review.

Note the shape of the arrows that *do* reach the sim. Player edits and a remote
peer's edits enter by the same road — the ordered command log — and neither
reaches the world any other way. That single funnel is what makes a mid-match
program update synchronizable at all: an update is agreed for a future tick, so
every peer holds it before that tick runs. There is deliberately no arrow from
the player to an individual unit.

## Decided

Rulings this doc owns. Each is normative — an implementer builds from this
section. The reasoning behind each, and the options rejected, live in
[history/questions-answered/](history/questions-answered/README.md) — one file
per ruling, each carrying the worksheet behind it. Don't read those in a normal
pass.

- **This is a fresh take on the predecessor project's core idea (Q1).** Lockstep
  multiplayer, player-programmed units, deterministic plain-Rust sim. The
  architecture is inherited and already proven in CI; the game around it is
  re-decided, and so is the corpus discipline that holds it.
- **The player programs a fleet of identical units (Q2).** Many units, few
  programs. Programs are addressed to a role or group, never to an individual
  unit, so the language needs no unit-identity concept in its core. Difficulty
  is sourced from interaction between copies rather than from authored puzzles.
- **The player updates programs while the match runs, and never orders an
  individual unit (Q3).** A program update is a lockstep-synchronized command,
  agreed for a future tick so every peer applies it on the same tick. Updates
  are to programs, which by Q2 address a role or group. Whether anything
  *besides* a program update — match control such as resigning — enters the sim
  mid-match is open (Q10); this ruling forbids unit orders, not that. So
  `Command` is a real ordered log whose principal variant is a program deploy, a
  replay is `(seed, timed command log)`, and determinism rule 7 is load-bearing:
  which byte-exact program version a unit runs is part of the state hash.
- **The unit language is ours, Python-shaped, deterministic by construction
  (Q5).** A purpose-built interpreter rather than an embedded runtime — not
  because an embedded one failed, but because owning it means determinism is
  designed in rather than audited for, the cost model is a design lever, and no
  dependency can change evaluation order under a checked-in replay hash. It is
  Python's *surface syntax*, not CPython's semantics.
- **The subset is broad (Q13).** Procedural Python — functions, control flow,
  collections, comprehensions, the expression grammar — plus `class`, `set`,
  `match` and `import`. Excluded: generators (implementation cost); dynamic
  reflection (permanently, since it defeats static analysis and makes the module
  graph dynamic); decorators, `with` and `async`/`await` (grammar for use cases a
  unit program does not have); nested `def`, `global` and `nonlocal`, which
  removes closure capture as a question while leaving `lambda` and methods in a
  `class` body; multiple inheritance; and floats, which Q14 owns and rule 2
  forbids in state-affecting paths anyway. `try`/`except` is **deferred to Q8**,
  not excluded.

  Three additions are only deterministic once we diverge from Python, and these
  are normative: **`set` iterates in insertion order** (and its operators
  preserve that order — without this, `set` is a rule-3 violation wearing
  familiar syntax); **`import` resolves within a closed module set**, which makes
  a program a bundle of named files hashed in sorted name order (determinism rule
  7) with circular imports rejected at load; and **`class` dispatches a closed
  dunder set**, named in `docs/01`, since an open-ended dunder protocol is an
  open-ended determinism surface. `match` needs no divergence: arms are tested
  top to bottom, which is Python's rule already.

  Familiarity is the point of choosing Python at all, so the
  divergence list, not the exclusion list, is what a player has to read. **This
  boundary depends on Q11 clearing all variables on a hot-swap** — if a swap ever
  resumes instead, `class` starts carrying live state across it and the boundary
  reopens. The number model is Q14.
- **PvE ships before PvP (Q4).** Lockstep is built now regardless, since it is
  not retrofittable, so deferring PvP costs nothing architecturally and buys
  slack on balance while the sim changes fastest.

## What the numbered docs will hold

Reserved, not written. The numbering is deliberately sparse so a topic can be
inserted without renumbering:

| Doc | Owns | Blocked on |
|---|---|---|
| `01` | The unit language — syntax, execution model, cost model | Q8, Q11, Q14 |
| `02` | Units — what they are, what they sense, what they do | Q7, Q9 |
| `03` | The world — terrain, resources, whatever the economy turns out to be | Q7 |
| `04` | Opposition — PvE now, PvP later | — |
| `05` | Progression | — |
| `06` | Architecture — crates, tick loop, netcode, testing strategy | Q6, Q10, Q12, Q15 |

Each becomes a doorway plus a parts directory only when it outgrows one file
(CLAUDE.md, *Splitting a doc*).

## Where to go next

[QUESTIONS.md](QUESTIONS.md) holds what is still open — in numeric order, since
numbering is append-only, so it is not a reading order. The table above is the
map from question to doc. **`01` waits on all three of the questions that table
names for it, not on the last one alone** — the single-blocker misreading has
happened, which is why the count is worth repeating even though the numbers
themselves are one section up.
