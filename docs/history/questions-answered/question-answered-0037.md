*Closed record — see [../README.md](../README.md). Not spec.*

# Q37 — How are the mark tools picked, and how do the team's own plans read on the map?

## Ruling

*Ruling (2026-09-12):* **a strip, a drag, an armed tool, and a preview.**
The tools window shows every mark as a strip — the building models, the
eight paint colors, the overlay labels, and a *clear* at the end of each
row; a click arms one and it stays armed until `Esc`; a drag with a tool
armed marks every tile the cursor crosses once; `Shift`+click withdraws
the team's plan of the armed kind; and a tile the team has planned shows a
translucent version of what the plan would make it.

### The rules

1. **The strip.** Three rows, from data: the building models a plan may
   name (`printer` excluded, Q31), the paint colors, the overlay labels
   (Q10, Q26, Q27), each row ending in *clear* — the plan to empty that
   slot. `B`, `P` and `O` pick a row, `1`–`9` an entry in it, `Esc`
   disarms; clicking the armed entry disarms it too.
2. **Marking.** With a tool armed, a left click on a tile is a `Mark` of
   that kind and value; a left drag marks every tile the cursor crosses,
   each once per drag, and does not pan the camera; `Shift`+click is an
   `Unmark` of the armed kind. With no tool armed, a click selects and a
   drag pans, as today. The renderer submits and forgets (Q15): what the
   player sees is the plan the sim reports, `delay` ticks later.
3. **The preview.** A tile carrying the player's team's plan shows the
   plan as a translucent version of its effect — the color at half alpha,
   the overlay's label, a ghost of the building — and a clear plan as the
   slot hatched; the armed tool shows the same ghost under the cursor.
   Only the player's team's plans are drawn, since plans are private
   (`03`); a plan a bot has consumed is gone from the map when the sim
   says so. The snapshot carries the player's team's plan values per tile
   for this, which is a read and touches no hash.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — Marks, the screen's
  tools row, and the Decided entry. [QUESTIONS.md](../../QUESTIONS.md) — the
  status block.
- **Task:** [T20](../../TASKS.md) — the strip, the drag, the preview and
  the snapshot's plan values.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Strip, drag, armed, preview** *(chosen)* | The most to build; bought: a route painted in one stroke, and a map that shows what the player asked for before a bot gets to it. |
| Strip, click only, disarm after each | Nine clicks for a nine-tile route, and a tool to re-pick each time. |
| The radio list with hotkeys | Five of the dozens of marks reachable, the rest by editing the list. |

### How the answer took its shape

A drag that marks must not also pan, so the camera's left-drag pan yields
to an armed tool — the same rule the predecessor reached, where the
gesture code already distinguishes a click from a pan.
