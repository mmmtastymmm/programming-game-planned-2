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

**Status 2026-10-11: 6 opened, 1 fixed — 5 open.**

## Open

**P2 — The code lags Q43: no `set_color`, no `color`, no outlines**

Q43 gave bots a `set_color(color)` builtin and a `color` attribute, and
every team an outline the player colors. `docs/02` states the builtin, the
attribute and the `dying` hook's use of it as current, its Costs section
says every builtin above has a row in `data/language/costs.toml`, and
`docs/07` names the neutral body color, the default outline color and the
outline list as constants in `data/interface.toml`. None exists yet: the
sim has no `set_color` or `color`, the cost table has no row for it, the
renderer draws bots white and teams as rings, and `data/interface.toml`
holds only the ring list. The work is
[T25](TASKS.md); this entry closes with the commit that finishes it.

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

**P4 — The code lags Q37: the tools are a list, not the mark strip**

Q37 ruled the mark tools a strip of three rows from data — building
models, paint colors, overlay labels — each ending in a clear, picked with
`B`/`P`/`O` and `1`–`9`, armed until `Esc`, with drag marking and a plan
preview; `07` states it as current. The renderer still offers a short
fixed list of tools on keys `1`–`6`, marks one tile per click, and draws no
preview. The work is [T20](TASKS.md), open since before Q42; this entry
records the gap the registers require beside it, and closes with the
commit that finishes T20.

**P5 — The code lags Q45: no forks, breakpoints or what-if**

Q45 ruled that a program is debugged on a fork, and `06`'s Forks section
and `07`'s Debugging section state it as current: watched lines passed to
`sim.step`, forks that stop mid-tick, step, save and restore, editable
variables, what-if, and a saved replay that records its team. None of it
exists yet: `sim.step` takes commands only, the language VM cannot stop
mid-slice, the renderer has no breakpoints or debugger window, and a
replay records no team. The work is [T26](TASKS.md), [T27](TASKS.md) and
[T28](TASKS.md); this entry closes with the commit that finishes the
last of them.

**P6 — A pause runs on to a scripted team's next command**

Q45 amended Q32 so a paused driver runs only to the next tick carrying a
peer's command. `render`'s driver still takes the next tick carrying any
command, through `net`'s `next_tick_with_a_command`, and a scripted
team's whole script enters the log when the driver is built (`04`). So a
pause in PvE, a player's or a debugger's, runs on without the clock to
the script's next entry. It is latent today only because the Fool's
script has nothing after tick 0. The work is [T29](TASKS.md).
