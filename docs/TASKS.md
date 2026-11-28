# Tasks

What is left to build. **This file holds open tasks only** — completing one moves
it to [history/tasks-completed/](history/tasks-completed/README.md) as its own
file, with the commit that finished it. Anything here is open by definition, so
there are no checkboxes to keep in sync.

Numbering is stable — **append new tasks, never renumber**, and never reuse a
number after it moves to history. Entries are written as `**T<n> — <title>**` on
their own line, which is how `scripts/check-registers.mjs` finds them; it rejects
a number that is open and completed at once, and any gap *below* the highest
number recorded. Deleting the highest-numbered entry outright shrinks the range
and goes unnoticed — the same honest limit
[history/questions-answered/](history/questions-answered/README.md) states.

Milestones below are groupings, not entries. A milestone is finished when every
task under it has left the file.

Two conventions carried over from the predecessor project:

- **`⚠HASH` marks a task that changes sim behavior**, and therefore the
  golden-replay hashes. Such a task's PR regenerates the fixture and says why
  (CLAUDE.md). **No other register carries it** — CLAUDE.md and
  `.claude/design-invariants.md` define it, and `docs/history/` may narrate it,
  but no entry outside this file is marked. An open question is not work, and
  nearly every open question would qualify while the sim is unbuilt, so a marker
  on all of them selects nothing. Where a *ruling* cannot be discovered during
  implementation, the question says so in words.
- **Decided-but-unbuilt** work — a ruling the code has not caught up to — is
  tracked here *and* as an entry in [PROBLEMS.md](PROBLEMS.md). The register owns
  the gap; this file owns the work. A task is only lag once it is actually
  buildable; before that it is merely pending.

## M13 — Height and water

What is left of Q65, and [P11](PROBLEMS.md) closes with the last.

**T46 — Height and water in `render`: what is left**

`07`'s Q65 entry is drawn but for the bot's own movement: tiles at their
heights, water as a sheet tipped to its slope, damp ground, a spring's
and a sink's mark, a ramp at a step of one level, the ground row with
*lower*, *raise*, *level* and its clear, a ground plan's outline and its
ghost, the flooded mark and the inspector's lines. What is left is the
bot, now that `03`'s Moving is built: stepping between heights at the
speed its pace gives, and shown falling and bonked.

**T47 — The starter library and the shipped maps know height and water**

`07`'s starter tree and `data/maps`. The first map gains heights, a
lake with a spring and ground worth digging, kept fair between its
teams. The starter's `nav` steps only where a step is allowed, never
drops by accident, keeps a bot moving so that it keeps its pace, and
treats deep water as closed; `jobs` gains a ground step that carries
out the team's ground plans — mining, digging and filling, taking earth
to a depot or to a fill and fetching it when a fill needs it — and
`act` its guarded calls; a flooded bot digs or fills its way out, or
waits. The Fool keeps working on a map with heights. The starter tests
run a match in which a channel is planned and dug and no bot faults.

## M14 — The water network

Q67, in the order to build it, and [P12](PROBLEMS.md) closes with the
last.

**T48 — The water network in `sim`** `⚠HASH`

`09`'s The water network. The models `pump`, `tank`, `spout` and
`aqueduct`, with their rows in `data/machines.toml` and deployments for
the first three; `tank`, `water_drain`, `print_water`, `well`,
`pump_rate` and `pour_rate` on a model, in levels, and a tank on the
printer. The pipe: a sixth plan kind in `Mark` and `Unmark` and on the
wire, what drops one on its tick and what consumes it, `pipe` and
`unpipe` with their cost and ticks, and a tile's `pipes`. Networks
traced as grids are, by pipe, adjacency and an aqueduct's span. The
network step in step 7 before the water step: each pump's source, the
needs, the spouts and the tanks, in `09`'s order, with every part
lifted, poured or used accounted for, and the figures kept. `watered`
and a dry building stopped as a dark one is, a print held and none
begun; `pouring` and `pour`; `water()` and `pipes()`; a tank's water
given to the ground water when its building leaves the world. Every
new attribute in the record, the snapshot and the hash. Tests: a
network joined each way and cut by a deconstructed aqueduct; a need
met by a pump, by a tank, and by neither; oldest served first when
short; a pump with no wet tile, one unpowered and one flooded; two
pumps on one tile; a spout pouring each way, onto rock and off the
map; a pond pumped dry and a hollow filled from it uphill; the tiles,
the ground water and the tanks never changing in sum; a dry printer
that prints nothing; and a team that lays no pipe playing as before.
The golden replay gains a pump, a pipe, a tank and a spout that fills
a hollow.

**T49 — The water network in `render`**

`07`'s Q67 entry. Pipes drawn as wire is, in water's color, and an
aqueduct's joins as lines; the four models' bodies, a tank that shows
how full it is and a spout that shows which way it pours; the dry
mark; the pipe tool and its clear on `T`, its ghost and its plan; the
building row's entries past the ninth; and the inspector's lines for a
machine's water and the team's networks.

**T50 — The starter library pipes water**

`07`'s starter tree. A program each for `pump`, `tank` and `spout`,
the spout's pouring toward the lowest ground beside it; `jobs` lays
and takes up planned pipe as it does wire, and `act` gains the guarded
calls. The starter tests run a match in which a pump, pipe and a spout
are planned and built and a hollow fills.

## M15 — The corpus

**T51 — Split `02` and `07` into a doorway and parts**

Both stand within two kilobytes of the size cap (I14), and Q67 had to
put the rows it adds to a machine in `09` for want of room in `02`.
Each splits as CLAUDE.md's *Splitting a doc* has it: a doorway holding
the invariants that cross its parts and a table of what each part
owns, and parts that open with their breadcrumb. `02`'s Decided
entries, attributes, senses and actions want a part each, and with
room made, the models, attributes, senses and actions Q67 listed in
`09` move back to `02`, with `09` citing them. Every link into either
doc still resolves, and the checks that read a doc's layout pass.
