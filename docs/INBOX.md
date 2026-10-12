# Inbox

Raw observations, in whatever words came out. **This file is never spec and holds
no rulings.**

The point of an inbox is that writing something down must be cheaper than
deciding where it goes. Dump it here; triage it later. An entry left here is a
review finding, not a backlog item — a doc pass that walks past this file has
missed something.

**This file holds untriaged entries only.** Triaging one moves it to
[history/inbox-triaged/](history/inbox-triaged/README.md) as its own file,
declaring where it went. Numbering is stable — **append new entries, never
renumber**, and never reuse a number. Entries are written as
`**I<n> — <short title>**` on their own line, which is how
`scripts/check-registers.mjs` finds them.

Triage means turning an entry into at least one of the outcomes below — two is
common, since an observation that opens a task often gets a doc fix at the same
time. The triaged file's `## Outcome` declares which, in these spellings and no
others:

- `- **Question:**` — a numbered question in [QUESTIONS.md](QUESTIONS.md),
  something undecided;
- `- **Problem:**` — a numbered entry in [PROBLEMS.md](PROBLEMS.md), decided
  text that is wrong;
- `- **Task:**` — a numbered task in [TASKS.md](TASKS.md), work to do;
- `- **Docs:**` — a documentation fix made on the spot, with the edits linked;
- `- **Dropped:**` — nothing, because it turned out to be a misreading.

That last outcome is the one this register has and the others do not, and it is
deliberate: an inbox that cannot absorb a false alarm stops being cheap to write
to, and then people stop writing to it. A dropped entry still gets a file and
still has to say why.

## Open

**I3 — A saved replay from before Q52 and Q53 probably no longer plays back the same**

`data/starter/replays/first-993-ec57f7d121fdc338.ron` was saved by a match
played before docstrings became free (Q52) and before the starter bot was
rewritten onto `lib/` (Q53). It is gitignored, so only this checkout has it,
and nothing in CI replays it. A replay that carries its bundles' bytes may
still replay, since the bot's source is in the file, but the interpreter now
charges that source less, so its state hash would likely differ from the one
in its name. Not tried. Worth knowing whether a replay is expected to survive
a language change at all, or is only promised to replay on the build that
wrote it.

**I4 — `NoIndirectDrawing` on the camera draws no 3D on this Mac**

While chasing the paint-plan flicker (#83, #84), adding Bevy's
`NoIndirectDrawing` to the `Camera3d` left the map black on macOS (Metal):
the panels drew, nothing 3D did. No error was logged. Not investigated
further, since the fix went another way (`crates/render/CLAUDE.md`). Matters
only if someone reaches for that setting later, for instance to make
see-through instance order deterministic.

**I5 — The language's golden fixtures cannot see a small cost change**

T30 made every `while` pass cost one more `op.statement`, and the
`showcase` and `interrupts` fixtures in `crates/lang/tests/golden` did not
move: their transcripts record calls, events and each tick's slice result,
so a cost change shows only if it moves one of those to another tick.
`showcase` has no `while` at all, `interrupts`' loops of 40 passes finish
in the same tick either way, and its 600-pass hook exhausts a fixed hook
budget at the same tick regardless. Only the match replay caught it. A
fixture whose transcript carried each tick's spent budget and deficit, or
the machine's `state_bytes` hash, would see any cost change. Worth deciding
whether the language fixtures are meant to guard costs, since the
determinism rules treat a cost as hash-affecting.
