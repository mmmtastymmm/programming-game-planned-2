# Known Problems Register

Defects found in the design docs that are **not open questions** — nobody is
undecided about them. Each is a decision that was made and then contradicted, a
number that does not survive arithmetic against the constants it derives from,
or a ratified decision the implementation never caught up to.

**This file holds open entries only.** Fixing one moves it into
[history/problems-fixed/](history/problems-fixed/README.md) as its own file, with
the commit that closed it. What stays here is the working set; what leaves is the
record.

Numbering is stable — **append new problems, never renumber**, and never reuse a
number after it moves to history. Open design questions still go in
[QUESTIONS.md](QUESTIONS.md); this file is for text that is already wrong **or a
ruling the code has not caught up to** — never for anything still undecided.
Unsorted observations start in [INBOX.md](INBOX.md) and become numbered entries
here once triaged. Answered questions are the other source: a ruling whose
consequence is that existing text or code is now wrong opens an entry here and
cites it from its `## Outcome` section.

An implementation-lag entry is fixed by *code*, so it closes with the
implementing commit rather than with a docs edit, and the work itself is tracked
in [TASKS.md](TASKS.md). Living in both files is the normal pipeline, not
duplication: the register owns the **gap**, the task owns the **work**.

Entries are written as `**P<n> — <short title>**` on their own line, which is
how `scripts/check-registers.mjs` finds them. The status headline below is
**derived** — open entries counted here, fixed entries counted in
[history/problems-fixed/](history/problems-fixed/README.md) — and machine-checked
on every CI run. Do not hand-edit half of it, and do not restate it anywhere
else; other docs cite entries and their relationships, never totals. There is one
status block and it is rewritten in place: `git log -p` is the history.

**Status 2026-10-12: 8 opened, 3 fixed — 5 open.**

## Open

**P3 — No rival bot can deconstruct a printer at today's numbers**

`05`'s table, `02`'s Q30 bullet and `04`'s ending all name a rival's
deconstruction as a way to lose a printer, and the numbers in
`data/machines.toml` make it impossible. A bot adjacent to a rival printer
takes `defence_damage` (1) every tick, so its `health` (10) reaches zero on
the tenth and it goes `dying`, which cancels the action in progress; the
printer's `deconstruct_ticks` is 30, and a second bot cannot help, since
each runs its own action. Attacking is possible but slow: a bot lands about
three hits of `attack_damage` (1) before it falls, so a printer's `health`
(60) takes some twenty bots, four at a time. The numbers were set for Q30's
conversion and never rechecked against deconstruction once Q42 made it the
way to take a printer down. The user deferred retuning them to
playtesting; this entry holds the arithmetic until then, and closes with
the commit that changes the numbers or the rule.

**P5 — The code lags Q55: no forks, breakpoints or what-if**

Q55 ruled that a program is debugged on a fork, and `06`'s Forks section
and `07`'s Debugging section and The match state it as current: a saved
state of the match, watched lines passed to `sim.step`, forks that stop
mid-tick, step, save and restore, editable variables, what-if, and
replays opened from the start screen, recording their team. None of it
exists yet: `sim.step` takes commands only, the language VM cannot stop
mid-slice, the renderer has no breakpoints, debugger window or replay
entry, and a replay records no team. The work is [T26](TASKS.md),
[T27](TASKS.md) and [T28](TASKS.md); this entry closes with the commit
that finishes whichever of them lands last.

**P6 — A pause runs on to a scripted team's next command**

Q55 amended Q32 so a paused driver runs only to the next tick carrying a
player's command, a scripted team's not counting. `render`'s driver still
takes the next tick carrying any command, through `net`'s
`next_tick_with_a_command`, and the driver puts a scripted team's whole
script into the log when it is built, a script being a command log written
in advance (`04`). So a pause in PvE, a player's or a
debugger's, runs on without the clock to the script's next entry. It is
latent today only because the Fool's script has nothing after tick 0. The
work is [T29](TASKS.md).

**P7 — A `while` header is charged once per loop, not once per pass**

`01`'s costs part charges a statement each time it executes, a `while`
header once per pass, and `06`'s Forks has a `while` begin again on each
pass for that reason. The compiler charges the header's `op.statement` once,
before the loop's start (`crates/lang/src/compile.rs`, the `While` arm), so
a loop of a hundred passes pays for its header once, and every program with
a `while` loop runs cheaper than ruled — the starter's and the Fool's robots
among them. Found in review of Q55, whose statement rule would otherwise be
built on the lagging charge. The work is [T30](TASKS.md).

**P8 — The code lags Q58: no power, wires or solar panels**

Q58 ruled that power is a team's flow, supplied by printers and solar
panels wired back to a printer and drawn by bots and some buildings, with
the newest machines stopping when the team is short; `02`, `03`, `06` and
`07` state it as current. None of it exists: the sim has no wires, no
`solar` model, no power step and no `powered` attribute, printing ignores
power, and the renderer draws none of it. The work is [T31](TASKS.md) and
[T32](TASKS.md); this entry closes with the commit that finishes whichever
lands last.
