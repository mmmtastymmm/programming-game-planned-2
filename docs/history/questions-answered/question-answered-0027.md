*Closed record — see [../README.md](../README.md). Not spec.*

# Q27 — Amend Q26: everything the player places is a plan a bot must realise

## Ruling

*Ruling (2026-09-08):* **the player places plans, and only a bot makes them
real.** A `Mark` no longer paints a tile or lays an overlay; it records a
**plan** — for a paint, an overlay or a building — that a bot must travel to
and carry out with an action. A paint or an overlay is on a tile only
because a bot painted or laid it, exactly as a building is there only
because a bot built it. This amends [Q26](question-answered-0026.md), whose
paint and overlay took effect on command, and is made under a new number
because Q26's file is a closed record. Q26's two realised slots per team,
the fold of blueprint into overlay, and "a building disturbs no mark" all
stand.

### The rules

Each is hash-affecting.

1. **Three plans per team per tile**: a paint plan, an overlay plan and a
   building plan, each `None` or a value — a color, a label, a model. `Mark`
   sets one plan; `Unmark` clears one plan. A plan is the player's request
   and changes nothing about the tile until realised.
2. **A plan's value may be "clear."** `Mark(tile, "paint", None)` plans the
   removal of the tile's paint; a bot realising it removes the paint. So the
   only way a realised mark ever changes is a bot carrying out a plan, and
   the player's `Unmark` withdraws a request, never a result.
3. **Three actions realise them** (`docs/02`): `paint(x, y)` and
   `overlay(x, y)`, by a bot on an adjacent tile, take `paint_ticks` and
   `overlay_ticks`, cost no resources, and on completion set the realised
   slot to the plan's value and clear the plan; and `build` and
   `build_nearest`, as before, which consume the building plan when the site
   is placed. A `paint` or `overlay` on a tile with no such plan is a
   `ValueError`. A bot cannot invent a mark: no action sets a realised slot
   to anything but a plan's value.
4. **Realised paint influences nothing** and realised overlay influences
   bots through the programs that read it, as Q26 ruled; the reads are
   `painted(color)`, `overlaid(label)` and the tile record's `paint` and
   `overlay`.
5. **Plans are read through `plans(kind)`**, which returns the team's pending
   plans of one kind — `"paint"`, `"overlay"` or `"building"` — as
   `(x, y, value)` tuples sorted by row then column, and through the tile
   record's `plans`. `blueprints()` is `plans("building")` and is retired as
   a name.
6. **Plans and realised marks are per team, visible to that team only, never
   sensed or remembered, and world state** — Q10's rules, unchanged.
7. **A plan on a tile that cannot take it** — a building plan on rock, a
   paint plan off the map — is refused at submission for the off-map case and
   accepted otherwise: a program reading a plan it cannot carry out decides
   what to do, and a `paint` or `build` that fails its preconditions is a
   `ValueError` like any action.

### Consequences

- **The player's only effect on the world is through programs.** A plan is a
  request; a program that ignores plans changes nothing. This is Q3's line
  in its final form: the player authors programs and requests, and a machine
  does everything.
- **`docs/02` gains two actions and one builtin**, and its mermaid tree
  grows; `03`'s tile carries plans beside its realised marks.
- **Painting is work**, so a painted base is a base a bot walked, which is
  what makes paint worth reading.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — the Q10 bullet, held
  until `docs/06` exists. [02-machines.md](../../02-machines.md) — the
  actions, the reads, the tree. [03-world.md](../../03-world.md) — the
  tile's plans. [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Marks take effect on command, as Q26 ruled | A paint or an overlay appears where the player clicks, instantly, anywhere on the map, fog included. The map is the one thing the player touches directly. |
| **Marks are plans; a bot realises each with an action** *(chosen)* | Two more actions and their durations, a plan slot beside each realised slot, and a base that cannot be painted faster than a bot walks. The player touches nothing directly. |
| Plans for buildings only; paint and overlay on command | Q26 as ruled, with the building plan named a plan. Inconsistent: one kind of mark is work and two are free. |

### How the answer took its shape

Q26 had already made a building the realisation of a plan, and the user
asked why paint and overlay should be different. They should not: if the
game's premise is that the player never touches the world except through a
program, a click that paints a tile is a touch. Making every mark a plan
removes the last one, and it gives paint a cost — a bot's time — which is
what makes it a signal rather than decoration.
