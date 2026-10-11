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

## M6 — The match's edges

**T20 — The mark strip, drag marking, the armed tool and the plan preview**

Q37: the tools window as three rows from data, each ending in a clear,
with `B`/`P`/`O` and `1`–`9`; a tool armed until `Esc`; a drag marking
every tile it crosses once and not panning; `Shift`+click an `Unmark` of
the armed kind; the player's own plans drawn as a translucent version of
their effect and the armed tool as a ghost under the cursor. The snapshot
gains the player's team's plan values per tile — a read the renderer
makes, in no hash — replacing the driver's `snapshot_plan` peek.

## M7 — Buildable printers

**T24 — Build printers at a rising price, print by turns, drop conversion, one deployment per model** `⚠HASH`

Q42, ruled in `02`'s Decided entry and its Printing section, `04`'s
ending, `05` and `06`'s print step. The sim: `build` and plans accept
`printer`; a site's capacity is `cost + n × cost_step`, `n` the team's
printers and printer sites at placement; the `print` builtin and its cost
row go, printing becomes tick step 6 with a per-team print turn in the
hash and a `printing` attribute on printers; `convert` and
`converted_this_tick` go; a team is out at the end of a tick with no
printer; deployments are `bot`, `printer` and `depot` only, a `Deploy`
naming another is refused at submission, and bots are named `bot<id>`.
The data: the printer's `cost`, `cost_step` and `build_ticks`, a
`cost_step` of zero on the depot, `convert_ticks` and the conversion sound
removed, the paint list's comment. The shipped programs: `data/starter`
and `data/opposition/fool` move `robots/1.py` to `robots/bot.py`, their
printer programs stop calling `print`, and the Fool's script deploys to
`bot`. The renderer: one robot file per model in the tree, bots in the
team's color with no number, `printer` on the mark strip. The tests and
the golden fixtures, regenerated with the reason in the PR.

**T25 — A bot's `set_color`, and the team outline the player colors** `⚠HASH`

Q43, ruled in `02` and `07`. The sim: a `color` attribute on bots, `None`
when printed, kept across redeploy and fault, in sightings and the hash;
the zero-tick `set_color(color)` action, refusing anything not in the
paint list or `None`, with its row in `data/language/costs.toml`. The
renderer: a bot's body in its color or the neutral body color, buildings
in the neutral color, an outline on every machine in its team's color
replacing the ring, the player's own green by default and the others from
a list in `data/interface.toml`, changed from the inspector and saved in
`colors.toml` beside the programs. The golden fixtures, regenerated with
the reason in the PR.
