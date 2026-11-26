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

**Status 2026-11-24: 11 opened, 9 fixed — 2 open.**

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

**P11 — The code lags Q65: no height, no digging, and water that is still terrain**

Q65 ruled that every tile has a height bots change, and that water is a
conserved volume standing on the ground; `02`, `03`, `06`, `07`, `08` and
`09` state it as current. None of it is built. In the sim a tile has no
height, a move tests only terrain, sight tests only rock, and `water` is
a third terrain kind that never moves; there is no `dig`, `fill` or
`mine`, no `earth`, no ground plan, no spring or sink, no water step and
no `flooded`. The renderer draws a flat map with water as a sunken tile.
The maps and the starter library know nothing of height. The work is
[T43](TASKS.md), [T44](TASKS.md), [T45](TASKS.md), [T46](TASKS.md) and
[T47](TASKS.md), and this entry closes with the commit that finishes the
last of them.
