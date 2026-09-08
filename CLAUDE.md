# Programming Game 2 (working title)

A lockstep-multiplayer programming game. The player writes a small number of
programs, a fleet of identical machines runs them, and **the player rewrites those
programs while the match is running** — each update a lockstep-synchronized
command applied on the same tick by every peer (Q3). The player never commands an
individual machine. The simulation runs identically on every peer.

The design is in its first pass. The framing rulings live in
[docs/00-overview.md](docs/00-overview.md)'s Decided section, the language's
in [docs/01-language.md](docs/01-language.md)'s parts, the machines' in
[docs/02-machines.md](docs/02-machines.md) and the world's in
[docs/03-world.md](docs/03-world.md); **everything still undecided is in
[docs/QUESTIONS.md](docs/QUESTIONS.md)**, which is the only place it may be — a
list of open topics repeated here would drift on every ruling. The corpus
scaffolding and the determinism gate are ported from the predecessor project
`../programming_game_planned` and proven in CI.

Crate layout: `crates/sim` (deterministic world — **plain Rust, no ECS**). A
language crate is ruled in — Q5 and Q13, whose rulings
[docs/01-language.md](docs/01-language.md) owns and this file does not repeat —
but unbuilt: that doc is its spec, and milestone M2 in
[docs/TASKS.md](docs/TASKS.md) builds it. A renderer crate is
open — Q15. Neither is needed for the determinism gate to be real.

## Determinism rules (CRITICAL — lockstep multiplayer)

The entire `sim` layer must be bit-for-bit deterministic across peers.
Violations surface as multiplayer desyncs, which are miserable to debug and
cheap to prevent. Non-negotiable rules for any code in `sim`, and for the
language crate when it lands:

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
6. Any query a machine program can make must return results in a stable sorted
   order, with ties broken by entity id.
7. Player programs are stored as **byte-exact plain text** (no whitespace
   normalization, UTF-8); program versions are identified by hashing source
   bytes. Q13 admits `import`, so a program is a **bundle of named files**: its
   version is the hash of every file's name and bytes, each length-prefixed,
   taken in **sorted name order** — the exact byte layout is in
   [docs/01-language/syntax.md](docs/01-language/syntax.md), and the wording
   gained "name" and "length-prefixed" when P1 found the bytes-only form was
   not injective — and a circular
   import is rejected at load rather than resolved.

Rules 2, 3 and 4 are *syntactic*, so they are also scanned mechanically by
`crates/sim/tests/no_floats.rs`. That test is a backstop, not the rule — it
finds banned names, not banned behavior.

Testing expectation: golden-replay tests (`(map, command log) → state hash`)
guard determinism in CI. **A PR that changes a replay hash must explain why.**
Regenerate with `UPDATE_GOLDEN=1 cargo test -p sim --test golden`. A fixture
that gets silently regenerated whenever it goes red is worse than no fixture,
because it looks like coverage. For the same reason, **a test that fails to run
must not score green**: an error transcript hashes identically in every process,
which is indistinguishable from agreement.

## Working on this repo

- `scripts/ci.sh` runs the whole suite locally; it is exactly what CI runs, so
  the two cannot drift. `scripts/ci.sh docs` is the fast half (seconds, no Rust
  build); `scripts/ci.sh rust` is fmt, clippy and tests.
  - `scripts/install-hooks.sh` once per clone installs the pre-commit gate,
    which refuses an oversized staged file and runs **most** of the fast half
    against the **staged** content — two steps stand down when it is handed a
    directory rather than the repository, and say so as they skip.
    `scripts/check-file-size.sh` owns the limit and has two modes: `staged` for
    the hook, which can still stop the blob being written,
    and `repo` for CI, which cannot — a large file is permanent once pushed — but
    which is the only one that runs for a contributor who never installed the
    hook, and the only one that reads the *history* rather than the index, where
    a blob deleted by a later commit still sits.
- **`scripts/check-checks.mjs` is the check on the checks.** It seeds known
  defects into a copy of the working tree and asserts each is caught by the
  right check with a message that names the real problem. It reads the working
  tree, not the index, so it agrees with every other local check — at the cost
  that a file you forgot to `git add` passes here and fails on CI's fresh clone.
  The pre-commit hook reads the index and narrows that gap, but it does not
  close it: it is opt-in per clone (`scripts/install-hooks.sh`), `--no-verify`
  skips it, and it stands down when Node is absent. **CI is the only thing that
  actually sees what a fresh clone sees.** **Every review round of this repo has
  found checks that were green while validating nothing** — citations scanned on
  one line of a wrapped bullet, fenced examples read as entries, success reported
  on zero inputs, a size gate no CI job ran. Each was found by mutating a corpus
  copy by hand, and the round that skipped that step is the round three of them
  shipped in.
  **Adding a check means adding mutations there**; a check with no mutation is a
  check nobody has ever seen fail.
- [.claude/design-invariants.md](.claude/design-invariants.md) lists the
  properties the design corpus must have, each with how to check it. Some bind
  only once the design grows the structure they describe; each says so.

## The four registers

Everything that accumulates lives in one of four registers, and they all work the
same way: **the live doc holds open entries only; closing one moves it to a file
of its own under `docs/history/`.**

| Register | Open entries live in | Closed entries move to | "Closed" means |
|---|---|---|---|
| Questions | [docs/QUESTIONS.md](docs/QUESTIONS.md) (`Q1…`) | `history/questions-answered/` | answered |
| Problems | [docs/PROBLEMS.md](docs/PROBLEMS.md) (`P1…`) | `history/problems-fixed/` | fixed |
| Tasks | [docs/TASKS.md](docs/TASKS.md) (`T1…`) | `history/tasks-completed/` | done |
| Inbox | [docs/INBOX.md](docs/INBOX.md) (`I1…`) | `history/inbox-triaged/` | triaged |

Rules common to all four, enforced by `scripts/check-registers.mjs`:

- **Numbering is dense and append-only.** Never renumber; never reuse a number
  after it closes.
- **An entry is open or closed, never both.**
- **One file per closed entry**, named for its number, with a heading that
  agrees: `question-answered-0013.md` holds `# Q13 — …` and nothing else. The
  filename *is* the index — `ls` is the table of contents, so there is no index
  table to maintain and none to go stale. Reading one ruling costs one ruling.
- **Nothing exceeds 40 KB**, live docs and history entries alike.

What each register uniquely holds:

- **Questions** — anything undecided, and **only** here. Another doc may cite a
  number inline ("open — Q12") but never restates a question's substance or its
  leaning; a question restated in two places gets answered in one of them.
- **Problems** — defects in *already-decided* text: a ruling that never
  propagated to its owning doc, a tuning number that fails arithmetic against its
  inputs, or a ratified decision the implementation never caught up to. Fixing
  one records the commit that closed it.
- **Tasks** — work to do, grouped under milestones. Milestones are groupings, not
  entries; a milestone is finished when every task under it has left the file.
  `⚠HASH` marks a task that changes sim behavior and therefore the golden-replay
  hashes, and it marks nothing else — a question is not work, and almost every
  open question would carry it, which is a marker that has stopped selecting.
- **Inbox** — raw observations, in whatever words came out. Writing one down must
  be cheaper than deciding where it goes; triage it later. An entry left here is
  a review finding, not a backlog item — a doc pass that walks past this file has
  missed something.

### Every closure declares its consequence

An answered question and a triaged inbox entry each carry an `## Outcome` section
saying where it went. At least one of:

| | Means |
|---|---|
| `- **Docs:**` | edits made in the same commit, with links |
| `- **Question:**` | a `Q<n>` opened |
| `- **Problem:**` | a `P<n>` opened, because existing text or code is now wrong |
| `- **Task:**` | a `T<n>` opened, because the work is not a doc edit |
| `- **Dropped:**` | **inbox only** — a false alarm, with the reason |

Two is normal: a ruling that lands in a Decided section often also creates work.
**Every cited number must resolve**, and the checker validates each one on the
line, not just the first.

Zero is not allowed for a ruling, and that is the point. A ruling that closes
with no consequence anywhere is either not a real ruling or a propagation that
was forgotten, and a month later those are indistinguishable — that failure is
why the predecessor's stalest text sat in its most authoritative-looking place,
where reading passes skim it because it looks settled.

**`Dropped` exists for the inbox alone.** An inbox that cannot absorb a false
alarm stops being cheap to write to, and then it goes unused and the observation
is lost instead — strictly worse than a file saying "misread this, here is why."

## Other doc conventions

- **The live docs hold current state; `docs/history/` holds closed records.**
  This is the load-bearing convention, because the live docs are read every
  session and history is read almost never. A register that appends forever makes
  every reader pay for every ruling ever made — the predecessor's `QUESTIONS.md`
  reached 166 KB, nearly all of it answered.
- `docs/history/` is **not spec.** It is expected to contradict current design
  and is never the authority on it. **Don't read it in a normal doc pass**; open
  the one file whose number you want, only to recover *why* a past call was made.
  See [docs/history/README.md](docs/history/README.md).
- When a design decision is made, it lands in the owning doc's **Decided**
  section. That section is what an implementer builds from, so it gets its own
  reading pass — it reads as settled history, and the eye slides past.
- **Counts are derived, so they are stated once and checked.** The register's
  totals live in `docs/PROBLEMS.md`'s status headline and **nowhere else**. Other
  files name individual entries and their *relationships* ("`P29` closes as a
  consequence of `Q127`"), which is information they own and which cannot go stale
  as the register grows. `check-registers.mjs` recomputes the headline from the
  entries themselves — open ones in the live register, fixed ones in the history
  directory — and rejects a restated total anywhere else. This rule exists
  because every hand-maintained count in the predecessor corpus drifted: four
  separate stale counts in one day, one of them stale again inside the commit
  that fixed the other three. Prefer an invariant to a running number wherever
  one is available ("every open task is `⚠HASH` except `T31`" beats "six of the
  seven").
- **A status block is current state, rewritten in place.** `QUESTIONS.md` and
  `PROBLEMS.md` each carry one dated status block and no more. Do not stack them
  and do not archive them: `git log -p` already records every status either file
  has carried, dated and attached to the commit that changed it — a stricter
  record than a hand-maintained one, and one that cannot be forgotten.
- Every numeric value in docs (cycle costs, XP curves, timers) is a tuning
  constant, expected to live in data files, not code — `data/` holds them, and
  `data/language/costs.toml` is the first — except a constant of the language itself, such as the scale of `num` (Q14, amended by Q19),
  which is spec:
  changing it changes every replay hash, so it is stated once in `docs/01` and
  changed only under a new question number.

### Splitting a doc

Once a numbered doc outgrows one file it splits Rust-module style: a doorway
`NN-name.md` beside an `NN-name/` directory. The doorway holds only the
invariants that cross its parts plus a table of what each part owns — **it is
not a summary and does not substitute for the parts.** Every part file opens
with a breadcrumb naming its **immediate** parent doorway, a blank line, then
its H1 — `*Part of [NN-name](../NN-name.md).*` for a part sitting directly in
`NN-name/`, and `*Part of [runtime](../runtime.md).*` for one nested a level
deeper in `NN-name/runtime/`, because the parts nest the way Rust modules do.
`scripts/check-doc-layout.mjs` enforces that opening, and that the doorway
exists at all, because seven of the predecessor's 62 part files had quietly
inverted the crumb and four hand reviews walked past all seven. A ruling that
changes a **cross-part invariant** must update the doorway's list too, not just
the part file; letting that drift is the split's characteristic failure mode.
