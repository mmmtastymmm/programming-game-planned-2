*Closed record — see [../README.md](../README.md). Not spec.*

# Q45 — Amend Q43: how does a bot show its team, now that its body shows its program's color?

## Ruling

*Ruling (2026-10-11):* **a bot shows its team by its lights, glowing in
the team's color over its paint; a building keeps its outline.** Made
under a new number because the ruling it amends,
[Q43](question-answered-0043.md), is a closed record. Its rule 5 outlined
every machine; seen in play, an outline round every bot crowded the map,
and a bot already has parts that read as lights.

### The rules

All are the renderer's and reach no hash.

1. **A bot's lights glow in its team's color.** The lights are the parts
   of its art marked as lights: the lens, the marker light on each side,
   the tail light and the dot on top; the 3D lens glass is one too. The
   rest of the art — the band, the trim and the chevron — is its paint,
   in its `color` or the neutral body as Q43 rules. Paint and lights
   combine: a bot painted red by its program on the blue team is a red
   bot with blue lights.
2. **A bot has no outline.** A building keeps Q43's outline in its team's
   color, until a ruling gives buildings lights of their own.
3. **The team colors are Q43's**: the player's own green by default, the
   others from the list in `data/interface.toml`, any of them changed from
   the inspector and saved in `colors.toml`. A change recolors every light
   and outline of that team at once.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided entry,
  the inspector row, Machines on the map and Costs.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T25](../../TASKS.md) — amended to build lights on bots and
  outlines on buildings.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Lights in the team color, paint on the rest** *(chosen)* | Team reads from small parts, so less well zoomed out; bought: one body carries both the program's color and the team's, and no outline crowds the map. |
| Q43 as ruled: an outline on every machine | Every bot ringed; seen in play, the outlines crowded the map and fought the paint. |
| The band in the team color, the lights in the paint | The team reads from far, but the program's color shrinks to the lights. |

### How the answer took its shape

The bot art already drew its band, trim and lights in one swapped accent
palette, so the split costs no new art: the light parts are marked in the
art with a color of their own, the bake draws them dark in every paint
atlas and white in one mask, and the body glows through the mask in its
team's color. Buildings keep their outline here only because the bot was
done first; their lights are a later question, not a rejected one.
