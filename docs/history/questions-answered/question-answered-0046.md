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
   like the lights (Q45), unlit ones stay dark.
3. **It turns to face the camera**, so it reads upright whichever way the
   bot faces; the chevron, the trim (the paint) and the light (the team)
   keep the heading, the program's color and the team readable from
   above.

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

### How the answer took its shape

The segments are geometry, not a texture: seven small boxes per digit,
spawned once per bot since its id never changes, lit or dark by the
digit. A display fixed to the face would read upside down whenever the
bot faced the camera, so it turns to the camera's yaw, sized and placed
so the turning block clears the chevron, the trim and the light. The top
face's hatch drawing gave way to the bezel, since the two overlapped.
