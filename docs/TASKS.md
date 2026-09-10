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

**T7 — Replace the placeholder sim in `crates/sim` with the real world model, and split out `lang`, `net` and `replay`**

⚠HASH — this regenerates the golden fixture by definition. The world model
has its rulings — Q7, Q9, Q21 through Q25, Q30 and Q31 — in `docs/02` and its
world in `docs/03`, so it is buildable, and `docs/06` fixes the tick's seven steps,
the command's fields, the snapshot's parts and the state hash's coverage and
order. The crate split `06` draws — `lang`, `sim`, `net`, `replay` — is this
task's too; `render` was T16, now closed.

Q3 admits mid-match program updates, so `Command` **is** an ordered per-tick log
— but its principal variant is a program deploy, not a machine order. The
placeholder's `Spawn` / `SetGoal` / `Despawn` variants command individual machines,
which Q3 forbids outright. They are scaffolding, not a model: not a register
entry, since the placeholder never claimed to be one, but wrong to copy from.

The golden fixture must exercise a mid-match redeploy. Q11 made it an interrupt
delivered per machine at its next operation boundary, which is the hash-affecting
path most likely to differ between peers.

*Progress (2026-09-10):* the real world model is in `crates/sim` — the
tables, the map with its validity rules, tiles and deposits, machines with
their actions, senses and memory, the eight-step tick, the state hash in
`docs/06`'s order, the snapshot, and scripted teams — with the game builtins
as the language's host. `crates/net` holds the command log and its bytes;
`crates/replay` the headless driver and the golden replay: the first map,
a player against the Fool, a mid-match redeploy, a speed change and marks.
The Fool's red bundle gained a step off a tile it stands on, since a bot
builds and picks beside a tile, never on it. **Still open under this
task:** the peer exchange itself, which lives with the renderer's driver
(closed as T16) since headless has one peer; and `docs/06`'s check-on-the-checks
extended to the Rust gates.
