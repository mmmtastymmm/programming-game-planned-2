*Closed record — see [../README.md](../README.md). Not spec.*

# Q26 — Amend Q10: a tile has a paint, an overlay and a building, all at once

## Ruling

*Ruling (2026-09-08):* **for each team a tile carries a paint and an
overlay, independently, and a building on the tile disturbs neither; a
blueprint is an overlay whose label names a building model, and it is the
one mark a building consumes.** This replaces the three slots
[Q10](question-answered-0010.md) gave a tile — a blueprint, a paint and a
set of layers — with two, by folding the blueprint into the overlay, and it
says what a building does to them: nothing, except that placing a site on
a team's blueprint for that model takes the blueprint with it. Made under a
new number because Q10's file is a closed record; everything else in Q10 —
one command log, marks read by programs and never sensed, the player marks
the map and never commands a machine — stands.

### The rules

Each is hash-affecting.

1. **A tile's marks, per team, are two slots**: a **paint**, a color from
   the deployment color list or `None`; and an **overlay**, a label from the
   closed list `mark.overlays` in data or `None`. `Mark` sets one slot and
   leaves the other; `Unmark` clears one slot.
2. **Paint influences nothing.** It changes the tile's color for the team
   that painted it, and it is readable by that team's programs through
   `painted(color)` and the tile record, which is all it does.
3. **An overlay influences bots**, through the programs that read it: an
   overlay's label is whatever the program makes of it, read through
   `overlaid(label)` and the tile record. One label per building model is
   reserved — `"printer"`, `"depot"` — and an overlay with a model's name
   is a **blueprint** for that model.
4. **A blueprint becomes a building when a bot builds there.** `build` and
   `build_nearest` (`docs/02`) may target a tile carrying the team's
   blueprint for the model they name; `blueprints()` returns those tiles.
   Placing the site **consumes the blueprint** — that team's overlay slot
   on the tile becomes `None`, because the overlay turned into the building
   — and touches nothing else. A blueprint is an instruction to a program,
   never to a bot: a program that never reads blueprints builds nothing from
   them.
5. **A building disturbs no other mark.** Paint stays under a building,
   another team's blueprint stays, and any overlay that is not the building's
   own blueprint stays; a tile with a building may still be marked. A bot
   standing on a tile disturbs nothing either.
6. **Marks outlive what they refer to.** A blueprint on a tile that later
   becomes unbuildable — occupied, say — stays until built or unmarked; a
   program reading it decides what to do.

### Consequences

- **The tile record has two mark fields per team**, `paint` and `overlay`,
  and `03`'s table says so. `layered(label)` is renamed `overlaid(label)`.
- **A blueprint has nowhere to go but into a building**, which is the
  player's intent: draw the base, and the program fills it in.
- **Nothing is ever cleared by the world.** Only the player's `Unmark` and a
  bot's build change a mark, so what a player drew stays drawn.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — the Q10 bullet, held
  until `docs/06` exists. [02-machines.md](../../02-machines.md) — the mark
  builtins and `build`. [03-world.md](../../03-world.md) — the tile's mark
  fields. [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Three independent slots — blueprint, paint, layers — as Q10 ruled | The most a player can say about a tile, with a blueprint as a special thing rather than an overlay, and no rule for what a building does to them. |
| One mark per team — paint or overlay — and a building clears it | One field and one rule, and a player who wants a color and a label on one tile cannot have them; a building erases what the player drew. |
| **Paint and overlay independently, a building disturbs neither, a blueprint is consumed by its building** *(chosen)* | Two fields per team. The player keeps everything they drew, and the only mark the world ever removes is the one that became a building. |

### How the answer took its shape

The user's first model had a building exclude both marks, then reversed to
all three coexisting; this ruling is the second. What survived the reversal
is the fold of blueprint into overlay, and the one consumption that follows
from it: if a blueprint *is* the overlay, and the blueprint becomes the
building, then the overlay slot is empty once the site is placed — not
because a building clears marks, but because that particular mark is now a
building.
