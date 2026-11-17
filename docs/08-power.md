# 08 — Power

Batteries, what fills and empties them, the grids buildings form, and the
two kinds of line that join them. This doc is what the power half of
`crates/sim` is built from, and it is normative: two implementers reading
it must produce the same tick-by-tick behavior (design-invariant DI7). The
machines themselves — their models, attributes, senses and actions — are
[02-machines](02-machines.md)'s, and each row there that touches power
cites this doc for the rule.

## Decided

- **Power is stored: every bot has a battery its calls run down and its
  own solar refills, buildings generate and drain, connected buildings
  share what they make as a grid, and a line between them is a row of
  pylons or a wire underground (Q62, withdrawing Q59 and amending Q8, Q10,
  Q16, Q23, Q27, Q37, Q41, Q42, Q49, Q53 and Q56; its flat bot and its
  `low_power` as Q63 amended them, below).** A battery holds
  **charge**, a `num`. Every game builtin a bot calls has a charge cost in
  data, which may be zero; a bot whose charge does not cover the call it
  has reached is **flat** and waits there, and its solar wakes it. At a
  level of its battery set in data, a fifth, a bot takes the `low_power`
  interrupt, the fifth kind. Every building model has a
  generation and a drain per tick; a team's buildings that touch, or are
  joined by its wire or its pylons, are a **grid**, served oldest first,
  each building keeping its own battery; a building the grid cannot cover
  is **dark**, and its rules, program and senses stop. A printer and the
  **charger** hold a battery any bot may `recharge` from; the **solar
  panel** only generates; a printer generates more than it uses, by a
  check on data, so it is never dark. A **pylon** is a building rivals may
  destroy; underground **wire** costs more and only its own team touches
  it. Q59's flow, its printer at the root of every grid, its newest bots
  stopping and its spare power for a print are withdrawn and have no entry
  anywhere.
- **`low_power` returns to main flow, and a bot out of power charges back
  to a level before it acts (Q63, amending Q62 and Q8).** `low_power`
  waits for the action in progress, sets main flow aside, runs
  `on_low_power`, which may do anything — a whole trip to a charger — and
  puts main flow back where it was, every variable intact; a bot with no
  hook is not disturbed. A bot whose main flow or whose `on_low_power`
  reaches a call it cannot pay for is out of power: flat until its solar
  has brought it to a **wake level** in data, which starts equal to the
  low-power level. Waking raises `low_power`, so the hook runs first — or
  starts over, if it was the hook that ran dry. `on_fault` and `on_dying`
  are never held for power.

## Charge and batteries

Power is one quantity, **charge**, a `num`, and it is not a resource kind
(`02`, Q22): no bot carries it as a load and no store holds it. A machine
holds charge in its **battery**: its `charge` attribute, never below zero
and never above its model's `battery`, the capacity, which is its
`battery` attribute. A model whose `battery` is zero holds none.

| Model | Battery | Generates | Drains | Recharges bots |
|---|---|---|---|---|
| `bot` | yes | a little, its own solar | nothing per tick; its calls cost charge | — |
| `printer` | yes | yes, more than it uses | per tick, and more while printing | yes |
| `charger` | yes, large | a little, its own solar | per tick | yes, quickly |
| `solar` | no | yes | nothing | no |
| `depot`, `pylon`, `site` | no | nothing | nothing | no |

Every number is a row of the model in
[data/machines.toml](../data/machines.toml): `battery`, `generation` and,
for a building, `drain`, each charge or charge per tick and zero where the
table says nothing; a printer's `print_drain`; and `recharge_rate` for a
building bots recharge from. The table above is what the first data will
hold, not a rule: the rules below read the numbers, never the model's
name, so a depot given a drain would go dark like anything else.

**What starts charged.** A bot is printed with a full battery, at no cost
to anything. A printer the map places starts full. A building a site
becomes starts with none.

**The sim refuses data** in which the bot's `generation` or `battery` is
not above zero; in which `low_power_level` is not above zero and below
one, or `wake_level` is not above zero and at most one; in which any
charge cost (below) is above `wake_level × battery` for the bot; or in
which the printer's `generation` is below its `drain` plus its
`print_drain`. The first three mean a flat bot always wakes and can then
pay for any one call (Q63); the last, that a printer is never dark.

## A bot's battery

### Charge costs

Every game builtin (`02`) has a **charge cost**, a row of the `[power.cost]`
table in data named for the builtin, zero or more. It is what a **bot's**
call of that builtin takes from the bot's battery; a building's calls take
nothing, since a building pays by its drain.

In main flow and in `on_low_power`, a call whose cost is zero runs as it
always did. For any other:

1. **If the bot is flat, or its `charge` is below the cost, the call does
   not begin.** The bot is **out of power** (below) — its `flat`
   attribute is `True` — and the program waits at the boundary before the
   call, as it waits on an action (`01`, Metering), with interrupts
   delivered there as at any boundary. Nothing of the call is charged, in
   charge or in cycles; what evaluating its arguments cost was spent
   before it, as any earlier operation's is — `log`'s rendering of an
   instance through `__str__` included.
2. **In the first slice that reaches that boundary with the bot awake**,
   the call runs as if first reached in that slice: the test above is
   made again, then its preconditions are checked, and a failure is the
   builtin's `ValueError`, which takes no charge.
3. **Otherwise the cost leaves the battery as the builtin begins**, with
   its cost in cycles (`01`, costs). An action, once begun, is never held
   for power: its ticks count down whatever the battery holds.

A call inside **`on_fault` or `on_dying`** is never held: it takes
`min(cost, charge)` and runs, whether or not the bot is flat, so `dying`
still ends in `death` and a flat bot's `on_fault` still runs.

### Solar

In the tick's power step (below), every bot's charge rises by its model's
`generation`, to at most its `battery`. This is the only thing that
charges a bot without its acting, and since data keeps it above zero, a
flat bot is flat for a bounded number of ticks: no team can lock itself by
running its bots down.

### Out of power

A bot is **out of power** (Q63) from the moment its main flow or its
`on_low_power` reaches a call it cannot pay for (Charge costs) until it
**wakes**; its `flat` attribute says so. While it is flat:

- **No call with a charge cost begins** in main flow or in
  `on_low_power`, whatever the battery has since come to hold; each waits
  at the boundary before it. A call that costs nothing runs.
- **Everything else goes on.** Interrupts are delivered; a `redeploy`
  swaps the program, and a restart of main flow, by a fault or a
  redeploy, leaves the bot flat. `on_fault` and `on_dying` run and are not
  held. The bot keeps its tile, its load and its health, and can be
  attacked.
- **Its solar charges it**, in the power step, like any bot's.

**Waking.** In the power step, straight after the bots' solar, every flat
bot whose charge is at least `wake_level × battery`, both from data, in
ascending entity id: it is no longer flat, and `low_power` is raised for
it, armed or not (Low power, below). What follows is `01`'s: if main flow
was what stopped, `on_low_power` runs first and main flow then makes the
call it stopped at; if `on_low_power` was what stopped, it starts over
from its first line, with main flow still set aside beneath it; and a bot
with no hook simply goes on. Data starts `wake_level` equal to
`low_power_level`, and the two are tuned apart freely.

### Recharging

`recharge()` (`02`, Actions) moves charge from an adjacent building's
battery into the bot's. Any building holding charge and powered (below)
qualifies, whatever its team, so a bot may take a rival's power as it may
take a rival depot's ore. The building's model sets the rate: the
charger's is high, the printer's lower.

### Low power

`low_power` is the fifth interrupt kind, between `fault` and `redeploy`,
and the one that returns to main flow (Q63): it waits for the action in
progress, runs `on_low_power` with main flow set aside, and puts main
flow back where it was. What that is in execution is
[01-language](01-language/execution.md)'s, and what it does to the body
— nothing — is `02`'s. The hook may do anything, and is where a program
goes to recharge. The world raises the kind from a bot's battery alone,
in two cases:

- **Falling to the level.** A bot is **armed** when it is printed.
  Whenever a bot's charge changes — a call's cost, a `recharge`, the
  solar — the level `low_power_level × battery` is compared, both from
  data. An armed bot whose charge is **at or below** the level has
  `low_power` raised for it and is no longer armed. A bot that is not
  armed and whose charge is **above** the level is armed again.
- **Waking** from being out of power (above), armed or not, which leaves
  whether it is armed as it was.

So the interrupt comes once each time the battery falls through the
level, however long it then stays low, once each time the bot wakes, and
not otherwise. Whether a bot is armed is its state, in the hash. A
building is never armed and never takes the interrupt.

## Grids

A team's **conductors** are the tiles carrying its wire (`03`) and the
tiles of its buildings, sites excluded. Two conductors are **joined** when
they are adjacent (`02`), and a pylon's tile is joined besides to the tile
of every pylon and building of its team, sites excluded, within the
pylon's `span`: squared distance at most `span²`, whatever stands between.
A **grid** is a set of a team's conductors joined to one another, directly
or through others, that holds at least one building; its **label** is the
least entity id among its buildings, which is each of those buildings'
`grid` attribute. A building that touches nothing is a grid of one. Wire
joined to no building is no grid. Grids are a team's: two teams'
conductors never join, though they share a tile.

### The power step

In step 6 of the tick (`06`), for each team in order:

1. **The grids are traced**, as above, from the world as steps 1 to 5 left
   it. A `dying` building is still a building until its `death` removes
   it.
2. **Each grid is served**, grids in ascending label, and within a grid
   its buildings in ascending entity id — oldest first — three times
   over:
   1. *Its own first.* A building's **need** is its model's `drain`, plus
      `print_drain` for a printer whose `printing` is above zero. It keeps
      `min(generation, need)` of its own generation; the rest of its
      generation goes into the grid's **pool**; and what its need still
      lacks is its **shortfall**.
   2. *The shortfalls.* A building with no shortfall is **powered**. Each
      other, in turn: if the pool plus the charge in every battery on the
      grid covers its shortfall, the shortfall is paid — from the pool
      first, then from the batteries in ascending entity id — and it is
      powered; if not, it takes nothing, it is **dark**, what it kept in
      the first pass goes into the pool, and the next building is tried.
   3. *What is left.* The pool fills the grid's batteries in ascending
      entity id, each to its `battery`, and whatever then remains is lost.
3. **Every bot's solar** is added (Solar, above), in ascending entity id.
4. **Flat bots wake** that have the charge to (Out of power, above).
5. **Prints begin** (`02`, Printing).
6. **The figures are kept**: for each grid its label and four sums over
   its buildings — `generation`; `drain`, every need, powered or dark;
   `stored`, the charge held after the step; and `capacity`, the
   batteries' — as team state, in the hash, which `power()` (`02`) returns.
   An out team has no grids and keeps none.

`powered` and `grid` are set here and hold until the next power step. The
match is built with its grids traced, every building powered and the
figures kept as they then stand, no charge having moved, so every building
has both and `power()` an answer from the first slice; a building
completed in step 3 has them from the same tick's step 6, before its first
slice. A site is on no
grid: its `grid` is `None` and its `powered` is `False`, and neither
matters, since a site does nothing.

### A dark building

A building the last power step left dark, until the next one:

- **Its rules stop.** A printer begins no print and its print in progress
  does not count down (`02`, Printing); it deals no defence damage (`02`,
  Q30); and no bot may `recharge` from it.
- **Its program stops.** Its slice opens with its budget as always, so a
  deficit is paid down, but main flow does not run and the action main
  flow began does not count down. A handler runs as always, with its
  hook and the hook's actions, so `dying` still ends in `death` and a
  redeploy still swaps.
- **Its senses stop.** It sees and hears nothing: it adds nothing to its
  team's `see()` and `hear()` (`02`) and nothing to the vision pass
  (`06`, step 8), so tiles only it saw become remembered.
- **Everything else stands.** It keeps its tile, its health, its store
  and its charge; it still generates, its battery still fills, and it
  still conducts; it can be attacked and deconstructed; and a dark printer
  still counts for its team's cap (`02`, Q24) and for the team not being
  out (`04`).

Data keeps a printer from ever being dark (Charge and batteries, above),
so the printer's entries here bind only if that check is one day removed;
they are stated so that the rule is whole.

## Lines

A grid reaches across open ground by one of two lines, and a team may mix
them.

**A pylon** is a building, model `pylon` (`02`): built from a site for
ore, with health, on a tile it occupies like any building. It has no
battery, generation or drain, no deployment and no program. It conducts,
and joins across its `span` (Grids, above), so a row of pylons spaced
within the span carries a grid overhead with free ground between them,
and bots walk that ground. Any team's bot may attack or deconstruct one
(`02`), and that is how a rival cuts a line: the grid divides wherever the
pylon was the only join, and since every building keeps its own battery,
nothing has to be shared out when it does.

**A wire** is underground: a mark of the team's on a `ground` tile (`03`),
laid and removed by the team's own bots alone with `wire` and `unwire`
(`02`), each tile priced in ore at `wire_cost`. It occupies nothing — a
bot, a building or another team's wire may share its tile — and it joins
only by adjacency, so a line of wire is one tile after another. No rival
can remove it. Every team sees and remembers a wire, as it does paint
(`03`). The player plans it with the wire tool, and a pylon with a
building plan (`07`). Wire is priced so that a line of it costs more than
the pylons that would cover the same ground; that is tuning, in data, and
no rule depends on it.

A team that is out has its wires cleared with its machines, its pylons
among them (`06`, step 5).

## What a program reads

- On every machine, `charge` and `battery`; on a bot, `flat`; on a
  building, `powered` and `grid` (`02`, attributes). A sighting carries
  them like any attribute, a rival's included.
- `power()`, the team's grids as the last power step kept them, and
  `wires()`, the tiles carrying the team's wire (`02`, Senses).
- A tile's record carries `wires` (`03`).

Rates are charge **per tick** everywhere in the sim and in what a program
reads. The interface shows them per second, at the speed in force
(`07`); that figure is the renderer's and nothing reads it back.

## Costs

`recharge`, `wire`, `unwire`, `power` and `wires` each have a row in the
`[game]` cost table like every builtin (`02`, Costs), and every game
builtin has a row in `[power.cost]`. The power step costs no program
anything.

## What this doc leaves open

- **More sources and sinks**: a generator that burns a resource, a panel
  whose output varies, a per-tick drain on a bot, and a charge cost on a
  building's calls. Each is a new question number.
- **More ways to move charge**: a bot giving charge to a bot or to a
  building, and a team closing its chargers to rivals.
- **A player-set order** for which buildings a short grid serves first.
- **A rival cutting wire.** Underground wire is untouchable today.
- **The numbers.** Every capacity, rate, cost and span is a placeholder
  until the game is played.
