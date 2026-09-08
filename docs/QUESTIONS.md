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

**Status 2026-09-08.** Q1 through Q5 are answered, and Q7, Q8, Q9, Q11, Q13,
Q14, Q16 through Q24 with them — Q17 through Q20, Q23 and Q24 were each
opened and answered in one commit, as the amendments to Q8, Q9, Q14, Q18, Q22
and Q23 that the history rules require to be new numbers. The rulings of Q7,
Q9, Q21, Q22, Q23 and Q24 are [02-machines](02-machines.md)'s Decided
entries, and `docs/03` will cite Q22 from there. Every other number this
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

**`docs/01` and `docs/02` are written** — [01-language](01-language.md) and
[02-machines](02-machines.md), each with the rulings it waited on as its Decided
entries. `02` opened Q24 in the writing. The overview's table says the same.

## Open

**Q6 — What is the tick, and how many ticks per second?**

Lockstep pins this early and every tuning constant in the corpus inherits it.

| Option | What it costs |
|---|---|
| Slow tick (5–10/s), one action per tick | Cheap to simulate, easy to reason about, and a program's cost is legible. Movement looks stepped unless the renderer interpolates. |
| Medium tick (20–30/s) | Conventional and smooth. More sim work per second — but not more than the sim can afford; see **Unblocked** below. |
| Fast tick (60/s) | Renderer and sim agree; no interpolation needed. The most sim work per second, and the tightest window for Q12's scheduling. |
| Decouple: slow sim tick, interpolating renderer | Best of both, at the cost of a rendering layer that must never feed anything back into the sim. |

**Unblocked.** Interpretation cost does not bound the tick rate — not at any
option in the table above, and not at any fleet size contemplated. The
measurement and its arithmetic are owned by
[spikes/lang-determinism](../spikes/lang-determinism/README.md), finding 5, and
are cited here rather than restated: a second copy of a measured number drifts
away from the conclusion drawn from it, and the number is the whole reason this
question is unblocked. What is left is a game-feel and netcode decision, not a
performance one. A slower tick widens the window Q12 needs for scheduling
updates, which is the remaining reason not to guess.

**Q10 — What else, besides program updates, may enter the sim mid-match?**

Q3 settled that program updates do. This is the question of whether *anything
else* does, and it decides how many categories of command the netcode carries.

| Option | What it costs |
|---|---|
| Nothing else — program updates are the whole input surface | Cleanest. Leaves no way to resign and no way to end a stalemate early. |
| Plus match-control only (resign, agreed draw) | Keeps every sim-affecting input a program update while remaining playable. A second command category with different rules is a permanent small complication. |
| Plus spectator-visible annotations | Nice for streaming and teaching. Anything visible risks becoming load-bearing, and then it is machine-level live input by another name — which Q3 forbids. |

**Q12 — How is a program update scheduled in lockstep?**

The player presses deploy at some wall-clock moment; every peer must apply the
update on the *same tick*. The standard answer is to agree it for a future tick,
far enough ahead that every peer holds it in time. Q11 fixed what *applying*
means — the deployment's program slot changes on the agreed tick, and each machine takes
the `redeploy` interrupt at its next operation boundary — so what this question
owns is the tick.

| Option | What it costs |
|---|---|
| Fixed turn delay (apply at tick `now + N`) | The classic RTS answer: simple, predictable, and the delay is a tunable felt directly as input lag. Picking `N` trades responsiveness against tolerance for a slow peer. |
| Delay negotiated from measured latency | Feels better on good connections and adapts to bad ones. The negotiation itself becomes shared state that must be deterministic. |
| Lockstep barrier — the tick does not advance until every peer has acked | No input lag and no wrong guesses, but one slow peer stalls everyone, which is the failure mode lockstep games are most hated for. |

Also to settle here: what happens when a peer **misses** its window — drop the
update, stall, or desync-and-resync. And whether updates are rate-limited, which
is where PvP fairness re-enters (Q4 defers PvP, so this may be deferred with it,
but the *hook* has to exist in the command format from the start).

**Q15 — What renders the game, and on what stack?**

`sim` is renderer-free plain Rust and the arrow from renderer back to sim does
not exist ([00-overview.md](00-overview.md)). What is undecided is **whether a
renderer crate exists yet and what it is built on** — determinism rule 1 makes
the second half a sim question too, because an ECS engine brings a second world
model alongside the authoritative one, which may reach the sim only as ordered
`Command`s.

| Option | What it costs |
|---|---|
| Bevy — full engine, ECS, its own scheduler | Most given for free: assets, input, windowing, UI. Brings a second world model into the process, which determinism rule 1 permits — but only for as long as nothing ECS-side reaches sim state except through ordered `Command`s, which is the boundary CLAUDE.md asks reviewers to flag every time, and which then has to be defended in review forever. |
| macroquad / miniquad — immediate-mode 2D | Small, no ECS and no scheduler of its own, so drawing from sim state is a plain read. Little given for free above drawing: UI, input mapping and asset handling are all ours. |
| `wgpu` directly | No opinions imposed and no engine to fight; the crate boundary is trivially safe. The most work by a wide margin, and none of it is game design. |
| Headless for now — no renderer crate until the sim earns one | Costs nothing today and keeps the corpus honest about what is built. A sim nobody watches hides the problems only visible in motion, and the renderer's needs then arrive late, as sim changes. |

Also to settle here: whether the renderer runs in the sim's process at all, and
what it is allowed to read. A renderer that samples state mid-tick sees a torn
world; one that reads only a completed tick's snapshot does not, and that is a
shape the sim has to offer deliberately.

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
