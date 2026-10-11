*Closed record — see [../README.md](../README.md). Not spec.*

# Q44 — Amend Q37: what do `Shift` and *clear* do with a mark tool armed?

## Ruling

*Ruling (2026-10-11):* **`Shift`+drag fills a rectangle, and *clear*
withdraws a plan the team already holds.** Made under a new number because
the ruling it amends, [Q37](question-answered-0037.md), is a closed
record. Q37's `Shift`+click withdrew the armed kind, which was found in
play to be what nobody reaching for `Shift` expected; filling an area is,
and withdrawing moves to the entry that already meant "nothing here".

### The rules

1. **A `Shift`+drag fills a rectangle.** With a tool armed, a left press
   with `Shift` held anchors one corner; the drag moves the other, and
   the armed tool's ghost previews every tile between them. Release
   marks each of those tiles with the armed tool, row by row — nothing is
   submitted before. `Esc` before release disarms and fills nothing. A
   corner past the map's edge stops at the edge. `Shift` is read at the
   press: pressing or releasing it mid-drag changes nothing. Every kind
   fills, *clear* included.
2. **A plain drag is unchanged.** Without `Shift`, a press marks the tile
   and a drag every tile the cursor crosses, each once, as Q37 ruled.
3. **_Clear_ withdraws where the team holds a plan.** Marking a tile with
   a row's *clear* is an `Unmark` of that kind when the team already
   holds a plan of that kind there — a value or a clear alike — and
   otherwise a `Mark` with no value, the plan to empty the slot. The
   choice is made per tile, against the plans the sim last reported, so
   one stroke of *clear* can withdraw some tiles and plan others.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided entry
  and Marks. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T20](../../TASKS.md) — amended to build the rectangle and
  the withdrawing *clear* in place of `Shift`+click.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Rectangle on `Shift`; *clear* withdraws a held plan** *(chosen)* | One entry that means two things by context; bought: an area in one stroke, and withdrawing with no extra key or entry. |
| Rectangle on `Shift`; withdraw on right-click | Takes the right button from the orbit while a tool is armed. |
| Rectangle on `Shift`; a *withdraw* entry per row | A tenth paint entry, past the `1`–`9` the digits reach. |
| Rectangle on `Shift`; no withdraw | A mistaken plan can only be overwritten, never taken back. |
| Q37 as ruled | `Shift`+click withdrew, which read as nothing happening once a bot had already painted the tile. |

### How the answer took its shape

Q37 shipped with `Shift`+click as an `Unmark`, and trying it showed why it
surprised: a withdrawn paint plan on a tile a bot had already painted
changes nothing visible, because the paint stays. The player's expectation
of `Shift` was a filled area. A clear plan on a tile the team has planned
already says "not this", so letting it take the plan back costs no new
control; the cost is that a *clear* stroke over planned tiles withdraws
rather than plans emptying them, and a player who wants both strokes it
twice.

A submission takes `delay` ticks to reach the snapshot, so *clear* on a
tile planned within the last few ticks still sees no plan there and plans
the slot emptied. That is the renderer's submit-and-forget (Q15) showing,
not a new rule.
