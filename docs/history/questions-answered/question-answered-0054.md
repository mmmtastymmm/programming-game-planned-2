*Closed record — see [../README.md](../README.md). Not spec.*

# Q54 — Amend Q50: should a paint plan's square be opaque?

## Ruling

*Ruling (2026-10-11):* **yes: a paint plan's square is drawn fully
opaque, a little higher above the paint.** Made under a new number because
the ruling it amends, [Q50](question-answered-0050.md), is a closed record:
its rule 1 drew the square nearly opaque, which left it translucent.

### The rules

All are the renderer's and reach no hash.

1. **The square is opaque**, still unlit, in the new color, inset from
   the tile's edge as Q50 rules.
2. **It sits clear of the paint** by a margin the depth buffer always
   resolves, and the hover and selection markers sit above it, so a
   planned tile still lights when hovered.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — Marks.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Opaque square** *(chosen)* | The square no longer shows a trace of the ground under it, which at 92% alpha it barely did; bought: it writes depth, so the translucent paint below can never draw over it. |
| Bias the square's sort order | Tried first; the rows still flickered. |
| Keep it nearly opaque (Q50) | Two translucent layers a hair apart drew in either order. |

### How the answer took its shape

Seen in play: a paint plan in teal over a large yellow area flickered,
whole rows of tiles switching each frame between the plan's color and a
pale mix with the yellow drawn over it. A three-tile test did not show
it; painting most of the screen and planning over it did, with the old
build. Biasing the square's sort distance did not stop it; making it
opaque did, over eight consecutive frames.
