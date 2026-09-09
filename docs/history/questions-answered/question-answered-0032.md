*Closed record — see [../README.md](../README.md). Not spec.*

# Q32 — Amend Q6 and Q17: a paused driver still reaches a command's tick, and a hook's wait spends its budget

## Ruling

*Ruling (2026-09-09):* two rules a whole-corpus review found missing, each
of which two implementers would have filled differently. Made under a new
number because [Q6](question-answered-0006.md) and
[Q17](question-answered-0017.md) are closed records.

**A paused driver still runs the ticks needed to reach a command.** Q6 made
speed `0` a pause and promised that the first `SetSpeed` above zero resumes,
but `docs/06`'s driver skipped `sim.step` entirely at speed `0`, and a
`SetSpeed` only takes effect when `sim.step` applies it on its agreed tick —
so nothing could ever unpause. Now: while paused, the driver issues no tick
on the clock, but whenever every peer's set is in hand for a run of ticks
ending in one that carries any command, it runs exactly those ticks, in
order, as fast as it can, and waits again. A `SetSpeed` agreed for tick `T`
therefore lands on `T`, and the sim's clock still advances only when the
log says something.

**A wait inside a hook spends the hook's budget.** Q17 gave every hook a
total budget so a handler holds off a lower-priority interrupt by at most
that budget, and `docs/02` then made an action a wait that costs nothing
while it lasts — so `wait(10**9)` in `on_dying`, with health frozen, was an
immortal machine holding a tile and a cap slot forever. Now: every tick a
hook spends waiting on an action debits `wait_per_tick` (the hook budget's, in `data/language/limits.toml`) from the hook
budget, so a hook that waits is a hook that runs out, and the bound Q17
promised holds again. Main flow's waits stay free.

### The rules

1. **Paused stepping.** At speed `0`, the driver checks the log for the
   lowest tick `T > current` for which some peer's set is non-empty. If every
   peer's set is in hand for every tick up to `T`, it runs those ticks
   without waiting on the clock, publishes each snapshot, and returns to
   waiting. If a set is missing it stalls as Q12 says. Every peer sees the
   same log, so every peer runs the same ticks.
2. **A hook's wait is metered.** While a hook's action is in progress, each
   tick debits `wait_per_tick` (the hook budget's, in `data/language/limits.toml`) cost units from that hook's budget, at
   the boundary the wait occupies; exhaustion escalates as Q17 rules,
   cancelling the action. Main flow's waits debit nothing.
3. **Both numbers are tuning**: `wait_per_tick` (the hook budget's, in `data/language/limits.toml`) in
   `data/language/limits.toml`, and the driver's `delay` and
   `stall_report_ticks`, which `docs/06` had named without a home, in
   `data/net.toml`.

## Outcome

- **Docs:** [06-architecture.md](../../06-architecture.md) — the driver
  loop and its Decided section. [01-language/execution.md](../../01-language/execution.md)
  — the hook budget and its Decided section. [QUESTIONS.md](../../QUESTIONS.md)
  — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| unpausing | The driver applies `SetSpeed` itself, outside `sim.step` | A second path that mutates what the sim applies, and a speed change that is not in the sim's order of events; contradicts `06`'s one-entry-point rule. |
| unpausing | **Run the ticks up to the command's** *(chosen)* | Nothing new: the sim steps as it always does, the clock is just not consulted. |
| the dying wait | Forbid `wait` in `on_dying` | Loses the one way a dying machine can pace its last words. |
| the dying wait | A per-kind cap on a hook's ticks | A second budget beside the operation budget, in a different unit. |
| the dying wait | **A wait debits the hook budget per tick** *(chosen)* | One budget, one unit, and a wait is simply an expensive operation inside a hook. |

### How the answer took its shape

Both holes were found by reading the docs against each other rather than
each on its own — the pause by tracing where speed changes, the wait by
asking what `on_dying` could do with an action that costs nothing. Each fix
is the smallest rule that closes the hole without a new mechanism.
