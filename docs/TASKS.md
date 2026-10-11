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

## M7 — Bot colors and team outlines

**T25 — A bot's `set_color`, and the team outline the player colors** `⚠HASH`

Q43, ruled in `02` and `07`, amended by Q45, Q46 and Q47. The sim: a
`color` attribute on bots, `None` when printed, kept across redeploy and
fault, settable while `dying`, in sightings and the hash; the zero-tick
`set_color(color)` action, refusing anything not in the paint list or
`None`, with its row in `data/language/costs.toml`. The renderer: a bot's
body in its color or the neutral body color with its lights glowing in its
team's color, printers and depots in the neutral color with their lights
glowing in it and sites outlined in it, replacing the ring (Q45), its
entity id in seven segments on its top face lit in the team's color (Q46),
the player's own green by default and the others from a list in
`data/interface.toml`, changed from the inspector and saved in
`colors.toml` beside the programs. The golden fixtures, regenerated with
the reason in the PR. Finishing it fixes [P2](PROBLEMS.md).
