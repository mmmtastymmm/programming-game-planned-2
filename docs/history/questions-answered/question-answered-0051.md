*Closed record — see [../README.md](../README.md). Not spec.*

# Q51 — Amend Q50 and Q37: a smaller overlay marker, and no cursor ghost over the same plan

## Ruling

*Ruling (2026-10-11):* **an overlay plan's marker is smaller too; and the
armed tool shows no ghost on a tile that already carries exactly the plan
it would make.** Made under a new number because the rulings it amends
are closed records: [Q50](question-answered-0050.md) left an overlay
plan's marker full size, and [Q37](question-answered-0037.md) drew the
armed tool's ghost under the cursor on any tile.

### The rules

Both are the renderer's and reach no hash.

1. **An overlay plan is a smaller marker**, scaled as a paint or building
   plan is (Q50), with its label over it. A plan to clear still hatches
   the marker full size.
2. **No ghost over the same plan.** Where the hovered tile, or a tile of
   a `Shift`+drag's rectangle, already carries the plan the armed tool
   would make — same kind, same value — the tool draws no ghost there,
   and no second overlay label: the placed plan already shows what a click
   would do. Two identical translucent ghosts in one place fought over
   which drew in front and blinked, most visibly after placing a printer
   with the cursor still on the tile.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — Marks.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Skip the ghost over the same plan** *(chosen)* | Hovering a planned tile with the same tool shows only the placed plan; bought: no blinking, and nothing a click would add is hidden. |
| Draw the cursor ghost a little larger | Two stacked translucent ghosts still darken each other, and the placed plan reads differently under the cursor than elsewhere. |
| Keep both (Q37 as ruled) | The two coincide exactly and blink. |

### How the answer took its shape

Seen in play right after Q50: placing a printer plan and leaving the
cursor on its tile made the tile blink, as the hover had once done near
the inspector. A burst of frames showed the placed ghost and the cursor
ghost overlapping, lighter and patchier than the placed ghost alone.
