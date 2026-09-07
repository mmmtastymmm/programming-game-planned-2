*Closed record — see [../README.md](../README.md). Not spec.*

# Q23 — Amend Q9 and Q22: printing costs time only; building costs resources

## Ruling

*Ruling (2026-09-07):* **a print is free of resources and costs time only;
a build costs resources.** This amends rule 3 of
[Q9](question-answered-0009.md), which priced a print in resources, and rules
5 and 7 of [Q22](question-answered-0022.md), which had a print spend from
the printer's store and stocked the starting printer to avoid a deadlock —
made under a new number because both files are closed records. Everything
else in Q9 and Q22 stands.

### The rules

Each is hash-affecting.

1. **A print costs time only.** A printer's program calls `print` with a
   role; the printer is printing for the print time, a tuning constant, and
   the bot appears when it ends. No resource is spent and no store is
   consulted. A printer prints one bot at a time.
2. **A build costs resources**, exactly as Q22 rule 6 says: a site with a
   store whose capacity is the building's cost, filled by bots, then the
   build time.
3. **The printer has no store.** Q22's "every building has a store" stands
   with the printer's capacity at zero in data; a `drop` into a printer is a
   `ValueError` fault. Resources are held in depots and in sites.
4. **The starting printer starts empty**, as does everything else. Q22's
   opening stock is withdrawn: with prints free, tick zero prints a bot, and
   that bot hauls from a deposit to the first depot's site.
5. **Fleet size is bounded by printers and time**, not by ore: one printer
   yields one bot per print time. More printers cost ore to build, so the
   economy still bounds the fleet, one step removed.

### Consequences

- **The first loop is simpler**: print, haul, build a depot, build a second
  printer. Ore is spent only on buildings, so every building is a decision
  and every bot is free — and a bot's cost is the printer time it occupied.
- **A depot is the first thing worth building**, since sites are the only
  other place ore can go and a site is consumed when it completes.
- **Q22's "feeding printers is a hauling problem" is withdrawn**; feeding
  *sites* is.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — the Q9 and Q22 bullets
  in the Decided section. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Prints cost ore, as Q9 and Q22 ruled | Every bot is a decision, and the starting printer must be stocked or tick zero deadlocks — an opening stock nobody asked for. Printers need stores, and feeding them is a second hauling problem beside sites. |
| **Prints cost time only; builds cost ore** *(chosen)* | Bots are free and printers are the bottleneck; ore goes only into buildings, each of which is a decision. No opening stock, no printer store, one hauling problem. Fleet size is bounded by printers times time, and printers cost ore. |
| Both free | Nothing to gather; the economy is decoration. Rejected by Q22 already. |

### How the answer took its shape

Q22's opening stock was the tell: a rule added to escape a deadlock the
pricing had created. Making prints free removes the deadlock at its source
and takes the printer store, the opening stock and the second hauling
problem with it, at the cost of one bound moving — fleet size is now limited
by printer throughput rather than ore — which the ore cost of a second
printer restores one step removed.
