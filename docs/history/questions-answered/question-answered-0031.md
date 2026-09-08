*Closed record — see [../README.md](../README.md). Not spec.*

# Q31 — Amend Q9 and Q30: printers cannot be built; a team starts with the map's and takes the rest

## Ruling

*Ruling (2026-09-08):* **no printer is ever built. The map places every
printer there will be, and a team holds the ones it started with, the ones
it takes, and no others.** This amends [Q9](question-answered-0009.md),
which let bots build printers, and [Q30](question-answered-0030.md), whose
progression had three ways to hold more printers; both files are closed
records, so this is a new number. Q22 and Q23 — buildings cost ore, prints
cost time — stand for every building that can be built, which is now every
model but the printer.

### The rules

Each is hash-affecting.

1. **`build` and `build_nearest` refuse the model `printer`**, as a
   `ValueError` before anything begins; a building plan naming `printer` is
   refused at submission (`06`). There is no printer-site, and the printer
   row in data has no cost and no build time.
2. **The map places every printer.** A map gives each team one or more
   starting tiles (`03`), and a printer stands on each at tick 0, on the
   team's `printer` deployment, with full health and an empty store. The
   number of printers in the world is fixed at load and only falls.
3. **A printer is lost by death or deconstruction and gained only by
   conversion** (Q30). Deconstructing a printer (Q28) is legal and
   permanent: it removes one from the world for everyone, which is a
   program's choice to deny rather than take.
4. **A team is out when it has no printer** (`04`): its last one died, was
   deconstructed, or was converted away. There is no site to keep a team
   alive, so the rule loses its site clause.
5. **The cap and the deployments** (Q24) are unchanged: ten bots and one
   color per printer held, rising and falling only with conversion and loss.

### Consequences

- **Progression is two things, not three**: keep the printers you have, and
  take a rival's. `05` says so.
- **Every match has a fixed number of printers**, so the whole game is a tug
  over them: the map author decides how many, and a map with more printers
  than teams has printers to race for.
- **The economy prices depots and whatever buildings follow**, never the
  objective. Ore buys storage and, later, defence; it never buys reach.
- **The Fool builds depots** and nothing else, and its bundle says so.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the model table, the
  `build` row, and the Q9, Q23 and Q30 bullets. [03-world.md](../../03-world.md)
  — the map's starting tiles. [04-opposition.md](../../04-opposition.md) —
  the ending rule. [05-progression.md](../../05-progression.md) — the
  Decided bullet and the table. [QUESTIONS.md](../../QUESTIONS.md) — the
  status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Printers can be built for ore, as Q9 ruled | A team that loses its printers can rebuild if it has a bot and ore, and a rich team can outgrow the map. Printer-sites, a cost row, and a second way to hold more. |
| **Printers are the map's; a team keeps, takes or loses them** *(chosen)* | The objective cannot be manufactured, so a match is a contest over a fixed set. A team that loses its last printer is out with no way back, which is the point. |
| Printers can be built, but only from a converted rival's | A middle path with a rule to explain; rejected as more mechanism than the game needs while it is starting small. |

### How the answer took its shape

The user's rule, and the reason is the one Q30 already gave: a printer is
the objective in every sense, and an objective a team can build is one it
can replace, which takes the edge off losing one. Fixing the count at the
map makes conversion the only way up and loss the only way down, and the
tug that `05` describes becomes the whole match.
