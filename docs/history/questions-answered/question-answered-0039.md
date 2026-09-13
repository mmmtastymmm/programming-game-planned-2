*Closed record — see [../README.md](../README.md). Not spec.*

# Q39 — Amend Q33 and `07`'s screen: everything floats over the map, as windows

## Ruling

*Ruling (2026-09-12):* **the map fills the window, and everything else is a
window over it** that the player drags, resizes, collapses and closes: one
per deployment's program, one for the inspector, one for the mark tools,
one for the log. The time bar is the only fixed strip, and it is the dock:
a button per window opens and closes it. The layout — each window's
position, size and whether it is open — is remembered between runs in a
file beside the programs directory. Made under a new number because
[Q33](question-answered-0033.md) placed the editor "in a panel beside the
map", and `07`'s screen ruled four fixed regions; both are closed records.

### The rules

1. **One window per deployment**, titled with its name and its state
   (`*` ahead, `!` a load error), holding what Q33's panel held: the file
   tabs, the text, the running version against the working copy, the
   deploy and export buttons, and the deployment's fault summary (Q34).
   Several may be open at once, which is the point: two programs side by
   side over the match they run.
2. **The inspector, the tools and the log are windows too**, with what
   `07` gave the right and bottom regions.
3. **The time bar is fixed** and doubles as the dock: a toggle per window,
   the deployments in the tabs' order, then inspector, tools, log.
4. **A window's layout is the renderer's** — never a peer's concern, never
   in a hash — and is saved to `windows.toml` in the programs directory
   whenever it changes, and restored at start; a missing or malformed file
   means the default layout.
5. **The map under a window is the window's**: a click there is not a
   click on a tile, which egui already rules.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — The screen, the
  Decided entry, and Q33's entry marked amended. [QUESTIONS.md](../../QUESTIONS.md)
  — the status block.
- **Task:** [T18](../../TASKS.md) — the windows.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Fixed panels around the map (Q33, `07` as first written) | One program at a time, and a fixed share of the screen for each region whether or not the player wants it now. |
| **Floating windows over the map** *(chosen)* | The player arranges the screen — two programs open side by side, the log gone until wanted — at the cost of a layout to remember and a map that windows can cover, which panning solves. |
| Programs float, the rest fixed | Half the benefit; the inspector and log are exactly the things a player wants to move out of the way. |

### How the answer took its shape

The reference is The Farmer Was Replaced, whose code windows float over
the farm and can all be open at once; the ruling is that shape with one
window per deployment rather than per file, since a deployment is what a
`Deploy` addresses (Q3).
