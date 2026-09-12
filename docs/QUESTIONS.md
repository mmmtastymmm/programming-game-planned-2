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

**Status 2026-09-12.** Every question this file has issued is answered
except the three under **Open** below, which the writing of
[07-interface](07-interface.md) raised. Q33, Q34 and Q35 were each opened and
answered in one commit, and their rulings are `07`'s Decided entries. The
rulings of Q7, Q9, Q21, Q22, Q23, Q24 and Q25 are
[02-machines](02-machines.md)'s Decided entries, `docs/03` cites Q21 and Q22
from there, the rulings of Q6, Q10, Q12, Q15, Q26 through Q28 and Q32 are
[06-architecture](06-architecture.md)'s, Q4 and Q29 are
[04-opposition](04-opposition.md)'s, and Q30 is
[05-progression](05-progression.md)'s. The
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
each with the rulings it waited on
as its Decided entries or citing them from `02`. `02` opened Q24 and `04`
opened Q29 in the writing, and both are answered; `05` was written from Q30,
opened and answered for it; `07` from Q33 through Q35, and it opened the
three below. The overview's table says the same.

## Open

**Q36 — What does the interface show before the first tick and after the last?**

A match today starts from the command line — a map, opposition directories,
a host or a join address — and ends with a line in the top bar. The
question is what the player sees instead: a start screen picking the map
and the cards (`04`), hosting or joining (`06`), and reading the programs
directory in (Q33); and an end screen naming the winner or the draw, with
the replay `(map, command log)` saved to a file for the report `06`
promises. *Leaning:* both screens in the game, the command line kept for
the headless driver and CI; the replay always saved, since a desync report
without one is nothing.

**Q37 — How are the mark tools picked, and how do the team's own plans read on the map?**

The tools are a building model, one of eight paint colors, one of the
overlay labels in data, a clear and a withdrawal (Q10, Q26, Q27), and
today they are five radio buttons. The question is the strip that holds
them — hotkeys, whether a drag paints a run of tiles, whether a tool stays
armed — and how a tile carrying the team's plan is drawn, since a plan is
private to its team (`03`) and a program consumes it. *Leaning:* a tool
strip with every color and label, a drag paints, a tool stays armed until
`Esc`, and a planned tile shows a translucent version of what the plan
would make it.

**Q38 — What may the player do while stalled on a peer, or after a desync?**

`06` reports a stall after `stall_report_ticks` and stops the driver on a
desync, and says what the player may do then is the renderer's. *Leaning:*
a banner naming the peer awaited with a resign button beside it; on a
desync, the match frozen with the report, the replay saved, and a button to
leave.
