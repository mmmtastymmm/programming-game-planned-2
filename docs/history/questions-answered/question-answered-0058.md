*Closed record — see [../README.md](../README.md). Not spec.*

# Q58 — How does power work: solar panels, wires to a printer, and bots that stop when a team is short (amending Q10, Q27, Q37, Q42 and Q49)

## Ruling

*Ruling (2026-10-15):* **power is a team's flow, tick by tick: printers
and solar panels supply it, bots draw it, a building supplies only when
wired back to one of the team's printers, and when the draw outruns the
supply the newest bots stop.** Made under a new number because every
ruling it amends is a closed record: [Q10](question-answered-0010.md) and
[Q27](question-answered-0027.md) on the plans a player may mark,
[Q37](question-answered-0037.md) on the mark strip,
[Q42](question-answered-0042.md) on deployments and printing, and
[Q49](question-answered-0049.md) on where paint and overlays may be
planned, which now covers wires.

### The rules

Rules 1 to 10 are the sim's and hash-affecting; rule 11 is the renderer's
and reaches no hash.

1. **Power is a flow, not a resource; bots draw it and buildings supply
   it or nothing.** The bot model has a `power_draw`; every building model
   a `power_supply`, above zero for the printer and the solar panel, zero
   for the depot and the site. Nothing hauls, holds or spends power, and
   nothing keeps it from one tick to the next. The sim refuses data whose
   printer supply is below one bot's draw, and data keeps a printer's
   supply below what `bots_per_printer` bots draw, so panels matter.
2. **The grid.** A team's **conductors** are the tiles carrying its wire
   and the tiles of its buildings, sites excluded. Its **grid** is every
   conductor joined to a tile of one of its printers through adjacent
   conductors. A building is **on the grid** when its tile is, so a
   printer always is. Bots need no grid.
3. **Supply** is the sum of `power_supply` over the team's buildings on
   the grid whose health is above zero.
4. **Who is powered.** The team's bots are taken in ascending entity id,
   oldest first, and each is powered while the draw so far, its own
   included, is within the supply; the first that does not fit and every
   newer one are not. Since a standing printer's supply covers one bot,
   the oldest bot always runs while the team stands.
5. **An unpowered bot stops.** Its slice opens with its budget, which pays
   down any deficit as on any tick, but main flow does not run on it and
   the action main flow began does not count down. Power never withholds a
   handler: interrupts are delivered and their hooks run as always, so
   `dying` still ends in `death` and a redeploy still swaps.
6. **Printing waits for spare power** (amending Q42). A team's **spare**
   is its supply, minus every bot's draw, powered or not, minus `bot`'s
   draw for each print in progress, so no print begins while any bot is
   dark. A print begins only while the spare covers one more bot's draw,
   as well as while Q42's shortfall is above zero.
7. **Wires are a team's, and only its own bots touch them.** A tile's
   `wires` is the set of teams with a wire on it, on `ground` only, with or
   without a deposit or a building. A wire carries only its team's power.
   Every team sees a wire and remembers it, as it does paint, but only the
   wire's own team lays or removes it, and a team that is out has its
   wires cleared with its machines.
8. **`wire(x, y)` and `unwire(x, y)`** are bot actions on an adjacent
   tile, each re-checked when it completes. `wire` lays the team's wire on
   ground without one, taking `wire_cost` from the bot's load; `unwire`
   removes it, refunding nothing. A **wire plan**, the fourth plan kind
   (amending Q10 and Q27), marks a tile for either; like paint, it goes
   only on ground the team has seen (amending Q49), and a plan asking for
   what the tile already has is dropped, while the action that makes the
   tile match a plan consumes it, so every wire plan can be carried out.
9. **The solar panel**, model `solar`, is a building like the depot: a
   site priced in ore, then `build_ticks`, with health, no store, and a
   constant `power_supply`. It adds the deployment `"solar"`, so a team has
   four: `"bot"`, `"printer"`, `"depot"` and `"solar"` (amending Q42).
10. **What a program reads, and where it runs.** Every bot has the
    attribute `powered`, and every building `on_grid`; a tile's record
    carries `wires`; `power()` returns the supply, demand and spare the
    last power step kept, as team state; `wires()` lists the tiles
    carrying the team's wire. The tick's step 6 becomes *power and
    printing*: each team's grid is traced and its bots powered, then its
    printers begin prints and its figures are kept. The match is built
    with that step's power half run at tick 0, so every figure and
    attribute exists from the first slice. The tiles' wires, the new
    attributes and each team's figures are in the state hash and the
    snapshot, with each of the team's own wire tiles marked on or off its
    grid.
11. **On the map** (amending Q37): a wire is a bar across its tile in its
    team's color with a line to each of the team's wires and buildings
    beside it; the mark strip gains a wire tool with its *clear*, on `C`;
    a dark bot and an off-grid building carry a small mark, their lights
    keeping their team's color; and the inspector shows the player's own
    team's supply, demand and spare.

### Consequences

- **Power is a second limit on bots, under the first.** Q24's cap still
  counts printers; a team also prints no bot it cannot power, so panels
  let a team fill a cap its printers' own power does not, and never raise
  it.
- **No team locks itself.** Only bots draw, and a standing printer powers
  at least the oldest one, so a team short of power still has a bot that
  can build a panel or lay a wire. A building that draws is left to a
  later question, which must keep this true.
- **Wires make placement a decision.** A panel may stand anywhere, but it
  counts only once a line of wire, or of the team's buildings, reaches it
  from a printer, and losing the printer at the root drops what hangs from
  it.
- **No sabotage of wires.** Rivals cannot cut a line; they cut power by
  attacking panels and printers, which have health. A line can still be
  cut off by the team's own `unwire` or by a building on its route being
  destroyed.
- **A shortfall is visible and ordered.** The newest bots stop first, so
  a team's oldest keep running, and a program can read who is powered and
  why.
- **A stopped bot is still a body.** It keeps its tile, its load and its
  health, can be attacked, and still dies; it simply does not think or
  act until power returns.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the Q22, Q42 and new
  Q58 Decided entries, A machine's deployments, the kind tree and table,
  the attributes, the senses, the actions, Printing, a new Power section
  and what it leaves open. [03-world.md](../../03-world.md) — the Q49
  bullet, the tile's fields and its slots.
  [01-language/execution.md](../../01-language/execution.md) — a dark
  tick's budget in Metering. [06-architecture.md](../../06-architecture.md)
  — the Q10 entry, validation at submission, the tick's steps 2, 3, 5 and
  6, the snapshot and the state hash.
  [07-interface.md](../../07-interface.md) — the Q37 entry and a new Q58
  entry, the screen's inspector and tools, the program tree, Machines on
  the map and Marks. [05-progression.md](../../05-progression.md) — power
  beside printers. [00-overview.md](../../00-overview.md) — the Decided
  listing. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T31](../../TASKS.md) — power in the sim: the grid, supply,
  powered bots, wires and their actions, the solar panel, the senses, the
  snapshot, the data and its load check, the cost rows, the starter's
  programs and library, and the fixtures.
- **Task:** [T32](../../TASKS.md) — power in the renderer: wires, the wire
  tool on `C`, the unpowered mark, the panel's art and the inspector's
  figures.
- **Problem:** [P8](../../PROBLEMS.md) — the docs state Q58 as current
  until T31 and T32 land.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| what power is for | **A budget bots and buildings share** *(chosen)* | One number to manage; a shortfall needs a rule for who loses out. |
| who draws | Bots and some buildings | A depot drawing ahead of newer bots can starve every bot, and a dark bot cannot remove the depot: a team frozen but not out. |
| who draws | **Bots only; buildings supply or nothing** *(chosen in review)* | No lock: a standing printer powers the oldest bot. A building that draws waits for a question that keeps this true. |
| what power is for | Bots carry batteries | The most programming-heavy option, and every program must handle it. |
| the pool | **Wired back to a printer** *(chosen)* | One team pool and wires that matter: a far panel must be connected. |
| the pool | Each network separate | Richer topology, but bots have no network to draw from. |
| a shortfall | Brownout: everyone slows by the same fraction | Nothing stops dead; fractional budgets in every machine. |
| a shortfall | **Unpowered machines stop** *(chosen)* | Plain to see and to program against; needs an order. |
| a shortfall | Actions cost power | Compute untouched; bookkeeping on every action. |
| the order | **Newest first** *(chosen)* | Deterministic, needs no input; the oldest bots are the last to go dark. |
| the order | Bots before buildings | Freezes the field to keep the base running. |
| the order | A player-set priority | Most control; one more command and window. |
| first power | **Printers generate some** *(chosen)* | A team starts alive with no panels; losing a printer costs power too. |
| first power | The map places panels | All power is panels; every map tunes the start. |
| printing short | **Only with spare power** *(chosen)* | Power becomes a second cap; printing never makes a shortfall. |
| printing short | Print anyway; the bot stays dark | A simpler rule; printers waste time making frozen bots. |
| wires and rivals | Rivals may cut them | Sabotage by line-cutting. |
| wires and rivals | **The team's own, untouchable** *(chosen)* | Power is an economy problem; rivals strike panels and printers instead. |
| wires and rivals | Shared like paint | Rivals could share or steal power through a common line. |
| the panel | **A building** *(chosen)* | Health, a tile, a site and a deployment, like the depot. |
| the panel | A ground fixture | Cheaper, but a machine with no program and no health. |
| handlers when stopped | Withheld too | A `dying` machine would never reach `death`, since `dying` freezes its health. |
| handlers when stopped | **Always run** *(chosen)* | Only main flow stops, so death and redeploys work unchanged. |
| a wire's price | Free, like paint | Lines cost nothing and stop being a decision. |
| a wire's price | **A little ore from the bot's load** *(chosen)* | Routes have a price and need no site to haul to. |
| a dark machine on the map | Its lights go dark | Clear, but a depot's or panel's lamp is its only team marking, so friend and foe blur. |
| a dark machine on the map | **A small mark over it** *(chosen in review)* | Team colors stay; the mark says power. |

### How the answer took its shape

The user set out to design water and electric systems and the resources
behind them. A list of metals, fuels and generators was weighed first, and
the user chose to start with power alone: a budget that bots and buildings
share, wires that bots build in the ground, and solar panels that can stand
anywhere. Asked how a shared budget and wires fit together, the user chose
a pool fed by everything wired back to a printer; for a shortfall, machines
that stop, the newest first; printers as the first source; printing only
with spare power; wires only their own team may touch; and the panel as a
building. That handlers still run on a stopped machine, and that a wire
costs a little ore from the bot's load, were the drafting's calls, made so
that `dying` still ends in `death` and so that a line of wire is a
decision; the wire plan followed from Q10's rule that the player marks and
bots act. The ruling's number moved from Q56 to Q58 when `main` took Q56
and Q57 while it was open. Review then found the first draft's depots
drawing power could freeze a team that was not out, so only bots draw; that
the deploy check, the snapshot and an out team's wires had not caught up;
that a wire plan could ask for what a tile already had and never be carried
out; that a dark tick's budget, the first tick's figures and `power()`'s
spare were undefined; that dark lights hid a building's team; that `W`
already panned the camera; and that the starter library laid no wire.
