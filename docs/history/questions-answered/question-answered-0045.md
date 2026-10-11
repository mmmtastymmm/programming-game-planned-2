*Closed record — see [../README.md](../README.md). Not spec.*

# Q45 — How does a player debug a program: breakpoints, a forked sim to step and rewind, and a what-if run

## Ruling

*Ruling (2026-10-11):* **a player debugs on a fork — a private copy of the
sim at a completed tick, which never feeds back into the match.** Line
breakpoints are the editor's, the live sim only reports reaching them,
and a hit opens a fork there. On a fork the player steps by line, by tick
or over a waiting action, rewinds, inspects the selected machine's
execution, and may edit its variables. A **what-if** run deploys the
editor's text on a fork and runs it forward, so a change can be tried
against the world before it is deployed. The same tool works on a live
match and on a saved replay. In PvP a fork never runs past the live tick,
and what-if is off.

### The rules

None changes the sim's behaviour or a hash: the live sim is only read.

1. **A fork** is the world at a completed tick of a match or a replay,
   with its command log to that tick: built by cloning the live sim or by
   re-running `(map, command log)`, either way agreeing with the match's
   state hash at that tick. It runs the tick loop exactly as the match
   does, but it is never a peer: it sends nothing, receives nothing, and
   no command or result of it reaches the match, except the one pause in
   rule 4.
2. **Breakpoints** are a line of a file in the player's tree, held by the
   renderer, never a command, never in the log or the hash. Each is scoped
   to the selected machine or to every machine whose bundle holds the
   file. Only the player's own team's machines can hit one.
3. **The live sim reports a hit, and nothing else.** While breakpoints are
   set, `sim.step` reports each machine of the player's team that begins
   an operation on a watched line, with the tick; it charges nothing and
   changes nothing, so a peer with breakpoints hashes as one without.
4. **A hit opens a fork** at the start of the hit's tick, run forward to
   the hit. In single-player and PvE the driver also submits a
   `SetSpeed(0)`, which pauses the match `delay` ticks later like any
   command (Q12). In PvP the match runs on.
5. **Stepping** on a fork advances the whole world in tick-loop order and
   stops on the selected machine: **line** — until it begins an operation
   on another line, or in another frame; **step over** — until it next
   begins an operation in the current frame, so a call, or a waiting
   action such as `wait(60)` or `move`, is one step; **tick** — to the end
   of the tick; **continue** — to the next breakpoint. Every machine's
   metering is the match's, so a fork left alone reproduces the match.
6. **Rewind** steps back to any earlier stop by restoring the nearest
   checkpoint at or before it and re-running. Checkpoints are kept every
   `debug_checkpoint_ticks` of the live match and of every fork, the last
   `debug_checkpoints_kept` of them, both tuning constants in
   `data/interface.toml`.
7. **Inspection** shows the selected machine's call stack, each frame with
   its file, line, function and locals, its globals, its action in
   progress, its tick budget and deficit, its pending interrupts and
   whether it is in a handler, and its attribute record. Only the player's
   own team's machines can be selected, and the map a fork draws is the
   team's view of it, fog and memory as Q21 rules, never the whole world.
8. **Editing a variable** on a paused fork sets a local or a global of the
   selected machine to a value written as a language literal and checked
   as the language would; the fork is marked **diverged** from then on,
   and no longer claims to be the match.
9. **What-if** deploys the editor's current text, as the same `Deploy`s a
   real deploy would send, into a fork at its current point, and runs it
   forward at any speed, with the map drawing the fork under a banner.
   Taking the change is an ordinary deploy to the match, `delay` ticks
   later from the match's own tick, so the result can differ from the
   fork's.
10. **In PvP no fork runs past the live match's latest completed tick**: a
    step that would cross it waits, and what-if is unavailable. In
    single-player, PvE and replays a fork runs ahead freely; a scripted
    team's commands are known in advance (`04`), so a fork sees them.
11. **A replay** opens with the same tool: the replay's log is the fork's,
    and there is no live tick to stay behind.

### Consequences

- **Determinism holds by construction.** Every rule reads the live sim or
  writes a fork; the one write to the match is a `SetSpeed`, a command
  like any other.
- **The language VM must stop at operation boundaries** on demand and say
  which file and line an operation is on. Both exist conceptually (`01`,
  Metering); the fork needs them as an API.
- **PvP keeps fog.** A fork holds the whole world, as every lockstep peer's
  sim does, so the debugger shows only what the team's renderer would.
- **An in-language `breakpoint()`** is not part of this ruling; it would
  need a builtin with a cost row, and is a new question number if wanted.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided entry,
  the screen's debugger window, and the Debugging section.
  [06-architecture.md](../../06-architecture.md) — the Decided entry and
  the Forks section. [00-overview.md](../../00-overview.md) — the Decided
  listing. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T26](../../TASKS.md) — the sim's fork, stepping, hit
  reporting and variable edits.
- **Task:** [T27](../../TASKS.md) — the renderer's debugger: breakpoints,
  the window, checkpoints and rewind, live and on replays.
- **Task:** [T28](../../TASKS.md) — what-if.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| where | Replays only | Simplest, but a bug is found after the match it lost. |
| where | **Live and replays** *(chosen)* | One tool on two sources; the live half needs hit reporting. |
| PvP hit | Pause both players | Fair, but one player can stop the other's game at will. |
| PvP hit | Debugger off in PvP | No abuse, and no debugger where programs matter most. |
| PvP hit | **Fork; the match runs on** *(chosen)* | The player debugs while the match continues without them. |
| stepping | Ticks only | Cheap; a tick can run a whole loop, so a line is lost in it. |
| stepping | Every operation | Exact to the cost model; dozens of steps per line. |
| stepping | **Line, tick and step over** *(chosen)* | What a programmer expects; the VM stops at boundaries. |
| PvP lookahead | Unlimited | A fork simulates the opponent's future; the game becomes prediction. |
| PvP lookahead | A capped horizon | A middle ground with a number to tune and argue about. |
| PvP lookahead | **Never past the present** *(chosen)* | No prediction; what-if is lost in PvP. |
| variables | Read-only | A fork stays faithful; a hypothesis needs a code edit. |
| variables | **Editable, the fork marked diverged** *(chosen)* | Fast hypothesis tests; the fork stops claiming to be the match. |
| what-if | Later, its own question | A smaller ruling now. |
| what-if | **Ruled with the debugger** *(chosen)* | The fork already exists; deploying into it is one rule more. |

### How the answer took its shape

Lockstep forbids pausing one peer or stepping one machine in the real
match, and the same determinism makes a private copy exact: any peer can
rebuild the state from `(map, command log)`. Everything else followed
from debugging the copy. The user chose live and replay, a fork for PvP,
line-level stepping, editable variables, what-if in the same ruling, and
no PvP fork past the present, which turns what-if off in PvP.
