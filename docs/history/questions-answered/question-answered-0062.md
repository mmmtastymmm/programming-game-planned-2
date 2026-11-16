*Closed record — see [../README.md](../README.md). Not spec.*

# Q62 — Should power be batteries and grids: bots that run down and recharge, buildings that generate and drain, and wire overhead or underground (replacing Q59)?

## Ruling

*Ruling (2026-11-15):* **yes. Power is stored: every bot has a battery its
calls run down and its own solar refills, buildings generate and drain,
connected buildings share what they make as a grid, and a line between
them is a row of pylons or a wire underground.** Made under a new number
because every ruling it touches is a closed record. It **withdraws
[Q59](question-answered-0059.md) outright** — the flow with nothing
stored, the printer at the root of every grid, the newest bots stopping
and the spare power a print waited for — and rules again what it keeps of
it: the underground wire with its plan and its tool, and the solar panel.
It amends [Q8](question-answered-0008.md) and
[Q16](question-answered-0016.md) on the interrupt kinds, which become
five; [Q41](question-answered-0041.md) on the hook files, which become
three; [Q23](question-answered-0023.md) and
[Q42](question-answered-0042.md) on printing, which now drains power, and
on the deployments, which gain `charger`;
[Q10](question-answered-0010.md), [Q27](question-answered-0027.md),
[Q37](question-answered-0037.md) and [Q49](question-answered-0049.md) on
the wire plan and its tool, which Q59 had added to them;
[Q53](question-answered-0053.md) on the starter library; and
[Q56](question-answered-0056.md) on the `Power` record.

### The rules

Rules 1 to 13 are the sim's and the language's and are hash-affecting;
rule 14 is the renderer's and reaches no hash. The one statement of each
is [08-power](../../08-power.md)'s, [02-machines](../../02-machines.md)'s
or [execution](../../01-language/execution.md)'s; this is the ruling as
made.

1. **Charge.** Power is one stored quantity, charge, a `num`, held in a
   machine's battery up to its model's capacity. It is not a resource
   kind. A bot is printed full, a printer the map places starts full, and
   a building a site becomes starts empty.
2. **Every game builtin has a charge cost**, in data, which may be zero,
   taken from a bot's battery when its call begins. A building's calls
   take nothing.
3. **A flat bot waits.** A bot whose charge is below the cost of the call
   its main flow has reached does not begin it: it is flat, and waits at
   that call until its charge covers it. Hooks are never held; a call in
   a hook takes what charge there is and runs, so `dying` still ends in
   `death`. An action once begun is never held.
4. **Every bot has solar.** Each tick a bot's charge rises by its model's
   generation, which data keeps above zero, so a flat bot wakes by itself
   and no team can lock itself.
5. **`low_power` is a fifth interrupt kind**, below `fault` and above
   `redeploy`, raised for a bot when its charge falls to a level of its
   battery set in data, a fifth; once per fall, and armed again only when
   the charge is back above the level. Its prologue cancels the action in
   progress, its optional hook `on_low_power()` runs under a budget, and
   its epilogue starts main flow over with every variable cleared, as
   `fault`'s does. A failure in the hook escalates to `fault`.
6. **Every building model has a generation and a drain per tick**, either
   of which may be zero, and some have a battery. A printer drains more
   while it prints.
7. **A grid** is a group of a team's buildings joined by standing
   adjacent, by the team's wire, or by a pylon's span; sites are on none,
   and no printer is needed at its root. Wire joined to no building is no
   grid.
8. **A grid is served oldest first**, in ascending entity id. A building's
   own generation covers its own need first and the rest is pooled; each
   building still short is paid from the pool and then from the grid's
   batteries, oldest first, if together they cover it, and otherwise is
   dark and takes nothing; what is left of the pool fills the batteries,
   oldest first, and the rest is lost. Every building keeps its own
   battery.
9. **A dark building** loses its rules, its program and its senses until
   a power step powers it: no print, no defence, no recharging from it,
   no main flow, no sight or hearing. Its handlers still run; it still
   stands, generates, fills and conducts; and a printer still counts for
   the cap and for its team not being out.
10. **A printer is never dark.** The sim refuses data whose printer
    generates less than its drain and its print drain together, so the
    printer's own power always covers it.
11. **Recharging.** `recharge()` is a bot's action, shaped like `pick`: it
    moves charge from an adjacent powered building's battery into the
    bot's, at the building model's rate. The printer and the new
    `charger` have batteries; the charger's is large and its rate high.
    Any team's bot may recharge from any team's building.
12. **Pylons.** `pylon` is a building model with health and no program,
    built from a site, joining every pylon and building of its team
    within its span whatever stands between. Rivals cut a line by
    destroying one.
13. **Wire** is Q59's, underground: laid and removed by the team's own
    bots with `wire` and `unwire`, for ore, on ground; planned as the
    fourth plan kind, only on ground the team has seen; seen and
    remembered by every team and touched by none but its own. It costs
    more than the pylons that would cover the same ground.
14. **On the map**: a pylon and the lines of its span; wire as Q59 drew
    it; the wire tool on `C`; a mark over a flat bot and a dark building;
    and the inspector's grids, with rates shown per second.

### Consequences

- **Power limits what bots do, not how many there are.** Q59's second cap
  is gone: a team prints to its printers' cap whatever its power, and
  pays for each bot in recharging.
- **No team locks itself.** Bots wake by their own solar and printers
  power themselves, so a team short of everything else still acts and
  still prints.
- **A line can be cut.** Overhead line is cheap and can be destroyed;
  underground wire is dear and safe. Q59's "no sabotage" is withdrawn for
  the one and kept for the other.
- **Power can be stolen.** A rival's bot beside a charger takes its
  charge, as it takes ore from a rival's depot.
- **Sight costs power.** A building's vision is something a grid keeps
  lit, so starving a grid blinds it.
- **Every bot restarts once as it runs low**, whether or not it has a
  hook, so a program's top is where it decides to recharge.

## Outcome

- **Docs:** [08-power.md](../../08-power.md), new — the whole of power.
  [02-machines.md](../../02-machines.md) — the Q9, Q22, Q23 and Q42
  Decided entries, Q59's replaced by Q62's, the deployments, the kind tree
  and table, the attributes, the senses, the actions, Printing, the
  interrupts' effect on the body, and what it leaves open; its Power
  section moved to `08`.
  [execution](../../01-language/execution.md) — the Q8, Q11, Q18 and Q16
  entries and a new Q62 entry, the lifecycle, Metering, the kinds, the hooks, delivery, escalation and the
  limits; [01-language.md](../../01-language.md) — doorway invariant 3;
  [syntax](../../01-language/syntax.md) — the `Power` record.
  [03-world.md](../../03-world.md) — the Q49 entry, the tile's wires and
  a starting printer's battery.
  [04-opposition.md](../../04-opposition.md) — what a team owns.
  [05-progression.md](../../05-progression.md) — power beside printers.
  [06-architecture.md](../../06-architecture.md) — the Q10 entry, the
  tick's steps 2 to 6 and 8, validation at submission, the snapshot, the
  state hash and a fork's stops. [07-interface.md](../../07-interface.md) — the Q41,
  Q53 and Q37 entries, Q59's replaced by Q62's, the inspector, the
  program tree, Machines on the map and Marks.
  [00-overview.md](../../00-overview.md) — the Decided listing and the
  table of docs. [CLAUDE.md](../../../CLAUDE.md) — the list of docs.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T31](../../TASKS.md), rewritten — power in the sim.
- **Task:** [T32](../../TASKS.md), rewritten — power in the renderer.
- **Task:** [T39](../../TASKS.md) — `low_power` in the language crate.
- **Problem:** [P8](../../PROBLEMS.md), rewritten — the docs state Q62 as
  current and none of it is built.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| a bot at zero | **Stops; its own solar wakes it** *(chosen)* | No lock, and no rescue needed; a flat bot crawls on its trickle. |
| a bot at zero | Stops until rescued | A team whose bots all go flat is frozen but not out. |
| a bot at zero | Dies | Frees its place under the cap; running out is lethal. |
| a bot at zero | Never stops, only slows | No lock, and no hard failure to program against. |
| what a call costs | Per tick of the action in progress, thinking free *(the first draft)* | One number per action; senses and `wait` can never cost. |
| what a call costs | **Every builtin has a cost, which may be zero** *(chosen)* | One table, one rule; the price of thinking is a tuning question. |
| low power | **A fifth interrupt that restarts main flow** *(chosen)* | Fits the model: nothing resumes. Every bot restarts once as it runs low. |
| low power | An interrupt that resumes | The first resumption, which Q8 excluded. |
| low power | No interrupt; programs poll | Nothing new in the language; every program polls its battery. |
| its variables | **Cleared, as `fault` and `redeploy` clear them** *(chosen)* | Programs are already written to restart. |
| who recharges bots | **A new charger, and the printer** *(chosen)* | A team starts with one place to recharge. |
| who recharges bots | A new charger only | The first bots run down before one is built. |
| who recharges bots | Every building | No placement decision. |
| whose bots | **Any team's** *(chosen; "that is funny")* | Power can be stolen, as ore can. |
| whose bots | The building's own team | No siphoning; a rule `pick` does not have. |
| a building short of power | Its rules stop | It still sees and thinks. |
| a building short of power | **Fully dark: rules, program and senses** *(chosen)* | Starving a grid blinds it. |
| a building short of power | It takes damage | A starved base falls apart. |
| a dark building | Makes nothing either | It could never recover. |
| a dark building | **Still generates and fills** *(chosen)* | A grid climbs back by itself. |
| the printer | Dark like any building | A lone printer short of its own drain never prints. |
| the printer | **Generates more than it uses, checked at load** *(chosen)* | A standing team always prints. |
| printing | **Drains while it prints** *(chosen)* | Printing eats the surplus bots recharge from. |
| printing | A lump sum to begin | Power as a price per bot. |
| printing | Free, as Q23 ruled | Power touches bots only. |
| stored power | **Each building its own battery, the flow shared** *(chosen)* | A grid that splits divides nothing. |
| stored power | One pool per grid | Simple to read; splitting and joining need a rule. |
| stored power | Batteries never share | Wires carry generation only. |
| the order | **Oldest first, the next tried when one does not fit** *(chosen)* | Deterministic and needs no input. |
| the order | Oldest first, everything newer cut off | One hungry building darkens all after it. |
| overhead wire | **Pylons with a span** *(chosen)* | Free ground under the line; one building to defend per span. |
| overhead wire | A pillar on every tile | A line is a wall. |
| overhead wire | Pillars at ends and corners | Tile-by-tile wire with two kinds of tile. |
| a pylon | **No program** *(chosen)* | Like a site; one fewer deployment. |
| underground wire | **Q59's wire, dearer** *(chosen)* | Nothing new to build or draw. |

### How the answer took its shape

Q59 was a month old and unbuilt when the user asked for "a large rework"
in seven points: a battery on bots that their actions deplete, a
low-power interrupt at 20%, buildings that recharge bots quickly with a
capacity of their own and slow solar built in, a generation and a drain
per tick on every building shown per second, a grid as a group of
connected buildings powered in order of build number, wires overhead on
pillars that occupy space and can be destroyed, and dearer wires
underground. Each reversed something Q59 had chosen on purpose — nothing
stored, only bots drawing, no sabotage — so the rework replaced it
instead of amending it.

Asked what a bot at zero does, the user chose a stop with a solar trickle,
which is what lets buildings drain without the lock Q59 feared. For the
interrupt, a fifth kind that restarts main flow, since every kind the
language has ends that way; for chargers, a new building and the printer;
for overhead wire, pylons with a span. In a second round: a building the
grid cannot cover goes fully dark; printing drains while it runs; every
building keeps its own battery. The drafting's calls were then put to the
user and all but one stood — `recharge` shaped like `pick`, rivals free
to use it ("that is funny"), variables cleared, a dark building still
generating, hooks never held, the three ways buildings join, a pylon
without a program, the underground wire as Q59's. The one overturned was
a drain per tick of an action with thinking free: the user asked that
everything have a possible charge cost, so the cost became a row per
builtin, taken as the call begins. The user added that a printer should
make more than it uses and never go dark, which became a check on data,
and made "its own generation first" the first pass of serving a grid.
