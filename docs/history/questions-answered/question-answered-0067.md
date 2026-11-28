*Closed record — see [../README.md](../README.md). Not spec.*

# Q67 — How is water taken off the map, moved, stored, used and put back?

## Ruling

*Ruling (2026-11-28):* **by a team's buildings, joined into networks as
power's are into grids: each building adds to its network or takes from
it, pumps lift water off the map into it, tanks hold it, spouts pour it
back, and a building the network cannot cover is dry and stops.** Made
under a new number because [Q65](question-answered-0065.md), which left
it open, is a closed record, and opened and answered in one commit.

### The rules

All of them are the sim's and are hash-affecting.

1. **A need, in data.** Every building model has a need for water a
   tick, which may be nothing, and a printer more while it prints. The
   numbers are tuned as the game is played; only the printer's is above
   zero to begin with.
2. **It is the map's water.** A pump lifts parts off a tile, a tank holds
   parts, a spout puts parts on a tile, and what a building uses goes to
   the ground water. The tiles, the ground water and the tanks together
   hold a fixed amount.
3. **Networks.** A team's buildings that touch, or are joined by its
   pipe or its aqueducts, are a network, labelled by its oldest
   building. It is traced as a grid is, from other joins: wire and a
   pylon's span join only a grid, pipe and an aqueduct's span only a
   network.
4. **Served oldest first.** Needs first, each in full or not at all,
   from what the pumps can lift that tick and then from the tanks; then
   the spouts that are pouring; then the tanks are filled by what the
   pumps can still lift. A pump lifts only what something takes.
5. **Dry.** A building whose need is not covered is dry until the next
   network step, and stops exactly as a dark one does. Being dry and
   being dark are apart.
6. **A pump** is a building on dry ground that lifts from the wet tile
   beside it holding the most water, up to its rate, only while it is
   powered and not flooded.
7. **A tank** is a building that holds water, and a printer has a small
   one, as it has a battery. Every tank starts empty.
8. **A spout** is a building that pours onto the tile beside it in the
   direction it faces, which its program sets and which is none when it
   is built.
9. **A pipe** is underground and an **aqueduct** overhead, as a wire and
   a pylon are: the one a sixth plan kind laid by the team's own bots,
   the other a building rivals may destroy.
10. **A well.** A printer covers its own need from a well, which moves
    no water, so a team that lays no pipe still prints. Nothing in data
    holds the well up: bringing it down is how pumps come to matter.
11. **No height.** A network moves water whatever the height of what it
    joins.
12. **Bots carry no water.**

### Consequences

- **Water becomes a second thing to supply**, shaped like the first, so
  a player who has learned grids has learned networks.
- **A lake can be moved.** A pump beside one, pipe, and a spout
  elsewhere put its water wherever the team builds, uphill included:
  a moat filled, a hollow flooded, a rival's printer drowned.
- **A pond can be pumped dry**, since what a tank holds is no longer on
  the map.
- **Water needs power**, through the pump, and so a flood that darkens a
  pump stops the water as well.
- **Nothing changes until the numbers do.** With the printer's well
  covering its need and no other need above zero, a match with no pump
  plays as it did.

## Outcome

- **Docs:** [09-water.md](../../09-water.md) — a Q67 Decided entry, the
  Q65 entry's last sentence, The water network and its six parts, where
  the water step stands, what a program reads, and what it leaves open.
  [02-machines.md](../../02-machines.md) — a Decided entry that points
  at `09` for the rows, the model lists and `plans`.
  [03-world.md](../../03-world.md) — the pipe plan, a tile's `pipes` and
  its six plans. [06-architecture.md](../../06-architecture.md) — steps
  3, 5, 6, 7 and 8, validation, the snapshot and the hash.
  [07-interface.md](../../07-interface.md) — a Decided entry.
  [08-power.md](../../08-power.md) — a dry building, beside a dark one.
  [00-overview.md](../../00-overview.md) — the Decided listing and the
  table of docs. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Problem:** [P12](../../PROBLEMS.md) — none of it is built.
- **Task:** [T48](../../TASKS.md), [T49](../../TASKS.md) and
  [T50](../../TASKS.md) — the network in `sim`, in `render`, and in the
  starter library.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| who needs water | The printer alone, while it prints | Simple, and a rule about one model. |
| who needs water | **Every model has a figure in data** *(chosen)* | More numbers to tune, and none of them known yet. |
| what the water is | A separate quantity, made by pumps | The total of a match is no longer fixed, and a pump beside a puddle is as good as one beside a lake. |
| what the water is | **The map's own parts** *(chosen)* | Tanks join the tiles and the ground water in what must sum. |
| what carries it | Power's wires and pylons, for both | Nothing new to build or lay; water running along a pylon's line. |
| what carries it | **Pipes and aqueducts of its own** *(chosen)* | A sixth plan kind, a model, and a second network to trace. |
| a building without water | Only its work stops | A second kind of stopped to learn. |
| a building without water | **It stops as a dark one does** *(chosen)* | A dry building sees nothing, which is harsh for want of water. |
| where a spout pours | The lowest tile beside it | No state; the player cannot aim it. |
| where a spout pours | **The way it faces, set by its program** *(chosen)* | A new builtin, and a spout does nothing until a program says. |
| a start with no water near | Pumps or nothing | A team far from a lake never prints. |
| a start with no water near | **A well that covers the printer's own need** *(chosen)* | Water asks nothing of anyone until the well is tuned down. |
| height | Pipes that lift only so far, or need a pump a level | A design of its own. |
| height | **Pipes take no account of it** *(chosen)* | A lake goes uphill for the price of a pump's power. |

### How the answer took its shape

The user had the outline before the question had a number: buildings
that consume water, a network, pumps beside water, storage. Height and
water on the ground came first, as Q65 and Q66, because pumping needed
somewhere to pump from. Asked the seven things that outline left open,
the user chose a need on every building over the printer alone, to be
tuned as the game is played; the map's own water over a separate one;
pipes of their own, under the ground or over it as power's lines are;
a pump that needs power and is a net producer; tanks mirroring
batteries; water that can be put back on the map; and no water carried
by bots. Five more followed from those. The user agreed that a dry
building stops; replaced the outlet that poured on the lowest tile
beside it with a **spout**, a building with a direction it pours in;
kept power's order of serving; agreed to the printer's well for now;
and set height aside. The names are the ones proposed, with the spout
the user's.

Three things here are the writer's and not yet the user's: that the
spout's direction is set by its program, with `pour`, and is none when
it is built; that a well moves no water, where the first sketch drew it
from the ground water, which a map that begins with none would have
left every printer dry on; and the order within a network's turn —
needs, then spouts, then tanks.
