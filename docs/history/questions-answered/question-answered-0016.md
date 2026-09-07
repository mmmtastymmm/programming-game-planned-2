*Closed record — see [../README.md](../README.md). Not spec.*

# Q16 — Event interrupts: do world events dispatch to handlers, and what resumes afterwards?

## Ruling

*Ruling (2026-09-07):* **no events. The world reaches a program only through
the queries it polls, and the interrupt kinds stay four.** Q7 already built
the thing an event system would have delivered: a shared, live list of
everything the colony sees now and heard last tick, read whenever a program
asks. A bot decides from that list on its own schedule. Nothing pushes into a
handler except the four kinds Q8, Q11 and Q17 defined — `death`, `dying`,
`fault`, `redeploy` — and none of them resumes anything, which Q13's boundary
depends on and this ruling keeps.

### The rules

1. **Sensing is polling.** Vision and sound are queries (Q7); whatever other
   queries `docs/02` gives a unit are queries too. A program that wants to
   react to the world reads the world, in a loop it writes.
2. **The interrupt kinds are closed at four.** Adding a kind — an event, a
   timer, a message — is a new question number, and the question it would
   have to answer is the one this ruling declined: what main flow does
   afterwards.
3. **The list is live.** It holds what the colony senses *now*, and nothing
   it sensed before. What a program wants to remember, it keeps in its own
   variables, which a restart clears. Whether the colony itself remembers is
   Q21.

### Consequences

- **The execution design is complete.** Every way control moves in a unit —
  metering, the four interrupts, escalation, restart, swap — is ruled, and
  none of them carries live state across an interrupt.
- **Reaction latency is the program's loop.** A bot that polls every tick
  reacts within a tick; one that polls every hundred operations reacts when
  it gets there. That is a cost-model trade the player makes with the tick
  budget, which is what Q5 wanted the cost model to be for.
- **Q21 is opened**, because "the information they have" is two different
  things — what the colony senses now and what it has sensed — and this
  ruling answered only the first.

## Outcome

- **Docs:** [01-language/execution.md](../../01-language/execution.md) — the
  Decided section, beside the interrupt rulings this one completes.
  [00-overview.md](../../00-overview.md) — the reserved-docs table.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Question:** [Q21](../../QUESTIONS.md) — fog of war and colony memory.
- **Task:** [T9](../../TASKS.md) — Q16 leaves its list and Q21 joins it.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **No events — programs poll the world** *(chosen)* | Q8's property survives untouched: nothing ever resumes. Every reaction is a poll in main flow, paid for in budget every tick whether anything happened or not. |
| Events resume main flow where it was interrupted | The natural reading of an interrupt. Main flow becomes a suspended frame across a handler, so a handler that touches the same variables races with it — a rule set of its own, and the one Q13's boundary assumed away. |
| Events restart main flow from the top, like a fault | Keeps Q8's no-resumption property: an event handler is a fault handler with a different epilogue — and after Q18, no guaranteed epilogue at all. A unit mid-task loses the task every time anything happens, which may make events unusable for anything frequent. |
| Events are queued values that main flow drains itself | No preemption at all: the program reads a queue when it chooses. Deterministic and simple, and not an interrupt — latency is whatever the program's loop is, and an unread queue needs a bound and a drop rule. |

### How the answer took its shape

The question was opened by Q8 as the parked half of the interrupt design: the
mechanism could carry events, but events would resume main flow, and
resumption was the live-state case every later ruling steered around. Q7
then answered the need without the mechanism. Once sensing was a shared,
live list that any unit reads on demand, the only thing an event would have
added was *timing* — being told rather than looking — and the fourth option,
a queue the program drains, is what that timing costs once you refuse to
resume: it is polling with a different name. Between polling a list the
world keeps current and polling a queue the sim has to bound and drop from,
the list won. The cost of polling is a real one, paid in tick budget, and it
is the kind of cost the game exists to make players reason about.
