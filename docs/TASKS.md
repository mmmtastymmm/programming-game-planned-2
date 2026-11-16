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

**T39 — `low_power` in `lang`: the fifth interrupt kind, its hook, and a call that waits**

Q62, ruled in `01`'s execution part. The kind `low_power`, ranked between
`fault` and `redeploy`, raised by the host and delivered at the next
boundary like `dying`; one pending entry, coalesced; a `fault` or a
higher handler running makes it wait, and it makes a `redeploy` wait. Its
prologue, its hook `on_low_power()` — bound at load from `main.py` or
the bundle file `on_low_power.py`, arity zero, any other a load error —
under `budget.hook.low_power` in `data/language/limits.toml`, and its
epilogue, which restarts main flow with every variable cleared and writes
no fault record. Escalation from the hook, by an escaping exception or
its budget, to `fault`, with the record written. The wait before a call:
the host may answer a game builtin's call with "not yet", which leaves
main flow at the boundary before the call, interrupts delivered there,
to be asked again at the next slice's opening — never inside a hook; and
the host may hold a machine's main flow for a whole slice, as a dark
building's is, its budget still opened. A fork stops on a `low_power`
restart as on a fault's (`06`, Steps). Language fixtures for each rule.
Nothing raises the kind until T31, so the golden fixtures hold; the PR
says so if they do not. With T31 and T32, fixes [P8](PROBLEMS.md).

**T31 — Power in `sim`: batteries, charge costs, grids, the charger, the pylon, the panel and wire** `⚠HASH`

Q62, ruled in `08`, with `02`'s models, attributes, senses, actions and
Printing, `03`'s tile and `06`'s tick, snapshot and hash. Needs T39.

The data, in `data/machines.toml`: every model's `battery` and
`generation`, and every building model's `drain`; the printer's
`print_drain`; `recharge_rate` on the printer and the charger; the bot's
`recharge_ticks`, `wire_cost`, `wire_ticks` and `unwire_ticks`; a `charger`,
a `solar` and a `pylon` model with every row a building model has, the
pylon's `span` besides; `[power]` with `low_power_level`, and
`[power.cost]` with a row for every game builtin; the `[deployment]`
comment naming five deployments; and the four load checks `08` lists.
For every new builtin a `[game]` cost row and a hover entry in
`data/language/docs.toml` (Q57), whose test holds the two tables equal;
the hover entries Q62 makes stale brought up to date — the `Machine`
record's `charge`, `battery`, `flat`, `powered` and `grid`, `tile()`'s
`wires`, `plans()`'s `"wire"` kind, and `build()`'s and `build_nearest()`'s
models.

The bot: `charge`, `battery` and `flat`; a charge cost taken as a call
begins, the flat wait before a call it cannot pay, and a hook's call
taking what there is; the solar in the power step; whether it is armed,
and `low_power` raised as the charge falls to the level and armed again
above it; `recharge()`, re-checked on completion; a printed bot full.

The grid: the tile's per-team `wires`, on `ground` only, sensed and
remembered like paint and cleared with an out team; the wire plan as a
fourth plan kind in `Mark` and `Unmark`, only on seen ground and dropped
when it asks for what the tile already has; `wire` and `unwire`,
re-checked on completion; conductors, joins by adjacency and by a
pylon's span, labels; step 6 as `08` orders it — trace, the three passes,
solar, prints, figures — and its tick-0 form; `powered` and `grid`; a
dark building's rules, program and senses stopped and its handlers run;
a print begun only by a powered printer and held while dark, with
no spare power and no second cap, which went with Q59; `power()` and `wires()`; the
`charger` deployment in `Deploy` validation, and none for the pylon; the
snapshot's tile wires, their part-of-a-grid flags and each team's grids;
the state hash over all of it.

The starter: `robots/charger.py` and `robots/solar.py`,
`interrupts/on_low_power.py`, a wire step and a recharge step in
`lib/jobs`, so a bot low on charge goes to a powered building and a panel
planned and wired on the default tree feeds a charger; the starter's
docstrings and comments brought up to date (design-invariant DI8).

Tests: a call the battery cannot cover waits and runs when the solar
covers it; a hook's call is never held; `low_power` comes once per fall
and again only after a recharge above the level; a hookless bot restarts;
a grid cut by destroying a pylon, and one cut by `unwire`, each darken
what the rest can no longer cover; the older building is served first and
the next is tried when one does not fit; a dark building still fills its
battery and is powered again by itself; a dark building's tiles become
remembered; a lone printer is never dark, and data that would let it be
is refused; a rival's bot recharges from a charger; two bots recharging
from one building in a tick are served lower id first; two bots wiring
one tile pay once. The golden fixtures change, and the PR says why.

**T32 — Power in `render`: wire, pylons, the charger, the flat and dark marks, the inspector**

Q62, ruled in `07`'s Decided entry, the screen table, Machines on the map
and Marks. Wires drawn from the snapshot as a bar across each wired tile
and lines to the same team's wires and buildings beside it, sites
excluded, two teams side by side, the player's own wires that are part
of no grid dimmed; a pylon's lines to its team's pylons and buildings
within its span, worked out from positions and the span in data, above
machines; the strip's wire row on `C`, its plan's see-through line, and
its ghost only on seen ground and only where it would change the tile, a
clear withdrawing a standing wire plan; `charger` and `pylon` in the
strip's building row; every new see-through layer on a mesh asset of its
own or opaque, as `crates/render/CLAUDE.md` requires of any translucent
layer over paint; a small struck-through bolt over a flat bot and a dark
building, their lights unchanged; the charge bar under the health bar of
a selected or hovered machine; the art — the panel a low tilted plate,
the charger a squat block with a gauge that fills, the pylon a thin
mast, each with a lamp; `interrupts/on_low_power.py` and
`robots/charger.py` as fixed files of the tree; the inspector's grids,
with generation and drain per second at the speed in force and stored
charge against capacity, and the selected machine's charge with its
`flat`, or `powered` and `grid`. [P8](PROBLEMS.md) closes with whichever
of T39, T31 and T32 lands last.
