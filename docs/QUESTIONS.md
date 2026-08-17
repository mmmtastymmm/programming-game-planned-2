# Open Design Questions

**This is the only file that holds open questions, and it holds nothing else.**
Other docs may cite a number inline ("open — Q7") but never restate a question's
substance or its leaning; a question restated in two places gets answered in one
of them.

Answering a question empties it out of this file in three directions: the ruling
goes to the current `questions-answered-NNN.md` shard, its worksheet body to the
current `questions-worksheets-NNN.md` shard, and the status block it displaces
to this year's `questions-status-log-YYYY.md` — all in
[history/](history/README.md). Nothing answered stays here. That discipline is
the whole reason this file is readable: the predecessor project let answered
material stack up and reached 166 KB, at which point everyone was reading it in
full to find the handful of things still open.

Numbering is append-only — **never renumber**, and never reuse a number after it
moves to history.

Known-wrong *decided* text is not a question — it goes in
[PROBLEMS.md](PROBLEMS.md). Raw unsorted observations go in
[INBOX.md](INBOX.md).

**Status 2026-08-17 (latest):** nothing is open, because nothing has been
proposed. The prompts below are the questions that block everything else; they
become numbered entries once each has an actual worksheet — a statement of the
options and what each costs — rather than just a topic.

## Where the design has to start

The predecessor project answered these in a particular order and paid for the
places where it guessed early. In rough dependency order:

1. **What does the player program — one unit, a fleet, or a factory?** Almost
   every other decision hangs off this one.
2. **What is the language?** Its execution model (interpreted one operation at a
   time? compiled to a bytecode with a cycle budget?) determines what "fair"
   means and what the tick loop costs.
3. **What is the tick, and how many ticks per second?** Lockstep pins this
   early; changing it later invalidates every tuning number.
4. **What does a unit sense?** Determinism rule 6 means every query is a sorted
   query — the sensing model and the sort order are the same decision.
5. **What is the win condition?** PvE, PvP, or both changes what the sim has to
   be fair about.

## Open

*(none yet — see above)*
