# 00 — Overview

A lockstep-multiplayer programming game. The player writes a small number of
programs and a fleet of identical machines runs them — and when a program turns out
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
  seed["map (data file)"] --> sim["sim: deterministic world"]
  progs["opening program set"] --> log["ordered command log"]
  edit["mid-match program update"] --> log
  peer["remote peer's updates"] --> log
  log --> sim
  sim --> hash["state hash per tick"]
  sim --> render["renderer: floats fine here"]
  hash --> desync{"hashes agree?"}
  desync -->|no| report["replay: map + command log"]
```

The diagram is why the crate boundary is worth its inconvenience: the arrow from
`render` back to `sim` does not exist, and any commit that draws one is the
architecture violation to catch in review.

Note the shape of the arrows that *do* reach the sim. Player edits and a remote
peer's edits enter by the same road — the ordered command log — and neither
reaches the world any other way. That single funnel is what makes a mid-match
program update synchronizable at all: an update is agreed for a future tick, so
every peer holds it before that tick runs. There is deliberately no arrow from
the player to an individual machine.

## Decided

Rulings this doc owns. Each is normative — an implementer builds from this
section. The reasoning behind each, and the options rejected, live in
[history/questions-answered/](history/questions-answered/README.md) — one file
per ruling, each carrying the worksheet behind it. Don't read those in a normal
pass.

- **This is a fresh take on the predecessor project's core idea (Q1).** Lockstep
  multiplayer, player-programmed machines, deterministic plain-Rust sim. The
  architecture is inherited and already proven in CI; the game around it is
  re-decided, and so is the corpus discipline that holds it.
- **The player programs a fleet of identical machines (Q2).** Many machines, few
  programs. Programs are addressed to a deployment, never to an individual
  machine, so the language needs no machine-identity concept in its core. Difficulty
  is sourced from interaction between copies rather than from authored puzzles.
- **The player updates programs while the match runs, and never orders an
  individual machine (Q3).** A program update is a lockstep-synchronized command,
  agreed for a future tick so every peer applies it on the same tick. Updates
  are to programs, which by Q2 address a deployment. What else enters the
  sim mid-match — marks on the map, the speed, a resignation — is Q10's
  bullet below; this ruling forbids machine orders and nothing else. So
  `Command` is a real ordered log whose principal variant is a program deploy, a
  replay is `(map, timed command log)` — the map being a data file, `docs/03`
  — and determinism rule 7 is load-bearing:
  which byte-exact program version a machine runs is part of the state hash.
- **The language's rulings live in [01-language](01-language.md)** — Q5
  and Q13 (the boundary), Q8, Q11, Q16, Q17, Q18 and Q20 (execution and
  interrupts) and Q14 and Q19 (numbers) are the Decided entries of the parts
  that elaborate them, and they are not repeated here.
- **The machines' rulings live in [02-machines](02-machines.md)** — Q7 and Q21
  (sensing and memory), Q9, Q23, Q24 and Q25 (production, health, the cap and combat), and Q22 (the economy,
  which `docs/03` will cite) are its Decided entries, and they are not
  repeated here.
- **The architecture's rulings live in [06-architecture](06-architecture.md)**
  — Q6 (the speed), Q10 with Q26, Q27 and Q28 (the command log and marks),
  Q12 (scheduling) and Q15 (the renderer) are its Decided entries, and they
  are not repeated here.
- **The opposition's ruling lives in [04-opposition](04-opposition.md)** —
  Q4 (PvE before PvP) is its Decided entry, and it is not repeated here.

## What the numbered docs will hold

`01`, `02`, `03`, `04` and `06` are written; `05` is reserved, not written. The numbering is
deliberately sparse so a topic can be
inserted without renumbering:

| Doc | Owns | Blocked on |
|---|---|---|
| `01` | The language — syntax, execution model, cost model | — |
| `02` | Machines — what they are, what they sense, what they do | — |
| `03` | The world — tiles, terrain, deposits, line of sight, the map | — |
| `04` | Opposition — teams, scripted teams, how a match ends, PvE now, PvP later | Q29 |
| `05` | Progression | — |
| `06` | Architecture — crates, the driver and tick loop, the command log, the snapshot, the state hash, testing strategy | — |

Each becomes a doorway plus a parts directory only when it outgrows one file
(CLAUDE.md, *Splitting a doc*).

## Where to go next

[QUESTIONS.md](QUESTIONS.md) holds what is still open — in numeric order, since
numbering is append-only, so it is not a reading order. The table above is the
map from question to doc. **`01`, `02`, `03`, `04` and `06` are written** —
[01-language](01-language.md), [02-machines](02-machines.md),
[03-world](03-world.md), [04-opposition](04-opposition.md) and
[06-architecture](06-architecture.md) — and the table
above, not this sentence, is the authority on what blocks the rest: earlier
passes misread `01` as having a single blocker while it had three.
