*Closed record — see [../README.md](../README.md). Not spec.*

# Q52 — How does a player debug a program: breakpoints, a forked sim to step and rewind, and a what-if run (amending Q15, Q32 and Q36)

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
read. Rule 13 amends Q32 and changes how a driver paces ticks, not what a
tick does; rule 1 amends Q15 by one read; rule 14 amends Q36's start screen
and replay file.

1. **A fork** is the match's sim's code run on a private copy of the world,
   from a saved state — the match's full world at a completed tick, which
   the driver saves for the debugger, the one read of the match's sim
   beyond the snapshot (amending Q15) — or from `(map, command log)`
   re-run. It is never a peer and never the match's sim, and has entry
   points the match's sim does not: stopping, editing, restoring.
2. **Positions.** A point in a tick is `(tick, machine, the boundaries that
   machine's slice has passed this tick)`, or the tick's end. Stops, hits
   and the fork's log entries are all at positions.
3. **Statements.** A machine begins a statement at the boundary before its
   first operation, on its first line; a compound statement's clauses begin
   each time they are evaluated or tested — `if`, `elif`, `while`, `for`,
   `except`, `case`. Nothing else begins, and a breakpoint on a line where
   nothing can begin moves to the next.
4. **A fork's commands** are those the match's log (or the replay's) holds
   for each tick when the fork runs it, recorded so a re-run takes the
   same; then its own what-if deploys in step 1, and its own edits at their
   positions. A fork is faithful while its own log is empty and every tick
   took the match's complete set; ahead past the match's latest completed
   tick; stale from a tick the match's log later gained a command for; and
   diverged once it has entries of its own.
5. **Breakpoints** sit on a line of the text a window shows, move with it
   as the text is edited, and watch whatever runs exactly that text — its
   content hash, with the deployment for a robot file, scoped to one
   machine or the file — greyed while nothing runs it. They are held by
   the renderer, never a command, never in a log or the hash, and only the
   player's team's machines hit them.
6. **The match's sim reports hits and nothing else.** The driver passes the
   team's watches to `sim.step` beside the tick's commands; `sim.step`
   reports each time a watched machine begins a statement on a watched
   line, with its position, charging nothing and changing nothing.
7. **A hit opens a fork** at the tick's first such hit, from the latest
   saved state before it. In single-player and PvE the driver also submits
   a `SetSpeed(0)`, landing `delay` ticks later (Q12), unless the match is
   paused or a pause is pending; in PvP the match runs on. Further hits
   leave the fork open, are listed by position, and can each reopen it.
   The match's own controls still act on the match.
8. **Steps**: **line** — to the selected machine's next statement in any
   frame; **over** — to its next statement in its current frame or, that
   frame gone, the nearest frame still on its stack; **tick** — to the end
   of the tick; **continue** — to any watched machine beginning a statement
   on a watched line, selecting it. Line, over and continue also stop at
   `dying`, at death and at a restart; every step stops after
   `debug_step_max_ticks` ticks, and in PvP at the live tick. Line and
   over need a selected machine.
9. **Back** returns to the previous stop, from the latest saved state the
   fork may use — its own, or the match's from before its first own entry
   while it was faithful. The driver saves the match's state, and each
   fork its own, every `debug_checkpoint_ticks`, keeping the last
   `debug_checkpoints_kept`, while the debugger is in use.
10. **Inspection** shows, from the fork at the stop: the selected machine's
    frames with file, line, function and locals; the globals of every
    module it has run; its record, action and progress; its tick budget,
    deficit and pending interrupts; and whether it is in a handler, which
    kind, and the hook budget spent. Only the team's machines can be
    selected, and a fork's map is the team's view of it.
11. **Editing a variable** on a paused fork sets a local, or a global of any
    module the machine has run, to a literal or a display of literals —
    a `num` may carry a leading `-` — parsed as `01` parses it and checked
    against `01`'s Limits, entered in the fork's log at its position.
    **What-if** puts the editor's deploys into the fork's log for its next
    tick, after the match's own; the file's breakpoints then hit in the
    fork. *Run* is continue drawn at a chosen speed. *Take it* is an
    ordinary deploy to the match, absent on a replay. A new edit or
    what-if at a position before entries already in the fork's log
    discards them and the saved states after it.
12. **In PvP a fork never passes the live match's latest completed tick
    and never diverges**: edits and what-if are refused, since a
    counterfactual world run to the present shows real hidden state
    through the team's fog. In single-player, PvE and replays a fork may
    run ahead, and sees a scripted team's commands in advance (`04`).
13. **A paused driver wakes only for a player's command** (amending Q32): a
    scripted team is a sender in the log but not a player, and its whole
    script is in the log from the start, so counting it would run a pause
    on to the script's next entry.
14. **Replays** (amending Q36): the start screen, and `--replay <path>`,
    open one; it plays back with the time bar's speed and pause, which also
    seeks. A saved replay records the saver's team, and its file is named
    `<map>-<tick>-<hash>-<team>.ron`, so peers sharing a directory keep
    both. A replay with a recorded team is debugged as it, its windows
    showing a version its log deployed; one without — cards only, or saved
    before this ruling — can only be watched.

### Consequences

- **Determinism holds by construction.** Every rule reads the live sim or
  writes a fork; the one write to the match is a `SetSpeed`, a command
  like any other.
- **The language VM must stop at any boundary** on demand, resume, accept a
  variable's new value, and say which statement and line an operation
  belongs to. `01`'s doorway invariant 3 and execution's Metering name this
  as a tool's act on a copy, outside the language's semantics.
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
  Q36 and Q52 Decided entries, the screen's debugger window, The match and
  the Debugging section. [06-architecture.md](../../06-architecture.md) —
  the Q15, Q32 and Q52 Decided entries, the Crates table, the driver, and
  the Forks section. [01-language.md](../../01-language.md) — doorway
  invariant 3, and [execution](../../01-language/execution.md)'s Metering.
  [04-opposition.md](../../04-opposition.md) — a script never wakes a
  paused driver. [00-overview.md](../../00-overview.md) — fixed point 2
  and the Decided listing. [CLAUDE.md](../../../CLAUDE.md) — determinism
  rules 1 and 5. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T26](../../TASKS.md) — the sim's fork, watches, stepping,
  saving and restoring a state, and variable edits.
- **Task:** [T27](../../TASKS.md) — the renderer's debugger: breakpoints
  bound to versions, the window, the checkpoint policy and rewind, live
  and on replays, and the team a saved replay records.
- **Task:** [T28](../../TASKS.md) — what-if.
- **Task:** [T29](../../TASKS.md) — the paused driver counting only peers'
  commands.
- **Problem:** [P5](../../PROBLEMS.md) — the docs state Q52 as current
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
rebuild the state from `(map, command log)`. Everything else followed from
debugging the copy. The user chose live and replay, a fork for PvP,
line-level stepping, editable variables, what-if in the same ruling, and no
PvP fork past the present, which turns what-if off in PvP. A review of the
first draft found watches keyed without the deployment, steps that could
never stop, rewinds that lost edits and what-if deploys landing mid-tick;
the user then chose breakpoints bound to deployed versions, further hits
counted under an open fork, and replays debugged as the team that saved
them. Two more reviews found the fork with no commands past its opening
tick, line entry counted twice across a call, edits lost or kept on
abandoned timelines, breakpoints greyed by unrelated deploys and blind in
what-if, a PvP edit that sees through fog, a pause that a script's later
commands would run straight through, and the framing rules in `00` and
CLAUDE.md left saying the debugger was forbidden. Two reviews after those
found a fork run ahead of a match whose log was not yet final, edits and
hits with no exact point in a tick, compound statements with no rule,
negative numbers unwritable, *run* undefined, saved states from the match
used across a fork's edits, no way to open a replay, a replay file two
peers in one directory would overwrite, and `01`'s and `04`'s rules left
contradicting the fork; each is answered above, and the PvP refusal of
edits extends the user's ruling against PvP lookahead to the case it was
meant to cover.
