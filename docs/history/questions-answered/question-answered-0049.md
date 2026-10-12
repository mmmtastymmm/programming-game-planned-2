*Closed record — see [../README.md](../README.md). Not spec.*

# Q49 — Where may paint and overlays be planned, and how does a paint plan over paint read?

## Ruling

*Ruling (2026-10-11):* **paint and overlay plans go only on ground the
team has seen; and a paint plan over a tile already painted shows as a
solid square of the new color inside a border of the old.** Made under a
new number because [Q37](question-answered-0037.md), whose preview the
second half amends, is a closed record, and nothing before limited where
a paint or overlay plan could go.

### The rules

Rules 1 and 2 are the sim's and hash-affecting; rules 3 and 4 are the
renderer's.

1. **Only ground the team has seen.** A `Mark` of a paint or an overlay
   plan — a value or a clear — applies only on a tile whose terrain is
   `ground` and which the sending team has seen (visible or remembered,
   `03`). On rock, on water, or on a tile still unknown to the team, it is
   dropped on its tick. A building plan may go anywhere on the map, as
   before, and an `Unmark` always applies.
2. **It is checked on the tick it applies, not at submission.** What a
   team has seen changes over a match, so the check cannot be part of
   validation at submission, which refuses only what can never apply
   (`06`); it is the case `06` already names of a `Mark` on a tile the
   sender has no right to. The unknown test comes first, so the answer for
   an unseen tile never depends on what lies there: marking cannot scout.
3. **The renderer refuses first.** It asks the same rule before
   submitting, so a single mark on rock, water or fog is refused with the
   reason, a stroke or rectangle over them marks the rest and says how
   many it skipped, and the armed tool's ghost does not appear on a tile
   it cannot plan.
4. **A paint plan over paint is a solid inner square.** On a tile already
   painted, a paint plan's preview is the new color nearly opaque, in a
   square inset from the tile's edge, so the old paint shows round it:
   "green now, purple planned". On an unpainted tile it is the half-alpha
   ghost Q37 rules. A translucent ghost over paint mixed the two colors.

## Outcome

- **Docs:** [03-world.md](../../03-world.md) — the Decided section and
  the `plans` row. [06-architecture.md](../../06-architecture.md) — the
  command log's dropped commands. [07-interface.md](../../07-interface.md)
  — Marks. [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Seen ground only, checked on its tick** *(chosen)* | A mark on a tile seen only after submission but before its tick still applies, and one on a tile seen only after its tick is dropped; bought: no scouting by marking, and a rule every peer applies alike. |
| Refuse rock and water by true terrain at submission | Marking a fogged tile would reveal its terrain. |
| Refuse unknown tiles at submission | Validation would depend on what the team has seen, which changes; a replay, validated against the opening state, would reject marks that were valid on their tick. |
| Only the paint action refuses rock and water | Plans could sit on rock forever. |

### How the answer took its shape

The first build put the check in validation at submission. The golden
replay, which validates every command against the opening state, then
rejected its own tick-0 paint and overlay marks beside the start, which
the team had not yet seen: validation must refuse only what can never
apply. The check moved to the tick a mark applies, with the renderer
asking it first so the player is still told why; the golden scenario's
two marks moved to tick 2, after the team's first sight.
