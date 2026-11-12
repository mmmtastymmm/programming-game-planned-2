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

## M10 — Power

**T31 — Power in `sim`: the grid, powered bots, wires and the solar panel** `⚠HASH`

Q59, ruled in `02`'s Power section, `03`'s tile, `06`'s tick and
snapshot, and `01`'s Metering. The data: the bot's `power_draw`,
`wire_cost`, `wire_ticks` and `unwire_ticks`; every building's
`power_supply`, zero for the depot and the site; a `solar` model with
every row a building model has — `cost`, `cost_step` (zero),
`build_ticks`, `deconstruct_ticks`, `health`, `death_threshold`, `store`
(zero), `defence_damage` (zero), `vision` and `hearing`; the
`[deployment]` comment naming four deployments; and a load check refusing
a printer supply below one bot's draw. The tile's per-team
`wires`, on `ground` only, sensed and remembered like paint, and cleared
with an out team; the wire plan as a fourth plan kind in `Mark` and
`Unmark`, only on seen ground and dropped when it asks for what the tile
already has; the `wire` and `unwire` actions, re-checked on completion,
taking `wire_cost` from the load and consuming the plan the tile now
matches. Step 6 as power and printing: each team's grid traced from its
printers through its wires and buildings, sites excluded; supply from
buildings on the grid above zero health; bots powered oldest first while
they fit; an unpowered bot's main flow and main-flow action held, its
budget still opened and paying down a deficit, its handlers run; a print
begun only while the spare covers every bot and one more; the team's
supply, demand and spare kept as team state; the power half run once at
tick 0. The attributes `powered` and `on_grid`; the
builtins `power()` and `wires()`, and for every new builtin a `[game]`
cost row and a hover entry in `data/language/docs.toml` (Q57), whose test
holds the two tables equal, `power()`'s signature naming the `Power`
record (`01` syntax, Annotations); the hover entries Q59 makes stale
brought up to date — `tile()`'s `wires`, `plans()`'s `"wire"` kind, the
`Machine` record's `powered` and `on_grid`, and `build()`'s and
`build_nearest()`'s `solar`; the `solar`
deployment in `Deploy` validation; the snapshot's tile wires, own-wire
grid flags and team figures; the state hash over all of it. The starter:
`robots/solar.py`, and a wire step in `lib/jobs` that carries out the
team's wire plans, so a panel planned and wired on the default tree
supplies; the starter's docstrings and comments brought up to date
(design-invariant DI8) — `lib/sense.py`'s list of models gains `solar`,
and `lib/jobs.py`'s and `robots/bot.py`'s mark keys gain `C`. Tests: a grid cut by unwiring drops a panel's supply, the newest
bots stop first, a stopped bot's move resumes when power returns, a
`dying` unpowered bot still reaches `death`, a team short of power still
runs its oldest bot, two bots wiring one tile pay once, and a printer
with no spare power begins no print. The golden fixtures change, and the
PR says why. Fixes [P8](PROBLEMS.md) with [T32](TASKS.md).

**T32 — Power in `render`: wires, the wire tool, the unpowered mark, the panel**

Q59, ruled in `07`'s Decided entry, the screen table, Machines on the map
and Marks. Wires drawn from the snapshot as a bar across each wired tile
and lines to the same team's wires and buildings beside it, sites
excluded, two teams side by side, the player's own off-grid wires dimmed;
the strip's wire row on `C`, its plan's see-through line, and its
ghost only on seen ground and only where it would change the tile, a
clear withdrawing a standing wire plan; every new see-through layer — the
wire plan's line, the dimmed off-grid wires — on a mesh asset of its own
or opaque, as `crates/render/CLAUDE.md` requires of any translucent layer
over paint; a small
struck-through bolt over an unpowered bot and over a building that
supplies power but is off the grid, its lights unchanged; the solar panel's art, a low tilted plate with a lamp;
the inspector's supply, demand and spare for the player's own team, and
the selected bot's `powered` or building's `on_grid`. [P8](PROBLEMS.md)
closes with whichever of T31 and T32 lands last.
