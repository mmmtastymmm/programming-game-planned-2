*Closed record — see [../README.md](../README.md). Not spec.*

# Q63 — Should `low_power` return to main flow where it left off, and what does a bot with no charge do (amending Q62 and Q8)?

## Ruling

*Ruling (2026-11-16):* **`low_power` sets main flow aside, runs its hook,
and puts main flow back where it was; and a bot that cannot pay for a
call is out of power until its solar has brought it back to a level, when
`on_low_power` runs first.** Made under a new number because
[Q62](question-answered-0062.md) and [Q8](question-answered-0008.md) are
closed records. It amends Q62's rule 5, whose `low_power` restarted main
flow with every variable cleared, and its rule 3, whose flat bot waited
only until its charge covered one call; and it amends Q8's "nothing
resumes across an interrupt", which stays true of `fault`, `redeploy`,
`dying` and `death` and is no longer true of `low_power`.

### The rules

All are the language's and the sim's, and hash-affecting.

1. **`low_power` resumes.** Its delivery abandons nothing: main flow is
   set aside as it stands, `on_low_power` runs, and its epilogue puts main
   flow back, which continues from where it was with every variable
   intact. A missing hook is an empty one, so a bot with none is not
   disturbed at all.
2. **It waits for the action in progress.** `low_power` is not delivered
   inside main flow's wait on an action; it stays pending until the
   action completes. A `redeploy` or a higher kind pending beside it is
   delivered there as always. Its prologue does nothing to the body.
3. **The hook may be the whole detour.** `on_low_power` may call anything,
   actions included, across as many ticks as its budget allows; its
   budget is the largest of the hooks', and a wait still debits it. It
   shares main flow's globals, and cannot see main flow's locals.
4. **What ends it early.** A higher kind preempts the hook, and the main
   flow set aside goes with it. An exception escaping the hook, or its
   budget running out, escalates to `fault`, which restarts main flow. A
   second `low_power` raised while the hook runs merges into it. A
   `redeploy` waits for the hook to return.
5. **Out of power.** A bot whose main flow or whose `on_low_power`
   reaches a call its charge does not cover is out of power: it is
   `flat`, and it stays flat until its solar has brought its charge up
   to a wake level set in data, which starts equal to the low-power
   level. `on_fault` and `on_dying` are never held, so a dying bot still
   dies.
6. **Waking runs the hook.** When a flat bot's charge reaches the wake
   level, it is no longer flat and `low_power` is raised for it, armed
   or not. If it was main flow that stopped, the hook runs first and
   main flow then makes the call it stopped at. If it was the hook that
   stopped, the hook starts over from its first line, with main flow
   still set aside.
7. **Data.** The sim refuses data in which any charge cost is above the
   wake level's share of the bot's battery, so a bot that has woken can
   pay for any one call.

### Consequences

- **A bot keeps its place.** Running low costs a program nothing it had
  worked out, and no action it had paid for.
- **The hook and main flow share globals between two operations.** The
  hook runs at one boundary of main flow, so main flow may find a global
  changed between one statement and the next. That is deterministic, and
  it is the player's to think about.
- **A redeploy can wait a whole trip.** A patch lands after the bot is
  back from recharging, bounded by the hook's budget.
- **A drained bot is down for longer**, and then has a fifth of its
  battery to reach a charger with, where Q62's crawled one call at a
  time.
- **A long hook cannot act for nothing.** Q62 held no hook for power,
  which a hook the length of a recharge trip would have turned into free
  movement at zero charge.

## Outcome

- **Docs:** [08-power.md](../../08-power.md) — a second Decided entry,
  the data checks, Charge costs, Solar, a new Out of power section and
  Low power. [execution](../../01-language/execution.md) — the Q8, Q16
  and Q62 entries, the lifecycle, Metering, the kinds, the hooks,
  delivery and escalation; [01-language.md](../../01-language.md) —
  doorway invariant 3. [02-machines.md](../../02-machines.md) — the Q62
  entry, the `flat` attribute, the charge paragraph and the interrupts'
  effect on the body. [06-architecture.md](../../06-architecture.md) —
  the tick's step 2, the state hash and a fork's stops.
  [07-interface.md](../../07-interface.md) — the starter library's
  recharge step. [00-overview.md](../../00-overview.md) — the Decided
  listing. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T39](../../TASKS.md), rewritten — `low_power` that resumes,
  and a program held for power, in the language crate.
- **Task:** [T31](../../TASKS.md), amended — out of power, the wake
  level and the wake's `low_power` in the sim.
- **Problem:** [P8](../../PROBLEMS.md), amended — the gap is Q62's as
  Q63 has it.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| which kinds resume | All of them | A fault has no value to continue with, a redeploy's program has changed under it, and `dying` and `death` end the machine. |
| which kinds resume | **`low_power` alone** *(chosen)* | One kind with its own epilogue; Q8's reasons still hold for the other four. |
| which kinds resume | None, as Q62 ruled | Every bot loses its place at a fifth of its battery, and an action it paid for. |
| what the hook is for | Set a flag and return | Main flow must then poll the flag, which is polling the battery with extra steps. |
| what the hook is for | **The whole detour** *(chosen)* | A recharge trip in one place; a long handler, which real systems avoid in an interrupt and allow in a higher-priority task. |
| the action in progress | Cancelled, as `fault` cancels it | Main flow's call would need a result for an action that never finished. |
| the action in progress | **Finishes first** *(chosen)* | The hook starts with a free body; a long `wait` delays it. |
| a bot with no hook | Restarts anyway | A restart nobody asked for. |
| a bot with no hook | **Nothing happens** *(chosen)* | Q18's missing hook is an empty one, and an empty hook that resumes does nothing. |
| a bot at zero | Waits for one call's charge, as Q62 ruled | It crawls: one call, flat, one call. |
| a bot at zero | **Stopped until solar reaches a wake level** *(chosen)* | Down for longer; wakes with enough to reach a charger. |
| on waking | Main flow just goes on | A program that ran dry once runs dry again. |
| on waking | **`on_low_power` runs at once** *(chosen)* | The hook gets its try with a fifth of the battery in hand. |
| a hook that ran dry | Goes on where it stopped | It continues a trip planned for a place it may no longer be sensible to finish. |
| a hook that ran dry | **Starts over** *(chosen)* | One rule for waking; the hook plans afresh. |
| which hooks power can hold | None, as Q62 ruled | A hook the length of a trip acts for free at zero charge. |
| which hooks power can hold | **`on_low_power` only** *(chosen)* | `on_fault` and `on_dying` must finish; the detour need not. |

### How the answer took its shape

T39 had just been built to Q62 — a `low_power` that cancelled the action
and restarted main flow — when the user said the interrupts were another
problem: they should save the state of the bot, run the handler, and
return to where the program was. Asked what it thought, the drafting
answered that this was right for `low_power` and for nothing else, since
the other four kinds have nothing to return to, and that the interpreter
already keeps a program's state as data and resumes it every tick. The
user then asked the kinds' order of severity and proposed two levels: a
low-power script that can do anything, and an out-of-power state that
forces the solar to charge back up to about a fifth. Before committing,
the user asked how real systems handle it; the answer — interrupts that
save context and return, a low-voltage warning above a brown-out hold
with a gap between them, a watchdog that restarts, and flight software
that samples its sensors on a schedule instead of being interrupted by
them — matched the proposal at every point, and the game's four older
kinds besides. The user added that the hook should run at once after the
bot has charged back up. That a hook which itself ran dry starts over,
that the action in progress finishes first, that a bot with no hook is
left alone, and that the wake level is a data value of its own were the
drafting's calls, put to the user and accepted.
