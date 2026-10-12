*Closed record — see [../README.md](../README.md). Not spec.*

# Q48 — Amend Q47: should a printer's bands carry its team's color, and its lights be larger?

## Ruling

*Ruling (2026-10-11):* **yes: a printer's bands and trim are painted in
its team's color, and its lights are larger.** Made under a new number
because the ruling it amends, [Q47](question-answered-0047.md), is a
closed record. Its rule 1 left a printer's bands in the neutral body;
seen in play, a printer's team read too faintly from its small lights
alone.

### The rules

All are the renderer's and reach no hash.

1. **A printer's trim is its team's color**: the band and stripe round
   its front and sides, the line on its back, and rails along both sides
   and the front edge of its top — the face the camera sees most. The
   trim is painted at a share of a light's glow, so it reads as solid
   color, not another lamp.
2. **A printer's lights are larger**: the panel light, the side markers
   and the top status light, glowing at full strength as Q47 rules.
3. **The rest of the printer stays the neutral body**, as Q43 rules for a
   building; a printer still has no `color` of its own.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided entry
  and Machines on the map. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Bands in the team color, larger lights** *(chosen)* | A printer becomes strongly team-colored, and green trim is quieter on grass than other colors; bought: a printer's team reads at a glance. |
| Larger, brighter neutral bands | A printer's team would still rest on small lights. |
| Printers get `set_color` | A sim change — an attribute in the hash — for a building that has no job to signal. |

### How the answer took its shape

The bake's light mask already decided where the team glow shines; it now
carries grey as well as white, so trim marked in the art with its own
colors glows at about 70% (the band) and 45% (the stripe) of a light,
over a dark base. Team colors are any color the player picks, so baked
color atlases could not have done it. Compared in play, blue trim stood
out clearly; the default green on grass is quieter but plain.
