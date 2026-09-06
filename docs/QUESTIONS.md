# Open Design Questions

**This is the only file that holds open questions, and it holds nothing else.**
Other docs may cite a number inline ("open — Q7") but never restate a question's
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

**Status 2026-08-23.** Q1 through Q5 are answered, and Q13 with them. Every
other number this file has issued is still undecided, and each one is below
under **Open**. Those rulings are owned by
[00-overview.md](00-overview.md)'s Decided section and are not repeated here;
each worksheet is a file of its own in
[history/questions-answered/](history/questions-answered/README.md), and the
spike behind Q5 is
[spikes/lang-determinism](../spikes/lang-determinism/README.md).

This block is **rewritten in place**, never stacked or archived: `git log -p
docs/QUESTIONS.md` already records every status this file has carried, dated and
attached to the commit that changed it.

**`docs/01` waits on Q8, Q11 and Q14** — Q14 last of the three in dependency
order, which is not the same as being the only one. Q8 and Q11 each carry a live
dependency on Q13, and each states it in its own entry below rather than here:
what this block owns is the *shape* of the two, which is that they point opposite
ways. Q11's answer can reopen Q13's boundary; Q13's answer has already narrowed
Q8's. Q8 also owns whether `try`/`except` is in the grammar at all, which
`docs/01` cannot specify around.

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

**Q7 — What does a unit sense?**

Determinism rule 6 makes every query a *sorted* query, so the sensing model and
its ordering are one decision, not two.

| Option | What it costs |
|---|---|
| Omniscient within a radius | Simplest to specify and to sort. Removes scouting and information asymmetry as design material. |
| Line-of-sight, per unit | Makes terrain matter and exploration real. Visibility becomes per-unit hashed state, a large addition to the state hash at fleet scale. |
| Shared fleet vision | One visibility set per player rather than per unit. Cheaper to hash, and makes the fleet feel like one organism rather than many agents. |
| Explicit sensors as equipment | Sensing becomes a build choice with costs and trade-offs. Most design surface, most tuning, most to get wrong. |

**Q8 — What happens when a program faults?**

Q2's failure mode is "one bad program, fifty dead units". Q3 softens it — the
player can patch mid-match — but that makes the *diagnosis* path matter as much
as the failure semantics: a fault the player cannot see is a fault they cannot fix.

| Option | What it costs |
|---|---|
| Hard fault — the unit stops dead | Brutal, legible, teaches fast. Fifty stopped units is a dramatic and readable signal to patch. |
| Fault, then fall back to a default behavior | Forgiving, keeps a match alive. The fallback becomes a hidden second program every player must learn, and masks the signal that something is wrong. |
| Faults are values — the program handles them | Most expressive and most in the spirit of a programming game. Requires an error model in the language from day one — Q13 deferred `try`/`except` here rather than ruling on it, so this option is what would admit it. |
| Static rejection — programs that can fault do not compile | Strongest guarantee a player can rely on. Demands real analysis in the toolchain, and a slow compile is punishing when editing under fire (Q3). **Q13 made this materially harder**: with user-defined classes and duck-typed attribute access, `x.foo()` needs type inference to check statically even though reflection is excluded. |

**Q9 — Where do units come from?**

Fixed roster at match start, or produced during it.

| Option | What it costs |
|---|---|
| Fixed roster, chosen before the match | Simplest. A match becomes a pure test of the program set, and the economy stops being gameplay. |
| Produced by a rule the player configures, and can reconfigure mid-match | Consistent with Q3: production is another thing you author and re-author. The configuration surface is a second, smaller language to design unless it is expressed in the unit language Q13 bounded, which `docs/01` would then have to cover. |
| Produced by ordinary programs, like any other unit behavior | One authoring surface, maximum consistency with Q2. Production becomes something a program can get catastrophically wrong — which Q8 then has to survive. |

**Q10 — What else, besides program updates, may enter the sim mid-match?**

Q3 settled that program updates do. This is the question of whether *anything
else* does, and it decides how many categories of command the netcode carries.

| Option | What it costs |
|---|---|
| Nothing else — program updates are the whole input surface | Cleanest. Leaves no way to resign and no way to end a stalemate early. |
| Plus match-control only (resign, agreed draw) | Keeps every sim-affecting input a program update while remaining playable. A second command category with different rules is a permanent small complication. |
| Plus spectator-visible annotations | Nice for streaming and teaching. Anything visible risks becoming load-bearing, and then it is unit-level live input by another name — which Q3 forbids. |

**Q11 — What happens to a unit mid-execution when its program is swapped?**

Q3 permits the swap; this decides what it does to fifty units that are
each somewhere in the middle of the old program. Every option below changes the
state hash, so this cannot be discovered during implementation.

**Q13 depends on the answer.** Its broad subset — `class`, and any later
admission of generators — is sound *because* a swap clears all variables, so
nothing survives it in a half-valid state. Choosing any option here that
**resumes** rather than clears reopens Q13's boundary. The leaning is therefore
to clear, and this note exists so that choosing otherwise is a deliberate act
rather than an oversight.

| Option | What it costs |
|---|---|
| Restart from the top, clearing all variables | Trivially defined, easy to explain, and the option Q13's boundary assumes. A unit halfway home drops everything and starts over, so a late patch can be worse than no patch. |
| Resume at the same instruction offset | Feels continuous, and is meaningless the moment the edit changes the program's shape — offset 12 of the new text is not the old offset 12. |
| Resume at a named re-entry point the program declares | Predictable and authorable, and gives the player real control over patch cost. Requires the language to carry the concept, which is outside Q13's boundary and would be a widening under a new number. |
| Finish the current action, then restart | A compromise that keeps in-flight work. "Current action" must then be a precisely defined boundary in the sim, which is a rule-7-grade specification burden. |

Whatever wins must also say what happens to a unit's **local state** — variables,
accumulated position in a loop — across the swap. Discarding it is simple;
preserving it means the new program must be type-compatible with the old one's
state, which is a language decision, not a sim decision.

**Q12 — How is a program update scheduled in lockstep?**

The player presses deploy at some wall-clock moment; every peer must apply the
update on the *same tick*. The standard answer is to agree it for a future tick,
far enough ahead that every peer holds it in time.

| Option | What it costs |
|---|---|
| Fixed turn delay (apply at tick `now + N`) | The classic RTS answer: simple, predictable, and the delay is a tunable felt directly as input lag. Picking `N` trades responsiveness against tolerance for a slow peer. |
| Delay negotiated from measured latency | Feels better on good connections and adapts to bad ones. The negotiation itself becomes shared state that must be deterministic. |
| Lockstep barrier — the tick does not advance until every peer has acked | No input lag and no wrong guesses, but one slow peer stalls everyone, which is the failure mode lockstep games are most hated for. |

Also to settle here: what happens when a peer **misses** its window — drop the
update, stall, or desync-and-resync. And whether updates are rate-limited, which
is where PvP fairness re-enters (Q4 defers PvP, so this may be deferred with it,
but the *hook* has to exist in the command format from the start).

**Q14 — The number model**

Every option below changes what arithmetic a program can express, so this
cannot be discovered during implementation. The spike disqualified Lua on
exactly this axis: `7 / 2` is `3.5`, and a float in a state-affecting path
violates rule 2. Python's `/` behaves the same way, so Python's own answer is
not available to us.

| Option | What it costs |
|---|---|
| Fixed-width `i64`, overflow faults | One number type, no surprises, and overflow is a legible in-game failure. The fault is hash-affecting, so the fault *boundary* becomes spec (Q8). |
| Fixed-width `i64`, overflow wraps | Never faults, never surprises the sim. Silently wrong answers are worse than loud ones in a language players debug under time pressure. |
| Arbitrary-precision integers | No overflow to specify at all, and deterministic. Unbounded memory and time per operation, which fights the per-tick cost model Q5 chose to own. |
| Fixed-point rationals for fractional values | Makes division expressible without floats. A second numeric type, and every mixed-type operation is a rule someone has to remember. |

Whatever wins must also answer what `/` *does*: reject at parse time (forcing
`//`), silently mean integer division (familiar-looking and quietly un-Pythonic),
or return a fixed-point value. Rejecting is the most honest and the most annoying;
it is also the only option that cannot silently produce a different answer than
the player expected.

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
