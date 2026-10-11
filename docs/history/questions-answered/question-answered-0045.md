*Closed record — see [../README.md](../README.md). Not spec.*

# Q45 — How does a player debug a program: breakpoints, a forked sim to step and rewind, and a what-if run (amending Q32)

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

None changes the match's behaviour or a hash: the match's sim is only
read. Rule 12 amends Q32 and changes how a driver paces ticks, not what
a tick does.

1. **A fork** is a private copy of the world from a saved state, or
   `(map, command log)` re-run, at a completed tick of a match or a
   replay, agreeing with its state hash there. It runs the tick loop
   unchanged; it is never a peer and never the match's sim, and has entry
   points the match's sim does not: stopping mid-tick, editing, restoring.
2. **A fork's commands** are the match's (or the replay's) log for each
   tick it runs, which it reads and never writes, then its own log — the
   what-if deploys and edits. A fork with an empty own log is faithful and
   reproduces the match hash for hash; one with entries is diverged.
3. **Statements.** Every rule naming a line means a machine *beginning a
   statement* on it — the boundary before the statement's first
   operation; returning from a call into the middle of a statement begins
   nothing.
4. **Breakpoints** are on a line of the text a file window shows, carried
   with their lines as the text is edited, and scoped to the selected
   machine or to every machine of the team whose bundle holds the file.
   Each watches the machines running exactly that text — the file's
   content hash, with the deployment for a robot file — and is greyed
   while nothing runs it. They are held by the renderer, never a command,
   never in a log or the hash, and only the player's team's machines hit
   them.
5. **The match's sim reports hits and nothing else.** The driver passes the
   team's watches to `sim.step` beside the tick's commands; `sim.step`
   reports each watched machine that begins a statement on a watched line,
   charging nothing and changing nothing, so a peer watching lines hashes
   as one that is not.
6. **A hit opens a fork** at it, from the latest saved state before the
   hit's tick. In single-player and PvE the driver also submits a
   `SetSpeed(0)`, landing `delay` ticks later (Q12), unless the match is
   paused or a pause is pending; in PvP the match runs on. Further hits
   while a fork is open leave it open, are counted, and can each reopen
   it.
7. **Steps**: **line** — to the selected machine's next statement in any
   frame; **over** — to its next statement in its current frame or, that
   frame gone, in the nearest frame still on its stack; **tick** — to the
   end of the tick; **continue** — to a watched machine beginning a
   statement on a watched line. Line, over and continue also stop at
   `dying`, at death and at a restart; every step stops after
   `debug_step_max_ticks` ticks, and in PvP at the live tick. Each stop
   reports its cause. Line and over need a selected machine.
8. **Back** returns to the previous stop, from the latest saved state
   before it. The renderer saves the match's state, and each fork's, every
   `debug_checkpoint_ticks`, keeping the last `debug_checkpoints_kept`.
9. **Inspection** shows, from the fork at the stop: the selected machine's
   frames with file, line, function and locals; the globals of every module
   it has run; its record, action and progress; its tick budget, deficit
   and pending interrupts; and whether it is in a handler, which kind, and
   the hook budget spent. Only the team's machines can be selected, and a
   fork's map is the team's view of it, fog and memory as Q21 rules.
10. **Editing a variable** on a paused fork sets a local, or a global of any
    module the machine has run, to a literal or a display of literals,
    parsed as `01` parses it and checked against `01`'s Limits. The edit
    enters the fork's log at its tick, machine and operation count, so a
    re-run applies it where it was made. **What-if** puts the editor's
    deploys into the fork's log for its next tick, after the match's own;
    the file's breakpoints then hit in the fork, since the fork runs that
    text. *Take it* is an ordinary deploy to the match. A new edit or
    what-if at a point before entries already in the fork's log discards
    them and the saved states after it.
11. **In PvP a fork never passes the live match's latest completed tick
    and never diverges**: edits and what-if are refused, since a
    counterfactual world run to the present shows real hidden state
    through the team's fog. In single-player, PvE and replays a fork runs
    ahead freely, and sees a scripted team's commands in advance (`04`).
12. **A paused driver counts only peers' commands** (amending Q32): at
    speed `0` it runs to the next tick carrying a player's command, not a
    scripted team's, whose whole script is in the log from the start —
    otherwise a pause, a debugger's or a player's, would run on to the
    script's next entry.
13. **A replay** records the team of the peer that saved it, outside its
    identity, and is debugged as that team; its file windows show the text
    of a version the replay ran, picked from those its log deployed. A
    replay with no recorded team — cards only, or saved before this ruling
    — can be watched but not debugged.

### Consequences

- **Determinism holds by construction.** Every rule reads the live sim or
  writes a fork; the one write to the match is a `SetSpeed`, a command
  like any other.
- **The language VM must stop at operation boundaries** on demand and say
  which file and line an operation is on. Both exist conceptually (`01`,
  Metering); the fork needs them as an API.
- **PvP keeps fog.** A fork holds the whole world, as every lockstep peer's
  sim does, so the debugger shows only what the team's renderer would, and
  a PvP fork never diverges, so it can show nothing the match will not.
- **The framing holds for the match.** The renderer still never writes the
  match's sim (`00`, CLAUDE.md rules 1 and 5); a fork is a copy, named as
  the one exception in both.
- **An in-language `breakpoint()`** is not part of this ruling; it would
  need a builtin with a cost row, and is a new question number if wanted.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the preamble, the
  Decided entry, the screen's debugger window, and the Debugging section.
  [06-architecture.md](../../06-architecture.md) — the Q15 and Q32 Decided
  entries and Q45's, the driver, and the Forks section.
  [00-overview.md](../../00-overview.md) — fixed point 2 and the Decided
  listing. [CLAUDE.md](../../../CLAUDE.md) — determinism rules 1 and 5.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T26](../../TASKS.md) — the sim's fork, watches, stepping,
  saving and restoring a state, and variable edits.
- **Task:** [T27](../../TASKS.md) — the renderer's debugger: breakpoints
  bound to versions, the window, the checkpoint policy and rewind, live
  and on replays, and the team a saved replay records.
- **Task:** [T28](../../TASKS.md) — what-if.
- **Task:** [T29](../../TASKS.md) — the paused driver counting only peers'
  commands.
- **Problem:** [P5](../../PROBLEMS.md) — the docs state Q45 as current
  until T26 to T28 land.
- **Problem:** [P6](../../PROBLEMS.md) — the driver still counts a scripted
  team's commands when paused, until T29.

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
| breakpoint versions | Follow edits by matching lines | Friendlier, and can land on the wrong statement. |
| breakpoint versions | A plain line number | Any version hits line N, often unrelated code. |
| breakpoint versions | **The deployed version; greyed while ahead** *(chosen)* | Stops only on the code that runs. Refined in review to the file's exact text by content hash, so a breakpoint survives deploys of other files, follows its line through edits, and hits a what-if fork running that text. |
| PvP edits | Allowed, as elsewhere | A counterfactual run to the present shows real enemy state through fog. |
| PvP edits | **Refused; a PvP fork never diverges** *(chosen in review)* | Edits stay in single-player, PvE and replays, beside what-if. |
| further hits | The newest replaces the fork | What the player was inspecting is lost. |
| further hits | **Keep the fork, count them** *(chosen)* | One pause, and no lost state. |
| a replay's team | Any team, picked on opening | Every replay debuggable; the file needs nothing new. |
| a replay's team | **The saver's team, recorded** *(chosen)* | A replay is debugged as the player who played it; cards-only replays are not. |
| what-if | Later, its own question | A smaller ruling now. |
| what-if | **Ruled with the debugger** *(chosen)* | The fork already exists; deploying into it is one rule more. |

### How the answer took its shape

Lockstep forbids pausing one peer or stepping one machine in the real
match, and the same determinism makes a private copy exact: any peer can
rebuild the state from `(map, command log)`. Everything else followed
from debugging the copy. The user chose live and replay, a fork for PvP,
line-level stepping, editable variables, what-if in the same ruling, and
no PvP fork past the present, which turns what-if off in PvP. A review of
the first draft found watches keyed without the deployment, steps that
could never stop, rewinds that lost edits and what-if deploys landing
mid-tick; the user then chose breakpoints bound to deployed versions,
further hits counted under an open fork, and replays debugged as the team
that saved them. Two more reviews found the fork with no commands past its
opening tick, line entry counted twice across a call, edits lost or kept
on abandoned timelines, breakpoints greyed by unrelated deploys and blind
in what-if, a PvP edit that sees through fog, a pause that a script's
later commands would run straight through, and the framing rules in `00`
and CLAUDE.md left saying the debugger was forbidden; each is answered
above, and the PvP refusal of edits extends the user's ruling against
PvP lookahead to the case it was meant to cover.
