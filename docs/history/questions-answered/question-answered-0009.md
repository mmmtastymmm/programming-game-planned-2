*Closed record — see [../README.md](../README.md). Not spec.*

# Q9 — Where do units come from?

## Ruling

*Ruling (2026-09-07):* **bots are printed by a building, the printer, under
program control; buildings are built by bots; and a match starts with one
printer per player and nothing else.** There is no fixed roster. Production
is ordinary program behavior — the register's third option — done by a
building whose program calls a game builtin, so what a colony makes is
authored and re-authored like everything else it does (Q3).

### The rules

Each is hash-affecting.

1. **The printer is a building kind** (Q7), with a vision and hearing range
   like any kind, and a program like any unit. Its program calls a `print`
   builtin — `docs/02` names it — with the **role** the new bot is to run.
2. **All bots are one kind.** Q7's `bot` is the only mobile kind; what
   differs between bots is the program their role runs, never their
   hardware. A second bot kind is a new question number.
3. **A print takes time and costs resources**, both tuning constants in data.
   The time is a number of ticks during which the printer is printing; the
   resources are `docs/03`'s and Q22's. A print the printer cannot afford is
   a `ValueError` fault in its program.
4. **A printed bot appears on a tile adjacent to the printer** — `docs/02`
   pins which — on the tick the print completes, in main flow, running its
   role's current bundle (Q11) from the top. Its first slice is the next tick.
5. **A role exists once a bundle is deployed to it.** Printing a role that
   has no bundle is a `ValueError` fault. A unit whose role's bundle is later
   removed does not exist as a case: a redeploy replaces a bundle and never
   removes one.
6. **Buildings are built by bots.** A bot's program calls a `build` builtin
   — `docs/02` names it — with a building kind and a tile; the build takes
   time and costs resources, both tuning; and the building appears on that
   tile, in main flow, running its role's bundle. What tiles may be built on
   is `docs/03`'s.
7. **A match starts with one printer per player, placed by the map**
   (`docs/03`), and no bots. The **opening program set** is the first entries
   of the command log: one deploy per role the player's starting bundles
   name, agreed for tick 0, so the printer has a bundle before its first
   slice. This is the answer to I2.
8. **A unit whose role has no bundle** runs the empty program: main flow with
   no operations, which reaches its end boundary and restarts, once per tick
   ([`docs/01`](../../01-language/execution.md)). It is not a state; it is a
   program with nothing in it. A printer in that condition prints nothing
   until a deploy reaches its role.
9. **Order**: prints and builds are actions in a unit's slice, so they happen
   in slice order (`docs/06`), and two units that complete on the same tick
   appear in entity-id order.

### The building list

The list is `docs/02`'s table, one row per kind with its ranges (Q7), its
cost and build time, and what it does. This ruling seeds it with one row,
**printer**, and leaves the rest to Q22, because every other building the
design has imagined — something that gathers, something that stores,
something that defends — is a building only an economy can price.

### What each doc owns

`02` names `print` and `build`, the adjacent-tile rule, and the building
table. `03` places the starting printer, says what may be built where, and
defines the resources. `06` orders the actions within a tick and the
opening deploys within the command log. Q12 owns when a deploy takes effect;
this ruling needs only that tick 0's deploys precede tick 1.

### Consequences

- **`docs/02` is unblocked.** Q7, Q21 and this ruling are its Decided
  entries, waiting in the overview until it is written.
- **Q22 is opened**: the economy — what resources exist, how they are
  gathered, what things cost — and with it the rest of the building list.
  `docs/03` now waits on it.
- **I2 is triaged.** The opening program set is the command log's tick-0
  deploys; a unit with no bundle runs the empty program.
- **Production is the second thing a program can get catastrophically
  wrong** (the register's own warning): a printer that prints the wrong role
  fifty times has done exactly what Q3 lets the player patch mid-match.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section and the
  reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Question:** [Q22](../../QUESTIONS.md) — the economy and the rest of the
  building list.
- **Task:** [T9](../../TASKS.md) — Q9 leaves its list and Q22 joins it;
  `02` is writable. [T7](../../TASKS.md) — no longer blocked on this
  question.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Fixed roster, chosen before the match | Simplest. A match becomes a pure test of the program set, and the economy stops being gameplay. |
| Produced by a rule the player configures, and can reconfigure mid-match | Consistent with Q3: production is another thing you author and re-author. The configuration surface is a second, smaller language to design unless it folds into the unit language. |
| **Produced by ordinary programs — a printer building's program prints bots, and bots build buildings** *(chosen)* | One authoring surface, maximum consistency with Q2. Production becomes something a program can get catastrophically wrong — which Q8's fault model, and Q3's mid-match patch, then have to survive. |

### How the answer took its shape

The third option was the lean from the start, for consistency with Q2 and
Q3: if everything a colony does is a program, production is too. Q7 supplied
the actor. Once buildings were unit kinds with programs, "a building whose
program prints" was the only shape that needed no new mechanism — a printer
is a unit that senses, decides and acts like any other, and its action
happens to be a bot.

Two things followed that the question had not asked. **Buildings need an
origin too**, and with bots the only mobile actor, bots building them is
forced. **Printing needs a price**, or a printer's program is a `while True:
print(role)` loop and the game is over on tick one; a price means resources,
and resources are an economy the corpus had not yet opened a question for.
Both are ruled here as far as the mechanism goes and deferred to Q22 for the
numbers and the list, which is where the register's warning about production
being "catastrophically wrong" gets its teeth.
