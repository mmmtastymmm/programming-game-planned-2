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

**Status 2026-09-06: 1 opened, 0 fixed — one open.**

## Open

**P1 — Determinism rule 7's bundle hash is not injective**

Rule 7 as written — "the hash of every file's bytes taken in sorted name
order" — and the layout `docs/01-language/syntax.md` first pinned from it
(each file's bytes then a zero byte, names not hashed) let two different
bundles share one version. `{main.py: "x\0y"}` and `{main.py: "x", z.py:
"y"}` hash the same byte stream, and two bundles differing only in the name of
a file nothing imports hash the same. The version is part of the state hash,
so two peers running different programs could agree. The fix length-prefixes
each file's name and its bytes, as `crates/sim/src/hash.rs` already does for
strings, and amends rule 7's wording to say so. Found by review of the
uncommitted `docs/01`, before any code depended on the layout.
