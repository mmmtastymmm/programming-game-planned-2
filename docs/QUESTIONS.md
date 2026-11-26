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

**Status 2026-11-24.** Every question this file has issued is answered
except the one under **Open** below, which I12 raised. Q33 through Q39 and Q41 were each opened and answered in one
commit as [07-interface](07-interface.md) was written and revised, and
their rulings are `07`'s Decided entries — Q39 amending Q33's panel to a
window, Q41 the working copies to one tree and the hook binding in `01`.
Q42 — building printers, printing by turns, one deployment per model —
amends Q9, Q23, Q24, Q30, Q31, Q35 and Q40, and withdraws Q31 and Q40
outright, so neither has a Decided entry anywhere; Q43, a bot's program-set
color and the player-colored team outline, amends Q35 and Q42; Q44, the
`Shift`+drag rectangle and the withdrawing *clear*, amends Q37 and is
`07`'s; Q45, a bot's team shown by its lights, amends Q43 and is `07`'s;
Q46, a bot's entity id on its top face, amends Q42 and Q43 and is `07`'s;
Q47, lights on printers and depots, amends Q45 and is `07`'s; Q48, a
printer's trim in its team's color, amends Q47 and is `07`'s. Q49, paint
and overlays only on ground the team has seen, is
[03-world](03-world.md)'s, and its preview of a paint plan over paint
amends Q37; Q50, every plan previewed as a smaller version of its effect,
amends Q37 and Q49 and is `07`'s; Q51, a smaller overlay marker and no
cursor ghost over the same plan, amends Q50 and Q37 and is `07`'s; Q52,
free docstrings, is [01-language](01-language.md)'s syntax part; Q53, the
starter library, is `07`'s; Q54, a paint plan's square drawn opaque, amends
Q50 and is `07`'s. Q55, debugging on a fork with what-if, was opened and
answered in one commit, amends Q15, Q32, Q36, Q39 and Q41, and is `07`'s
and `06`'s. Q56, type annotations that do nothing at run time, amends Q13
and is [01-language](01-language.md)'s syntax part; Q57, a name's signature
and docstring on hover, is `07`'s; Q58, going from a hover to the
definition, amends Q57 and is `07`'s. Q59, power from printers and solar
panels wired to a printer, with the newest bots stopping when a team is
short, amended Q10, Q27, Q37, Q42, Q49, Q53 and Q56, and was withdrawn
outright by Q62 before any of it was built, so it has no Decided entry
anywhere; Q60, a replay
tied to the version that wrote it, was opened and answered in one commit
from I3, amends Q36 and Q55, and is `07`'s; Q61, the editor's red
underlines for types that cannot fit, is `07`'s. Q62 — batteries on bots,
the `low_power` interrupt, buildings that generate and drain, grids served
oldest first, chargers, pylons and underground wire — withdraws Q59,
amends Q8, Q10, Q16, Q23, Q27, Q37, Q41, Q42, Q49, Q53 and Q56, and is
[08-power](08-power.md)'s, with entries in `01`'s execution part, `02`,
`03`, `05` and `07`. Q63 — a `low_power` that sets main flow aside and
returns to it, and a bot out of power until its solar has brought it back
to a wake level — amends Q62 and Q8, and is `08`'s and `01`'s execution
part's. Q65 — a height on every tile that bots dig, fill and mine, and
water as small parts that flow over it a tile a tick — was opened
and answered in one commit, amends Q7, Q10, Q21, Q22, Q27, Q28, Q37, Q49
and Q62, and is `03`'s and [09-water](09-water.md)'s, with entries in
`02`, `06`, `07` and `08`. The rulings of Q7, Q9, Q21,
Q22, Q23, Q24, Q25, Q30, Q42, Q43, Q62 and Q65 are
[02-machines](02-machines.md)'s Decided entries, `docs/03` cites Q21 and
Q22 from there, the rulings of Q6, Q10, Q12, Q15, Q26 through Q28, Q32 and
Q55 are [06-architecture](06-architecture.md)'s, Q4 and Q29 are
[04-opposition](04-opposition.md)'s, Q42 is also
[05-progression](05-progression.md)'s, and Q43 is also `07`'s.
The framing rulings are owned by [00-overview.md](00-overview.md)'s Decided
section and the language's by [01-language](01-language.md)'s parts; none
is repeated here. Each worksheet is a file of its own in
[history/questions-answered/](history/questions-answered/README.md), and
the spike behind Q5 is
[spikes/lang-determinism](../spikes/lang-determinism/README.md).

This block is **rewritten in place**, never stacked or archived: `git log -p
docs/QUESTIONS.md` already records every status this file has carried, dated and
attached to the commit that changed it.

**Every numbered doc is written** — [01-language](01-language.md),
[02-machines](02-machines.md), [03-world](03-world.md),
[04-opposition](04-opposition.md), [05-progression](05-progression.md),
[06-architecture](06-architecture.md), [07-interface](07-interface.md),
[08-power](08-power.md), which Q62 added, and [09-water](09-water.md),
which Q65 added,
each with the rulings it waited on as its Decided entries or citing them
from `02`. `02` opened Q24 and `04` opened Q29 in the writing, and both are
answered; `05` was written from Q30, opened and answered for it, and
rewritten from Q42; `07` from Q33 through Q39, the last three opened in its
writing and answered after. The overview's table says the same.

## Open

**Q64 — Should the state hash cover a team's memory by a digest kept as it changes, not a walk of the whole table?**

`06` has the state hash write every team's memory table, tile by tile,
on every tick, so that part costs teams times the tiles they know
whatever changed that tick: at a hundred teams on a 208 by 208 map it is
most of the hash (I12), and what T42 leaves of it is the 876,004 known
tiles written in full. The question is whether a team's memory may enter
the hash as a digest the sim keeps beside the table and brings up to
date wherever the vision pass changes a tile, so that the hash costs
what changed. With it come three more. What the digest is, so that it
is a function of the table and not of the order the writes came in: a
wrapping sum, or an exclusive-or, of each tile's own hash keyed by its
index. What it gives up, since FNV-1a over the whole table is one chain
and a combination of per-tile hashes is a weaker check of the same
bytes. And whether the tiles and the plans, which are walked whole too,
go the same way. Any of it moves every replay hash. *Leaning:* only if a
hundred teams is still too slow once T42 is measured; then the memory
alone, as a wrapping sum of per-tile hashes keyed by index, with a build
with debug assertions recomputing it from the table and comparing, as
the sight cache does.
