*Closed record — see [../README.md](../README.md). Not spec.*

# Q22 — The economy: what resources exist, how they are gathered, what things cost, and which buildings there are beyond the printer

## Ruling

*Ruling (2026-09-07):* **one resource, gathered by bots from deposits that
regrow, carried to buildings that hold it, and spent from where it is
held.** Resources are a *table* with one row today — `ore` — and every
quantity in the economy is keyed by resource kind, so a second row is data
and not a redesign. The building list gains a **depot**, whose purpose is to
hold resources, and is otherwise `docs/02`'s table to grow under new numbers.

### The rules

Each is hash-affecting; every number is tuning in data.

1. **Resources are a table.** Each kind has a name; every store capacity,
   cost, deposit amount and bot load is a map from kind to `num`. Today the
   table holds `ore`. Adding a kind is a data change and a `docs/03` row,
   not a question, unless the new kind needs a mechanism this ruling lacks.
2. **Deposits are terrain** (`docs/03`): a tile carries an amount of a kind,
   up to a cap, and **regrows** by a rate per tick until the cap. A deposit
   is remembered by Q21 with its amount as of last sight, so a program acts
   on what the colony believes is there.
3. **Bots carry.** A bot has a **capacity** per kind and a load. Adjacent to
   a deposit, its program calls `pick` — `docs/02` names it — and takes up to
   a rate per action, bounded by the deposit's amount and its own free
   capacity; adjacent to a building, it calls `drop` and delivers up to the
   building's free capacity, and `pick` from a depot takes from it. A bot
   carries while doing anything else.
4. **Every building has a store**, a capacity per kind. The printer's is
   small; the **depot's** is large and holding is all it does. The colony's
   stock is the sum over its buildings, and a program reads any building's
   store as an attribute (Q7 sightings carry all attributes).
5. **A print spends from the printer's own store.** A printer whose store
   cannot cover the print faults (`ValueError`, Q9). Feeding printers is
   therefore a hauling problem, which is the point.
6. **A build is a site that bots fill.** `build` (Q9) places a site on the
   tile with a store whose capacity is the building's cost; bots `drop` into
   it like any building; when the store reaches the cost, construction runs
   for the build time and the building appears with its store empty. A site
   is a building for sensing, memory and targeting, of a kind `docs/02`
   names, and it holds no program until it completes.
7. **The starting printer starts stocked**, with an opening amount per kind
   in its store, so the first prints need no hauler. Q9's start — one printer
   per player, placed by the map, no bots — stands, with the stock added.
8. **Draining is deterministic.** A print takes from its printer's store
   only, a site from its own; nothing spends from a store it does not own,
   so there is no ordering to define across stores. Within one tick, actions
   happen in slice order (`docs/06`).
9. **The building list**, `docs/02`'s table, now holds **printer** and
   **depot**, each with Q7's two ranges, a store capacity per kind, a build
   cost and time, and — for the printer — a print cost and time. Buildings
   the design has imagined and not ruled — something that defends, something
   that extends senses — are each a new question number when wanted.
10. **Order** (determinism rule 6): deposits and buildings in any query sort
    as tiles and sightings already do; a store's kinds sort by the resource
    table's order.

### What each doc owns

`03` defines deposits, their amounts, caps and regrowth, and where the map
puts them and the starting printers. `02` names `pick`, `drop`, `build` and
`print`, the site kind, the bot's capacity, and the building table with its
stores, costs and times. The resource table lives in data with the rest.

### Consequences

- **`docs/03` is unblocked**, and every numbered doc from `02` to `05` is
  now writable or waits only on the architecture questions.
- **The first program a player writes is a hauler**, and the second feeds a
  printer. Q2's fleet fantasy has its first concrete loop: scout for a
  deposit (Q7, Q21), carry from it (this ruling), print more carriers (Q9),
  and patch the program that does it badly (Q3).
- **Sites make buildings a target before they exist**, and a depot makes
  stock a target after. Defence is worth a building, which is a new number.
- **Regrowth bounds the game's pace without ending it**: no deposit runs dry
  for good, so a match is about routing and territory rather than depletion.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section and the
  reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T9](../../TASKS.md) — Q22 leaves its list; `03` is writable.
  [T7](../../TASKS.md) — no longer blocked on this question.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **One resource, gathered by bots from map deposits and carried to a building** *(chosen, with a resource table so more can follow)* | Simplest economy that makes scouting, routing and defence all matter. Carrying is a program's job, so the first thing a player writes is a hauler, which is either the game or a chore. |
| One resource, extracted by a building placed on a deposit | Bots are freed for exploration and fighting; the economy is territory. Extraction is passive, so early play is placement rather than programming. A widening if hauling proves a chore. |
| Several resources with distinct uses | Richer decisions, a bigger tuning surface, and every building's row has more numbers to get wrong. The table makes this a data change later. |
| No scarcity — prints and builds cost time only | Nothing to gather and nothing to defend. The fleet's size is bounded by time alone, and every match is the same race. |

### How the answer took its shape

Hauling won because it is the one shape in which the economy *is*
programming. An extractor is placed once; a hauler is written, watched
failing, and rewritten, which is the loop Q3 chose the whole game around.
The resource table was the cheap way to keep the door open: keying every
quantity by kind costs nothing now and turns "add a second resource" into a
row.

**Stores everywhere** came from asking where a delivered resource goes. A
player-wide pool was simpler but made the depot pointless and delivery
meaningless — a bot could drop anywhere. Giving every building a store made
the depot a building that does one thing well, made printers something to
feed, and made a build site fall out for free: a site is a store with a
target amount.

**Regrowth** was the user's call and it is the right one for a lockstep game:
a deposit that runs dry is a clock, and a clock ends matches by attrition;
a deposit that regrows is a rate, and a rate is something a program can be
tuned against.

**The opening stock** is the only rule here nobody asked for. Without it, a
printer, no bots and a priced print is a deadlock on tick zero. Stocking the
printer rather than starting with a depot or a bot keeps Q9's start state
exactly as ruled.
