*Closed record — see [../README.md](../README.md). Not spec.*

# Q40 — Amend Q24 and Q35: bot deployments are numbers, one per printer without bound; the color is derived and every bot wears its number

## Ruling

*Ruling (2026-09-13):* **a team's bot deployments are `1`, `2`, `3`, …,
one per printer it holds, with no bound but the printers on the map.** The
color list in data no longer names them: deployment `n` is *drawn* in the
`n`-th paint color, cycling, and every bot wears its number. Made under a
new number because [Q24](question-answered-0024.md), which named
deployments from the color list, and [Q35](question-answered-0035.md),
which colored a bot by its deployment's name, are closed records. A map
with more printers than colors was the case Q24 left out: its ninth
printer raised the cap and unlocked nothing.

### The rules

Each is hash-affecting.

1. **Numbering.** The first printer a team holds unlocks deployment `1`,
   the `n`-th unlocks `n`; losing a printer locks the highest unlocked, as
   Q24 ruled for the last color. Building deployments stay `"printer"` and
   `"depot"`. There is no eighth-printer limit: a team with twelve printers
   has twelve bot deployments.
2. **In the language a bot deployment is a `num`:** `print(1)`; the
   `deployment` attribute is `1` for a bot, the model's name for a
   building, `None` for a site. In the command log and the slot table it
   is the decimal text `"1"`, which is what a `Deploy` names and what the
   programs directory's subdirectory is called. A deploy to any positive
   integer is accepted and waits if that deployment is locked, as Q24
   ruled; there is no "unknown deployment" among the numbers.
3. **The color is derived, not named.** The paint list in
   `data/machines.toml` keeps its two jobs — the paint values a plan may
   name, and the cycle a bot's color is read from — and deployment `n`
   wears the `((n − 1) mod len) + 1`-th color. This is the renderer's rule
   (`07`); the sim knows no color for a deployment.
4. **Every bot wears its number**, drawn by the renderer over its body,
   always: `1` on the first deployment's bots, `9` on the ninth's, whose
   body is the first color again.
5. **Names.** A bot is named `<deployment>-<id>` — `2-17` is deployment
   `2`'s bot with entity id 17 — since `color` plus `id` would read `217`.
   Buildings keep `<model><id>`.
6. **The snapshot lists the team's unlocked bot deployments and any
   holding or awaiting a bundle**, so the editor shows what can be written
   for, and a *next* tab opens the number after the highest.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the Decided entry,
  Deployments, the attributes, the `print` row, the printer row.
  [03-world.md](../../03-world.md) — the paint row's wording.
  [07-interface.md](../../07-interface.md) — the Decided entry and Machines
  on the map. [00-overview.md](../../00-overview.md) — the Decided listing.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
  [data/machines.toml](../../../data/machines.toml) — the list's comment.
- **Task:** [T22](../../TASKS.md) — the sim, the shipped programs, the
  fixtures and the badge. `⚠HASH`.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| past the eighth | Cap at the colors: extra printers raise the cap only | The simplest, and a twelfth printer is worth ten bots and no new program; the map's biggest prize stops changing what the player can write. |
| past the eighth | Colors then cycles: `red`, …, `red2` | Today's programs stay valid, but a name that is a color and a number at once is two rules in one string, and `red2` is not the second red anything to a reader. |
| past the eighth | **Numbers** *(chosen)* | Every shipped program changes from `print("red")` to `print(1)`, once; bought: one rule, no bound, and a name that says its place in the order. |
| the badge | The cycle, past the first | A clean map until a team is large — and a `9` that reads `1` until the player counts. |
| the badge | **The index, always** *(chosen)* | Redundant with the color for the first eight, and never ambiguous. |
| the name | `color` plus `id` as Q24 had | `1` plus `17` is `117`. |
| the name | **`n-id`** *(chosen)* | One character, and both halves readable. |

### How the answer took its shape

The first map has two printers, so the case never showed; it was found by
asking what the tenth printer on a larger map would do. The color list's
paint job is untouched — paint is a tile's, not a deployment's (Q28) — so
nothing else in the sim moves.
