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
`flat`, or `powered` and `grid`. The last of the three tasks Q62 and Q63
opened: it closes [P8](PROBLEMS.md).
