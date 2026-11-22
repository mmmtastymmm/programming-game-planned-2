*Closed record — see [../README.md](../README.md). Not spec.*

# T32 — Power in `render`: wire, pylons, the charger, the flat and dark marks, the inspector

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
opened: it closes [P8](../problems-fixed/problem-fixed-0008.md).

Completed in `054bd20`, the merge of #127; the task's own commit is
`27297c2`. The rules that decide what is drawn are plain functions in
`crates/render/src/power.rs`, tested without a window: which neighbours a
wire runs a line to and when it is dimmed, which pairs a pylon draws,
a rate per second at the speed in force, and the inspector's lines. The
drawing was checked by eye on a copy of the showcase replay with more
power marks in it: plan ghosts, built pylons and their lines, a charger,
a panel, and wire lit and dimmed. The bolt and the charge bar were looked
at by forcing them on for a frame, since no bot ran out of power in that
replay; the inspector's lines, a rival's wire and a plan to take a wire
up were not seen on screen. One line of `07` changed with it: a pylon's
line runs from its head to the head of each pylon and the top of each
other building, where the text had it level at the height of the head,
because a level line ended in the air above a low building. No sim code
changed, so no replay hash moved. It was the last task under M10, Power,
and fixes [P8](../problems-fixed/problem-fixed-0008.md).
