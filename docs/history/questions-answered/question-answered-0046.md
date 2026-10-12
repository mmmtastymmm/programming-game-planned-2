*Closed record — see [../README.md](../README.md). Not spec.*

# Q46 — Amend Q42 and Q43: may a bot wear a number again, and which?

## Ruling

*Ruling (2026-10-11):* **every bot shows its entity id on its top face in
seven segments, lit in its team's color.** Made under a new number because
the rulings it amends are closed records: [Q42](question-answered-0042.md)
withdrew [Q40](question-answered-0040.md)'s deployment numbers and drew no
number on a bot, and [Q43](question-answered-0043.md)'s entry restated it.
Those numbers named a deployment, which Q42 made one per model; the
entity id names the bot itself, which the inspector and the log already
show as its name.

### The rules

All are the renderer's and reach no hash.

1. **The number is the bot's entity id** — the `17` of `bot17` — its last
   three digits right-aligned, leading places blank, so an id past 999
   shows its last three.
2. **It is a seven-segment display set into the top face**, in a dark
   bezel behind the facing chevron: lit segments glow in the team's color
   like the lights (Q45), unlit ones stay dark but visible.
3. **The segments never move; which are lit does.** They are a fixed
   lattice on the lid that holds the three-digit number in any of the four
   quarter turns, and the number is lit in the turn that reads upright
   from the camera — switching as the bot turns past halfway or the camera
   orbits past halfway, like a display that redraws. The chevron, the trim
   (the paint) and the light (the team) keep the heading, the program's
   color and the team readable from above.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided entry
  and Machines on the map. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T25](../../TASKS.md) — amended to build the number.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **The entity id, three digits** *(chosen)* | An id past 999 is ambiguous on the map; bought: the bot on the map is the bot named in the inspector and the log, with no sim change. |
| A number the program sets (`set_display`) | A new attribute in the hash; a program could show a job or a squad, as `set_color` shows a color. |
| No number (Q42 as ruled) | A bot on the map cannot be matched to its log line or inspector record by eye. |
| A display that turns to face the camera | Reads upright with larger digits, but a display that physically spins on the lid breaks the illusion of a device switching segments. |
| A hexagonal lattice | Symmetric under sixth turns, not the quarter turns a bot makes. |

### How the answer took its shape

The segments are geometry, not a texture. A number fixed to the lid in
one turn reads upside down whenever the bot faces the camera; the first
build turned the whole display to the camera instead, which read well
but spun on the lid, and was replaced. A three-digit number fills a
5-by-2 band of cells; turned about a half-cell point, every quarter turn
of that band lands on the same integer lattice, so its union — 52 edges,
a 5-by-5 grid without its corner edges — is a lid that looks the same
from every side and can spell the number upright from any of them. The
number sits half a cell toward the reader in every turn, which is the
price of the shared lattice. The top face's hatch drawing gave way to the
bezel, since the two overlapped.
