# Known Problems Register

Defects found in the design docs that are **not open questions** — nobody is
undecided about them. Each is a decision that was made and then contradicted, a
number that does not survive arithmetic against the constants it derives from,
or a ratified decision the implementation never caught up to.

**This file holds open entries only.** Fixing one moves it out, into the current
[history/problems-fixed/](history/problems-fixed/README.md) as its own file,
with the commit that closed it. What stays here is the working set; what leaves is the record.

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

**Status 2026-08-18: 0 opened, 0 fixed — zero open.**

The register is empty because the design corpus is empty. The first entries will
arrive with the first rulings.

## Open

*(none yet)*
