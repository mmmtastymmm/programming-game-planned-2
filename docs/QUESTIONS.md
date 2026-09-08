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

**Status 2026-09-08.** Q1 through Q27 are all answered except Q25 — Q17
through Q20, Q23, Q24, Q26 and Q27 were each opened and answered in one commit,
as the amendments that the history rules require to be new numbers. The rulings of
Q7, Q9, Q21, Q22, Q23 and Q24 are [02-machines](02-machines.md)'s Decided
entries, `docs/03` cites Q21 and Q22 from there, and the rulings of Q6, Q10,
Q12 and Q15 sit in the overview's Decided section until `docs/06` exists to
own them. Every other number this
file has issued is still undecided, and each one is below under **Open**. The
framing rulings are owned by [00-overview.md](00-overview.md)'s Decided section
and the language's by [01-language](01-language.md)'s parts; none is repeated
here. Each worksheet is a file of its own in
[history/questions-answered/](history/questions-answered/README.md), and the
spike behind Q5 is
[spikes/lang-determinism](../spikes/lang-determinism/README.md).

This block is **rewritten in place**, never stacked or archived: `git log -p
docs/QUESTIONS.md` already records every status this file has carried, dated and
attached to the commit that changed it.

**`docs/01`, `02` and `03` are written** — [01-language](01-language.md),
[02-machines](02-machines.md) and [03-world](03-world.md), each with the
rulings it waited on as its Decided entries or citing them from `02`. `02`
opened Q24 in the writing. The overview's table says the same.

## Open

**Q25 — Combat: what deals damage beyond a fault, whether bots attack, whether a building defends, and whether anything repairs**

Q24 gave every machine health and made damage the one mechanism that lowers
it, with a single cause — a fault costs one health — and deferred every
other cause here. A game whose colonies cannot touch each other is not the
game Q4 deferred PvP from, and PvE needs an opposition (`docs/04`) that can
hurt. This is the largest tuning surface the game will have, which is why it
gets its own number rather than riding on `docs/02`.

What it has to settle, each hash-affecting: what deals damage — a bot
action, a building that fires by rule, terrain, the opposition — with what
range, amount and cooldown; whether damage is in `num` so armour and
modifiers can multiply later; whether buildings and sites take damage the
same way; whether anything repairs, and what it costs; and what a dying
machine's last words may still do beyond what `docs/02` allows now.

| Option | What it costs |
|---|---|
| Bots attack: an `attack` action with a range, a damage, and a cooldown | The smallest combat that makes programs matter — targeting is code. Damage per hit and range per model, and the first tuning that decides matches. |
| Buildings defend: a turret model that fires by rule, no program needed | Territory becomes defensible without writing a fighter. Passive, so defence is placement; a building row with a range and a damage. |
| Both, with repair as a bot action that spends ore | The full surface: every number above, twice, plus a repair rate and cost, and a balance problem Q4 chose to defer. |
| Neither yet — faults are the only damage until PvE needs more | Nothing to tune, and `dying` fires only from bugs. Defers the failure mode the fleet fantasy wants until `docs/04` demands it. |

Whatever wins adds its causes to the interrupt table in `docs/02`, its
actions or models to that doc's tables, and its numbers to data.
