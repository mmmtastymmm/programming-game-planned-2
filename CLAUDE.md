# Programming Game 2 (working title)

A lockstep-multiplayer programming game: the player writes a small number of
programs, a fleet of identical units runs them, and **the player rewrites those
programs while the match is running** — each update a lockstep-synchronized
command applied on the same tick by every peer (Q3). The player never commands an
individual unit. The simulation runs identically on every machine.

The design is in its first pass. The framing rulings and the language decision
are made and live in [docs/00-overview.md](docs/00-overview.md)'s Decided
section; the tick, sensing, fault behavior, unit production and the language's
own subset boundary are still open in [docs/QUESTIONS.md](docs/QUESTIONS.md). The corpus scaffolding and the
determinism gate are ported from the predecessor project
`../programming_game_planned` and proven in CI.

Design lives in `docs/`; unresolved design questions in
[docs/QUESTIONS.md](docs/QUESTIONS.md).

Crate layout: `crates/sim` (deterministic world — **plain Rust, no ECS**). A
unit-language crate is ruled in (Q5: our own Python-shaped interpreter,
deterministic by construction) but unbuilt, pending Q13 and Q14. A renderer crate
is expected and still undecided. Neither is needed for the determinism gate to be
real.

## Determinism rules (CRITICAL — lockstep multiplayer)

The entire `sim` layer must be bit-for-bit deterministic across machines.
Violations surface as multiplayer desyncs, which are miserable to debug and
cheap to prevent. Non-negotiable rules for any code in `sim`:

1. **No ECS in `sim` — keep it that way.** World state is plain Rust structs +
   `BTreeMap`s, so iteration is deterministic by construction. If a renderer
   crate with an ECS lands, it may influence the sim exclusively through ordered
   `Command`s. ECS-side code feeding sim state any other way is the #1
   architecture violation to flag in review — every time.
2. **No float types (`f32`/`f64`) in any state-affecting path.** Integer /
   fixed-point math only. Floats are fine in rendering and UI.
3. **No `HashMap`/`HashSet` iteration in sim logic** — hash order is
   nondeterministic. Use `BTreeMap`, sorted `Vec`s, or sort before iterating.
4. **No wall clock, no frame time, no OS randomness.** All randomness comes
   from named, seeded RNG streams owned by the sim and advanced only by sim
   systems ([crates/sim/src/rng.rs](crates/sim/src/rng.rs)).
5. **All external input enters as ordered `Command` values** — even in
   single-player, which is lockstep with one peer.
6. Any query a unit program can make must return results in a stable sorted
   order, with ties broken by entity id.
7. Player programs are stored as **byte-exact plain text** (no whitespace
   normalization, UTF-8); program versions are identified by hashing source
   bytes.

Rules 2, 3 and 4 are *syntactic*, so they are also scanned mechanically by
`crates/sim/tests/no_floats.rs`. That test is a backstop, not the rule — it
finds banned names, not banned behavior.

Testing expectation: golden-replay tests (`(seed, command log) → state hash`)
guard determinism in CI. **A PR that changes a replay hash must explain why.**
Regenerate with `UPDATE_GOLDEN=1 cargo test -p sim --test golden`. A fixture
that gets silently regenerated whenever it goes red is worse than no fixture,
because it looks like coverage.

## Working on this repo

- `scripts/ci.sh` runs the whole suite locally; it is exactly what CI runs.
  `scripts/ci.sh docs` is the fast half (seconds, no Rust build).
- `scripts/install-hooks.sh` once per clone installs the pre-commit doc gate.
- [.claude/design-invariants.md](.claude/design-invariants.md) lists the
  properties the design corpus must have, each with what it caught.

## Design-doc conventions

These are ported wholesale, because every one of them was written after
something got through in the predecessor project.

- Every numeric value in docs (cycle costs, XP curves, timers) is a tuning
  constant, expected to live in data files, not code.
- **The live docs hold current state; closed records live in `docs/history/`.**
  This is the load-bearing convention, because the live docs are what gets read
  every session and history is what gets read almost never. A register that
  appends forever makes every reader pay for every ruling ever made — the
  predecessor's `QUESTIONS.md` reached 166 KB, nearly all of it answered.
- When a design decision is made, it moves to the owning doc's **Decided**
  section. Open items live in [docs/QUESTIONS.md](docs/QUESTIONS.md)
  (numbered — don't renumber, append) and **only** there: any other doc may cite
  a number inline ("open — Q12") but never restates a question's substance or
  leans. **Answering a question empties it out of that file** — ruling to
  `history/questions-answered-NNN.md`, worksheet to
  `history/questions-worksheets-NNN.md`, displaced status block to
  `history/questions-status-log-YYYY.md`.
- Known defects in *already-decided* text — a ruling that never propagated to
  its owning doc, a tuning number that fails arithmetic against its inputs, or a
  ratified decision the implementation never caught up to — live in
  [docs/PROBLEMS.md](docs/PROBLEMS.md) (numbered P1…, same append-only rule),
  **open entries only**. Fixing one moves it to `history/problems-fixed-NNN.md`
  with the commit hash.
- **History is sharded and bounded.** `scripts/check-registers.mjs` fails any
  register or shard over 40 KB, so filling one is what tells you to start the
  next — nobody has to notice. Shards fill in order, and each shard's H1 states
  the range it actually holds, derived and checked. There is no index table by
  design; `grep -H '^# ' docs/history/*.md` is the index.
- **Counts are derived, so they are stated once and checked.** The register's
  totals live in `docs/PROBLEMS.md`'s `(latest)` status headline and **nowhere
  else**. Other files name individual entries and their *relationships* ("P29
  closes as a consequence of Q127"), which is information they own and which
  cannot go stale as the register grows. `scripts/check-registers.mjs`
  recomputes the headline from the entries themselves — open ones in the live
  register, fixed ones across the history shards — on every CI run, and rejects
  a restated total anywhere else. This rule exists because every hand-maintained
  count in the predecessor corpus drifted — four separate stale counts in one
  day, one of them stale again inside the commit that fixed the other three.
  Splitting the fixed entries into history makes a hand-kept count strictly
  worse, since the number now lives in a different file from what it counts.
  Prefer an invariant to a running number wherever one is available ("every open
  entry is ⚠HASH except P31" beats "six of the seven").
- Raw observations land in [docs/INBOX.md](docs/INBOX.md), a deliberate
  **inbox**: write the problem down in whatever words come out, then *triage* it
  into a P-number, a question, or a task and move it to that file's **Triaged**
  log with a pointer. It is never spec and holds no rulings. An entry left in
  **Open** is a review finding, not a backlog item.
- **One block per date; a block records a day, not an edit.** While its date is
  current, rewrite the day's block freely — consolidate passes, restate counts,
  cut what a later pass made wrong; that is how a headline and its body stay in
  agreement. Once a later dated block exists the day is **closed**, and what
  closes is its *content*: no later counts, no later entries, no changed
  conclusions — correct those with a new block.
- `docs/history/` is **closed records, not spec** — it is expected to contradict
  current design and is never the authority on it. **Don't read it in a normal
  doc pass**; open one shard there only to recover *why* a past call was made,
  and only the shard whose heading covers the number you want. See
  [docs/history/README.md](docs/history/README.md).

### Splitting a doc

Once a numbered doc outgrows one file it splits Rust-module style: a doorway
`NN-name.md` beside an `NN-name/` directory. The doorway holds only the
invariants that cross its parts plus a table of what each part owns — **it is
not a summary and does not substitute for the parts.** Every part file opens
with `*Part of [NN-name](../NN-name.md).*`, a blank line, then its H1;
`scripts/check-doc-layout.mjs` enforces that, because seven of the predecessor's
62 part files had quietly inverted it and four hand reviews walked past all
seven. A ruling that changes a **cross-part invariant** must update the
doorway's list too, not just the part file; letting that drift is the split's
characteristic failure mode.
