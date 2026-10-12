# Open Design Questions

**This is the only file that holds open questions, and it holds nothing else.**
Other docs may cite a number inline ("open — `Q73`") but never restate a question's
substance or its leaning; a question restated in two places gets answered in one
of them.

Answering a question empties it out of this file entirely: the ruling and its
worksheet become one new file in
[history/questions-answered/](history/questions-answered/README.md), which must
declare its **Outcome** in the spellings the checker accepts — `- **Docs:**` for
edits made in the same commit, `- **Question:**`, `- **Problem:**` or
`- **Task:**` for what it opened.
A ruling that changes nothing anywhere was not a ruling. Nothing answered stays
here. That discipline is
the whole reason this file is readable: the predecessor project let answered
material stack up and reached 166 KB, at which point everyone was reading it in
full to find the handful of things still open.

Entries are written as `**Q<n> — <short title>**` on their own line, which is how
`scripts/check-registers.mjs` finds them; a heading or any other spelling is not
an entry, and the register will not see it. Numbering is append-only — **never
renumber**, and never reuse a number after it moves to history.

Known-wrong *decided* text is not a question — it goes in
[PROBLEMS.md](PROBLEMS.md). Raw unsorted observations go in
[INBOX.md](INBOX.md).

**Status 2026-10-11.** Every question this file has issued, Q1 through
Q52, is answered. Q33 through Q39 and Q41 were each opened and answered in
one commit as [07-interface](07-interface.md) was written and revised, and
their rulings are `07`'s Decided entries — Q39 amending Q33's panel to a
window, Q41 the working copies to one tree and the hook binding in `01`.
Q42 — building printers, printing by turns, one deployment per model —
amends Q9, Q23, Q24, Q30, Q31, Q35 and Q40, and withdraws Q31 and Q40
outright, so neither has a Decided entry anywhere; Q43, a bot's
program-set color and the player-colored team outline, amends Q35 and
Q42; Q44, the `Shift`+drag rectangle and the withdrawing *clear*, amends
Q37 and is `07`'s; Q45, a bot's team shown by its lights, amends Q43 and
is `07`'s; Q46, a bot's entity id on its top face, amends Q42 and Q43 and
is `07`'s; Q47, lights on printers and depots, amends Q45 and is `07`'s; Q48, a
printer's trim in its team's color, amends Q47 and is `07`'s. Q49, paint
and overlays only on ground the team has seen, is
[03-world](03-world.md)'s, and its preview of a paint plan over paint
amends Q37; Q50, every plan previewed as a smaller version of its effect,
amends Q37 and Q49 and is `07`'s; Q51, a smaller overlay marker and no
cursor ghost over the same plan, amends Q50 and Q37 and is `07`'s; Q52,
free docstrings, is [01-language](01-language.md)'s syntax part. The rulings of Q7, Q9, Q21, Q22, Q23, Q24, Q25, Q30, Q42 and Q43 are
[02-machines](02-machines.md)'s Decided entries, `docs/03` cites Q21 and
Q22 from there, the rulings of Q6, Q10, Q12, Q15, Q26 through Q28 and Q32
are [06-architecture](06-architecture.md)'s, Q4 and Q29 are
[04-opposition](04-opposition.md)'s, Q42 is also
[05-progression](05-progression.md)'s, and Q43 is also `07`'s. Nothing is
undecided, and **Open** below is empty until the next question is opened.
The
framing rulings are owned by [00-overview.md](00-overview.md)'s Decided section
and the language's by [01-language](01-language.md)'s parts; none is repeated
here. Each worksheet is a file of its own in
[history/questions-answered/](history/questions-answered/README.md), and the
spike behind Q5 is
[spikes/lang-determinism](../spikes/lang-determinism/README.md).

This block is **rewritten in place**, never stacked or archived: `git log -p
docs/QUESTIONS.md` already records every status this file has carried, dated and
attached to the commit that changed it.

**Every numbered doc is written** — [01-language](01-language.md),
[02-machines](02-machines.md), [03-world](03-world.md),
[04-opposition](04-opposition.md), [05-progression](05-progression.md),
[06-architecture](06-architecture.md) and [07-interface](07-interface.md),
each with the rulings it waited on as its Decided entries or citing them
from `02`. `02` opened Q24 and `04` opened Q29 in the writing, and both are
answered; `05` was written from Q30, opened and answered for it, and
rewritten from Q42; `07` from Q33 through Q39, the last three opened in its
writing and answered after. The overview's table says the same.

## Open

*(none — every question issued so far is answered)*
