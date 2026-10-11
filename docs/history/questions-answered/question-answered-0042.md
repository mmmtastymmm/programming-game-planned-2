*Closed record — see [../README.md](../README.md). Not spec.*

# Q42 — Amend Q9, Q23, Q24, Q30, Q31, Q35 and Q40: printers are built at a rising price and print by themselves, nothing converts, and a team has one deployment per model

## Ruling

*Ruling (2026-10-11):* **bots build printers, each one dearer than the
last by how many the team has; printers print by themselves, taking turns,
until the team is at its cap; no printer ever changes hands; a team with no
printer at the end of a tick is out; and a team has one deployment per
machine model — `bot`, `printer`, `depot` — and no others.** Made under a
new number because every ruling it amends is a closed record:
[Q9](question-answered-0009.md) and [Q31](question-answered-0031.md) on who
makes printers, [Q23](question-answered-0023.md) on what the economy
prices, [Q24](question-answered-0024.md) and
[Q40](question-answered-0040.md) on deployments per printer,
[Q30](question-answered-0030.md) on conversion, and
[Q35](question-answered-0035.md) on a bot's color.

### The rules

Each is hash-affecting except the last sentence of rule 9, which is the
renderer's.

1. **Printers are built.** `build` and `build_nearest` accept the model
   `printer`, and a building plan may name it. The printer gains a `cost`
   and a `build_ticks` in data like the depot.
2. **The price rises with what the team has.** A site's store capacity is
   `cost[model] + n × cost_step[model]`, per resource kind, where `n` is the
   number of the team's machines of that model — standing buildings and
   sites alike — at the moment the site is placed, the new site not
   counted. Every buildable model has a `cost_step` in data; the depot's is
   zero. The capacity is fixed when the site is placed and never revisited.
   The count is of what stands **now**, not of what was ever built, so a
   team that loses printers pays less for the next one: the comeback is the
   rule's, by design. The curve is linear to start; another shape is a data
   change only if it stays `base + n × step`, and a new question number
   otherwise.
3. **The map still places the first printers** (`03`): one on each of a
   team's starting tiles at tick 0. They count toward `n` like any other.
4. **The cap is unchanged** (Q24): `bots_per_printer` bots per standing
   printer, sites excluded; losing a printer lowers it and bots above it
   are kept.
5. **Printing is automatic, and printers take turns.** There is no `print`
   action. In a new tick step after out teams and before regrowth (`06`,
   step 6), each team's **shortfall** is its cap minus its bots minus its
   prints in progress. While the shortfall is above zero and the team has an
   idle printer, one begins a print: the idle printer with the least entity
   id greater than the team's **print turn**, or failing one, the least
   idle id at all. The print turn becomes that printer's id. A printer is
   idle when it is not printing and not `dying`. The print turn is team
   state, in the hash, and starts as no printer.
6. **A print belongs to its printer once begun.** A printer's `printing`
   attribute is `print_ticks` when it begins, falls by one in step 3 of each
   later tick, and at zero the print completes there: the bot appears on
   the first free tile of `n`, `e`, `s`, `w`, or the print is cancelled with
   nothing made if none is free. A printer that dies, goes `dying`, or is
   deconstructed loses its print. Either way the shortfall is simply
   recomputed in the next print step; nothing is queued and nothing is
   redistributed.
7. **Nothing converts.** `convert`, `convert_ticks`, its sound and Q30's
   same-tick tie rule are withdrawn. A printer still deals
   `defence_damage` to every adjacent bot of another team each tick, and
   any team's bot may still deconstruct any printer (Q28).
8. **A team is out at the end of any tick it has no printer** (`04`): its
   last one died or was deconstructed. A printer site, bots and ore
   do not keep it in the match.
9. **One deployment per model.** A team's deployments are `"bot"`,
   `"printer"` and `"depot"`, each unlocked from the start and never
   locked; Q24's deployment per printer and Q40's numbers are withdrawn.
   Every bot runs the `bot` bundle, its `deployment` attribute is `"bot"`,
   and every machine is named its model plus its entity id, bots included
   (`bot17`). A `Deploy` naming any other deployment is refused at
   submission. The renderer draws every machine's body in its team's ring
   color, bots as buildings already were, and draws no number on a bot
   (amending Q35 and Q40's badge).

### Consequences

- **Progression is building printers** (`05`): reach is bought with ore,
  each printer dearer than the last, and lost to a rival's attacks and
  deconstructions, never to a capture.
- **The objective can be rebuilt, but not from nothing.** Q31's worry —
  that a buildable objective takes the edge off losing one — is answered by
  rule 8 instead of by a fixed count: a team may replace any printer but
  its last.
- **A printer's program decides nothing about printing.** It senses, logs
  and waits; printing is a building's rule, as its defence already was.
- **The shipped programs change shape**: `robots/1.py` becomes
  `robots/bot.py`, and a printer program that calls `print` has nothing to
  call. That is the task's, below.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the Decided entries,
  A machine, Deployments, the kind tree and table, the attributes, the
  sounds, the actions, the damage paragraph and the interrupt table, the
  game `print`. [03-world.md](../../03-world.md) — the starting tiles.
  [04-opposition.md](../../04-opposition.md) — Ending and Why the printer.
  [05-progression.md](../../05-progression.md) — rewritten for building.
  [06-architecture.md](../../06-architecture.md) — the Decided entry, the
  tick's new print step, the command log, the snapshot and the hash.
  [07-interface.md](../../07-interface.md) — the Decided entry, the tree,
  the script's `bundle`, Machines on the map and the mark strip.
  [01-language/syntax.md](../../01-language/syntax.md) — the game `print`.
  [00-overview.md](../../00-overview.md) — the Decided listing and the
  table. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T24](../../TASKS.md) — the sim, the data, the language's
  builtin and cost rows, the shipped programs and scripts, the renderer and
  the fixtures. `⚠HASH`.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| who makes printers | The map only, as Q31 had | A fixed set to fight over; reach can only be taken, and a map's printer count is the whole game. |
| who makes printers | **Bots, at a rising price** *(chosen)* | Reach is bought with ore, so the economy prices the objective; the rising price keeps one rich team from outgrowing the map linearly. |
| what `n` counts | Printers only | Five sites placed at once all pay the first price. |
| what `n` counts | **Printers and printer sites** *(chosen)* | One more model count; the price cannot be dodged by building in parallel. |
| what `n` counts | Plans too | Painting a plan would raise the team's own price, and a plan is private and never physical. |
| present or ever | Ever built | Losing a printer is permanent in price as well as in reach. |
| present or ever | **Present** *(chosen)* | Losing a printer makes the next cheaper — a comeback rule, on purpose. |
| the curve | Geometric | Caps growth harder; a fixed-point power to specify. |
| the curve | **Linear** *(chosen)* | Two numbers per kind; total spend grows with the square of the count. |
| who prints | Printer programs call `print(n)` | A choice with nothing to decide: prints cost only time, so a program that does not print is a mistake. |
| who prints | A team queue filled by `enqueue(k)`, redistributed across printers | Program control over a decision that is not one, plus a queue to hash and redistribute on every printer change. |
| who prints | **Automatic, by turns, to the shortfall** *(chosen)* | One counter per team; the player's lever is how many printers to build and where. |
| conversion | Keep Q30 | Two ways to gain printers, one of them free. |
| conversion | **Withdraw** *(chosen)* | Printers are built or lost; defence and deconstruction stay. |
| the end | No printer and no printer site | With the price lowest at zero, a team with ore and one bot is almost never out. |
| the end | **No printer at the end of a tick** *(chosen)* | Strict, and it keeps the last printer the thing a match is about. |
| deployments | One per printer, numbered (Q40) | Unbounded once printers are buildable, and each new printer would open a program slot. |
| deployments | **One per model** *(chosen)* | Every bot is one program, which is Q2's fleet in its plainest form; a bot's color carries the team only. |

### How the answer took its shape

The user found the printer rules did not make sense as a game and set out
four changes: printers buildable at a price rising with the count, each
raising the cap, no stealing, and the end at zero printers. A team queue
spread across printers by the count was the first form of printing; once
printing was automatic, a queue held nothing a shortfall did not, so it
became printers taking turns. One deployment per model followed from
buildable printers: a deployment per printer would grow without bound.
The bot's color going to the team's is this record's consequence, made so
Q40's badge does not outlive the numbers it showed.
