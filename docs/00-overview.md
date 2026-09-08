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
  (sensing and memory), Q9, Q23 and Q24 (production, health and the cap), and Q22 (the economy,
  which `docs/03` will cite) are its Decided entries, and they are not
  repeated here.
- **The sim has no rate; the players choose the speed together (Q6).** A
  tick is the unit of simulated time and every duration is a count of ticks.
  How many ticks pass per real second is a `SetSpeed` command in the log,
  agreed for a tick like a deploy, applied by each peer's driver and never
  read by the sim, from a closed list of steps in data with `0` as pause;
  the map sets the starting speed. The renderer interpolates between
  completed ticks. This ruling moves to `docs/06` when it is written.
- **The player may mark the map, and everything the player does enters one
  ordered command log (Q10).** Besides `Deploy`, the log carries `Mark` and
  `Unmark` — a blueprint (a building model, which `build` may target and
  `blueprints()` returns), a paint (a deployment color), or a layer (a label
  from a closed list) on a tile, per team, visible to that team only, read by
  programs and never sensed or remembered — `SetSpeed` (Q6) and `Resign`,
  which removes the sender's team on the agreed tick. A mark changes what a
  program can read about a tile and never what a machine does: the player
  authors programs and marks the map, and never commands a machine. Every
  command carries its sender and its tick; one that cannot apply is dropped,
  except a deploy to a locked deployment, which waits. This ruling moves to
  `docs/06` when it is written.
- **A fixed delay, and a stall for a late peer (Q12).** A command entered at
  tick `t` is agreed for `t + delay`, a tuning constant in ticks fixed at
  match start; a peer missing another's command set for the next tick issues
  no tick until it arrives, so nothing is dropped and nothing desyncs.
  Single-player is lockstep with one peer and feels the same delay. Within a
  tick, commands apply by sender then submission order. Rate-limiting is
  deferred with PvP; the sender stamp is its hook. This ruling moves to
  `docs/06` when it is written.
- **Bevy renders, in its own crate, in the sim's process, reading only
  completed-tick snapshots and writing only to the command log (Q15).** A
  `render` crate depends on `sim` and Bevy; `sim` depends on neither. The
  driver — wall clock, speed, peer wait, next tick — is a Bevy system that
  owns the sim as a resource; every other system sees only the snapshot the
  sim publishes after each tick, from which the renderer's own entities are
  built. Interpolation is in floats and never read back. Input becomes
  commands the renderer submits and forgets. Headless — `sim` plus a driver
  with no Bevy — stays the build CI runs. This ruling moves to `docs/06`
  when it is written.
- **PvE ships before PvP (Q4).** Lockstep is built now regardless, since it is
  not retrofittable, so deferring PvP costs nothing architecturally and buys
  slack on balance while the sim changes fastest.

## What the numbered docs will hold

`01`, `02` and `03` are written; the rest are reserved, not written. The numbering is
deliberately sparse so a topic can be
inserted without renumbering:

| Doc | Owns | Blocked on |
|---|---|---|
| `01` | The language — syntax, execution model, cost model | — |
| `02` | Machines — what they are, what they sense, what they do | — |
| `03` | The world — tiles, terrain, deposits, line of sight, the map | — |
| `04` | Opposition — PvE now, PvP later | Q25 |
| `05` | Progression | — |
| `06` | Architecture — crates, the driver and tick loop, the command log, the snapshot, testing strategy | — |

Each becomes a doorway plus a parts directory only when it outgrows one file
(CLAUDE.md, *Splitting a doc*).

## Where to go next

[QUESTIONS.md](QUESTIONS.md) holds what is still open — in numeric order, since
numbering is append-only, so it is not a reading order. The table above is the
map from question to doc. **`01`, `02` and `03` are written** —
[01-language](01-language.md), [02-machines](02-machines.md) and
[03-world](03-world.md) — and the table
above, not this sentence, is the authority on what blocks the rest: earlier
passes misread `01` as having a single blocker while it had three.
