*Closed record — see [../README.md](../README.md). Not spec.*

# Q43 — Amend Q35 and Q42: a bot's program sets its color, and every team is an outline the player colors

## Ruling

*Ruling (2026-10-11):* **a bot's body color is whatever its program last
set with `set_color`, and a team is shown by an outline around every
machine, in a color the player chooses per team — their own green by
default.** Made under a new number because
[Q35](question-answered-0035.md), which colored a body by its deployment
and a team by a ring, and [Q42](question-answered-0042.md), whose last rule
put every bot in its team's color, are closed records.

### The rules

Rules 1 to 3 are the sim's and hash-affecting; rules 4 to 6 are the
renderer's and reach no hash.

1. **`set_color(color)`** is a bot action of zero ticks, not noisy:
   `color` a color from the paint list in data, or `None`; anything else
   is a `ValueError`. Like `rename`, it is an action, so a busy bot cannot
   call it.
2. **`color` is a bot attribute**: a paint color or `None`, `None` when the
   bot is printed. It survives a redeploy and a fault, as `name` does, and
   is carried by sightings like every attribute — a rival sees it.
3. **Buildings have no `color`.** The attribute is absent on them, as
   `load` is.
4. **A bot's body is its `color`**, from the paint list; `None` draws in a
   neutral body color from `data/interface.toml`. A building's body is
   that neutral color too.
5. **Every machine has an outline in its team's outline color**, replacing
   Q35's ring. The player's own team defaults to green; every other team
   defaults to the next color of the outline list in
   `data/interface.toml`, in team order.
6. **The player may change any team's outline color**, their own
   included, from the inspector's team list, for colorblindness or taste.
   The choice is saved beside the programs in `colors.toml`, restored at
   start, and never reaches a peer.

### Consequences

- **The map shows what bots are doing, as the program says.** Q42 gave
  every bot one program, which left nothing on the map to tell a miner
  from a fighter; a program that colors its bots by job restores that, in
  the player's own terms.
- **A color is a signal both ways.** A rival's program can read it from a
  sighting, so honest colors inform the enemy and misleading ones are a
  tactic. Nothing stops either.
- **Friend and foe is an outline, and its colors are the player's.** With
  more than two teams, each keeps its own outline, so a three-way fight
  stays readable.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the Decided entry, the
  kind tree, the attributes and the `set_color` row.
  [07-interface.md](../../07-interface.md) — the Decided entry, the
  inspector row, Machines on the map and Costs.
  [00-overview.md](../../00-overview.md) — the Decided listing.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T25](../../TASKS.md) — the builtin, the attribute, its cost
  row, the renderer's body and outline, and the color settings. `⚠HASH`.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| who sets the color | A local setting, per team | No sim change, but one color per team says nothing a team outline does not, and a rival never sees it. |
| who sets the color | **The program, per bot** *(chosen)* | One attribute and one builtin in the hash; bought: the map shows what the player's own code says each bot is doing. |
| the palette | Any RGB value | A second color grammar to validate and hash. |
| the palette | **The paint list** *(chosen)* | One list for paint and bodies, already in data and already validated. |
| the team cue | Q35's ring, the player's white | A small cue, and fixed colors. |
| the team cue | Friendly green, enemy red | Unreadable to the commonest colorblindness, and every enemy alike. |
| the team cue | **An outline per team, colored by the player** *(chosen)* | Defaults to friendly green and a distinct color per rival; any of them changes for colorblindness or taste. |

### How the answer took its shape

The user proposed letting players set their bots' color, with a green
outline for friends and red for enemies. Red and green were flagged as the
pair the commonest colorblindness cannot tell apart, and a single enemy
color as unreadable with more than two teams; the user kept green as the
default for their own team and made every team's outline the player's to
choose. Of setting the color locally or by program, the user chose a
function, which made it the bot's state.
