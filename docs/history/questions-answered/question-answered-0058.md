*Closed record — see [../README.md](../README.md). Not spec.*

# Q58 — How does power work: solar panels, wires to a printer, and machines that stop when a team is short (amending Q10, Q27, Q37, Q42 and Q45)

## Ruling

*Ruling (2026-10-12):* **power is a team's flow, tick by tick: printers
and solar panels supply it, bots and some buildings draw it, a building
counts only when wired back to one of the team's printers, and when the
draw outruns the supply the newest machines stop.** Made under a new
number because every ruling it amends is a closed record:
[Q10](question-answered-0010.md) and [Q27](question-answered-0027.md) on
the plans a player may mark, [Q37](question-answered-0037.md) on the mark
strip, [Q42](question-answered-0042.md) on deployments and printing, and
[Q45](question-answered-0045.md) on a machine's lights.

### The rules

Rules 1 to 10 are the sim's and hash-affecting; rule 11 is the renderer's
and reaches no hash.

1. **Power is a flow, not a resource.** Every model has a `power_supply`
   and a `power_draw` in data, either of which may be zero; today the
   printer and the solar panel supply, and the bot and the depot draw.
   Power is not a resource kind: it is never hauled, held or spent from a
   store, and nothing keeps it from one tick to the next.
2. **The grid.** A team's **conductors** are the tiles carrying its wire
   and the tiles of its buildings, sites excluded. Its **grid** is every
   conductor joined to a tile of one of its printers through adjacent
   conductors. A building is **on the grid** when its tile is, so a
   printer always is. Bots need no grid: they draw wherever they stand. A
   building off the grid neither supplies nor draws.
3. **Supply** is the sum of `power_supply` over the team's buildings on
   the grid whose health is above zero.
4. **Who is powered.** A machine whose `power_draw` is zero is always
   powered. The others — every bot, and every building on the grid that
   draws — are taken in ascending entity id, oldest first, and each is
   powered while the draw so far, its own included, is within the supply;
   the first that does not fit and every newer one are not. A building off
   the grid that draws is never powered.
5. **An unpowered machine stops.** Its main flow does not run, and the
   action main flow began does not count down; an unpowered printer's print
   does not count down and it begins none, and an unpowered site's
   construction waits; a printer's defence goes on. Power never withholds a
   handler: interrupts are still delivered and their hooks run as always,
   so `dying` still ends in `death` and a redeploy still swaps.
6. **Printing waits for spare power** (amending Q42). A team's **spare** is
   its supply, minus the draw of every consumer, powered or not, minus
   `bot`'s draw for each print in progress, so no print begins while any
   machine is dark. A print begins only while the spare covers one more
   bot's draw, as well as while Q42's shortfall is above zero, so printing
   never on its own pushes a team short.
7. **Wires are a team's, and only its own bots touch them.** A tile's
   `wires` is the set of teams with a wire on it, on `ground` only, with or
   without a deposit or a building. A wire carries only its team's power.
   Every team sees a wire and remembers it, as it does paint, but only the
   wire's own team lays or removes it.
8. **`wire(x, y)` and `unwire(x, y)`** are bot actions on an adjacent
   tile. `wire` lays the team's wire on ground without one, taking
   `wire_cost` from the bot's load when it completes; `unwire` removes the
   team's own wire, refunding nothing. A **wire plan**, the fourth plan
   kind (amending Q10 and Q27), marks a tile for either, is the usual
   target and is consumed; like paint, it goes only on ground the team has
   seen (Q49).
9. **The solar panel**, model `solar`, is a building like the depot: a
   site priced in ore, then `build_ticks`, with health, no store, and a
   constant `power_supply`. It adds the deployment `"solar"`, so a team has
   four: `"bot"`, `"printer"`, `"depot"` and `"solar"` (amending Q42).
10. **What a program reads, and where it runs.** Every machine has the
    attribute `powered`, and every building `on_grid`; a tile's record
    carries `wires`; `power()` returns the team's supply, demand and spare,
    and `wires()` the tiles carrying its wire. The tick's step 6 becomes
    *power and printing*: each team's grid is traced and its machines
    powered, then its printers begin prints. A machine keeps its powered
    state until the next step 6, and one made in step 3 is unpowered until
    then. The tile's `wires` and the machines' new attributes are in the
    state hash.
11. **On the map** (amending Q37 and Q45): a wire is a line across its
    tile in its team's color, joining its wired neighbours; the mark strip
    gains a wire tool with its *clear*; a machine's lights go dark while it
    is unpowered or, for a building, off the grid; the inspector shows the
    player's own team's supply, demand and spare.

### Consequences

- **Power is a second cap on bots.** Q24's cap still counts printers; a
  team also prints no bot it cannot power, so panels raise how many bots a
  team runs once printers alone no longer cover them.
- **Wires make placement a decision.** A panel may stand anywhere, but it
  counts only once a line of wire, or of the team's buildings, reaches it
  from a printer, and losing the printer at the root drops what hangs from
  it.
- **No sabotage of wires.** Rivals cannot cut a line; they cut power by
  attacking panels and printers, which have health. A line can still be
  cut off by the team's own `unwire` or by a building on its route being
  destroyed.
- **A shortfall is visible and ordered.** The newest go dark first, so a
  team's oldest machines — its first printer and first bots — keep running,
  and a program can read who is powered and why.
- **A stopped machine is still a body.** It keeps its tile, can be
  attacked, and still dies; it simply does not think or act until power
  returns.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the Q22, Q42 and new
  Q58 Decided entries, A machine's deployments, the kind tree and table,
  the attributes, the senses, the actions, Printing, a new Power section
  and what it leaves open. [03-world.md](../../03-world.md) — the Q49
  bullet, the tile's fields and its slots.
  [06-architecture.md](../../06-architecture.md) — the Q10 entry, the
  tick's step 6 and the state hash.
  [07-interface.md](../../07-interface.md) — the Q37 and Q43 entries and a
  new Q58 entry, the program tree, Machines on the map, the inspector and
  Marks. [05-progression.md](../../05-progression.md) — power beside
  printers. [00-overview.md](../../00-overview.md) — the Decided listing.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T31](../../TASKS.md) — power in the sim: the grid, supply,
  powered machines, wires and their actions, the solar panel, the senses,
  the data, the cost rows, the starter programs and the fixtures.
- **Task:** [T32](../../TASKS.md) — power in the renderer: wires, the wire
  tool, dark lights, the panel's art and the inspector's figures.
- **Problem:** [P8](../../PROBLEMS.md) — the docs state Q58 as current
  until T31 and T32 land.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| what power is for | **A budget bots and buildings share** *(chosen)* | One number to manage; a shortfall needs a rule for who loses out. |
| what power is for | Bots carry batteries | The most programming-heavy option, and every program must handle it. |
| the pool | **Wired back to a printer** *(chosen)* | One team pool and wires that matter: a far panel must be connected. |
| the pool | Each network separate | Richer topology, but bots have no network to draw from. |
| a shortfall | Brownout: everyone slows by the same fraction | Nothing stops dead; fractional budgets in every machine. |
| a shortfall | **Unpowered machines stop** *(chosen)* | Plain to see and to program against; needs an order. |
| a shortfall | Actions cost power | Compute untouched; bookkeeping on every action. |
| the order | **Newest first** *(chosen)* | Deterministic, needs no input; the base is the last to go dark. |
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

### How the answer took its shape

The user set out to design water and electric systems and the resources
behind them. A list of metals, fuels and generators was weighed first,
and the user chose to start with power alone: a budget that bots and
buildings share, wires that bots build in the ground, and solar panels
that can stand anywhere. Asked how a shared budget and wires fit together,
the user chose a pool fed by everything wired back to a printer; for a
shortfall, machines that stop, the newest first; printers as the first
source; printing only with spare power; wires only their own team may
touch; and the panel as a building. That handlers still run on a stopped
machine, and that a wire costs a little ore from the bot's load, were the
drafting's calls, made so that `dying` still ends in `death` and so that a
line of wire is a decision; the wire plan followed from Q10's rule that the
player marks and bots act.
