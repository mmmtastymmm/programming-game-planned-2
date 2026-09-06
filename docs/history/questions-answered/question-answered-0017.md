*Closed record — see [../README.md](../README.md). Not spec.*

# Q17 — Amend Q8: death is two kinds, and hooks carry a budget

## Ruling

*Ruling (2026-09-06):* two amendments to the interrupt model
[Q8](question-answered-0008.md) built, made under a new number because Q8's
file is a closed record. Everything in Q8 not named here stands.

**Death splits into `dying` and `death`.** `dying` is where the player's last
words live: it has a hook, `on_dying()`, and its epilogue raises `death`.
`death` has no hook at all — prologue, epilogue, and the unit leaves the world —
so nothing the player writes can delay or handle it. The world raises `dying` in
the ordinary case and `death` directly when things are bad enough, and which
causes are which is `docs/02`'s list.

**Every hook has a total operation budget.** A hook is a handler's player code.
Each invocation of one gets a budget — a tuning constant `docs/01` pins, which
may differ per kind — and when it is exhausted the player's code is abandoned
and the epilogue runs. Combined with the priority order, a handler can hold off
a lower-priority interrupt by at most its budget, which is what makes both
`redeploy` (Q11) and `death` reliable.

The kinds, in priority order, are now:

| Kind | Raised by | Player hook | Budget | The epilogue guarantees |
|---|---|---|---|---|
| `death` (highest) | the world, when things are bad enough; `dying`'s epilogue | *none* | — | the unit leaves the world |
| `dying` | the world, when the unit is destroyed; a fault inside `on_fault` | `on_dying()` | yes | `death` is raised |
| `fault` | the unit's own code: an exception nothing caught | `on_fault(e)` | yes | main flow restarts from the top, all variables cleared |
| `redeploy` (lowest) | the command log: a program update agreed for a tick (Q12) | *none* | — | the unit takes its role's current program, all variables cleared, main flow restarts from the top |

### The rules

Q8's rules apply with the following replacements and additions, each
hash-affecting. Numbers refer to Q8's list.

- **Rule 3 becomes:** a fault inside `on_fault` escalates to `dying`. The
  chain is fault, dying, death — each strictly higher than the last — so it
  terminates, and a software failure still gets last words.
- **Rule 4 becomes:** a fault inside `on_dying` is absorbed. The hook's code is
  abandoned and the epilogue raises `death`. `death` has no player code, so it
  needs no rule of its own.
- **Rule 8 gains:** each invocation of a hook has a total operation budget,
  metered like main flow across ticks. Exhaustion abandons the player's code
  and runs the epilogue. Prologue and epilogue remain free.
- **`death` during `on_dying`** preempts it (Q8, rule 5): `on_dying`'s epilogue
  raises `death`, which coalesces with the one already pending (Q8, rule 7),
  and then `death`'s epilogue runs. A unit that is bad enough to die outright
  while saying its last words simply stops saying them.
- **A dying unit is in the world** from `dying`'s delivery until `death`'s
  epilogue, and may act within what `docs/02` allows a dying unit to do. Its
  time in that state is bounded by `on_dying`'s budget.

### What each doc owns

The mechanism — the two death kinds, the budget rule, the escalation chain —
is `docs/01`'s. Which world causes raise `dying` and which raise `death`, what
a dying unit may still act on, and what `death`'s epilogue leaves behind are
`docs/02`'s. The budget numbers are tuning constants, which live in data files
(CLAUDE.md).

### Consequences

- **Q11's accepted cost is retired.** Q11 accepted, for want of a budget, that
  a looping `on_fault` could hold off a redeploy until the unit died. With hooks
  budgeted, the hold-off is at most `on_fault`'s budget, and Q11's file says so.
- **`death` is now the guarantee Q8 wanted `explode()` to be.** Q8 put the
  removal in an epilogue so player code could not skip it; a budgeted hook in
  front of it could still *delay* it. Splitting the kinds makes the removal
  undelayable as well as unskippable.
- **Q16's framing counts four kinds.** None resumes anything; events would be
  the first that did.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section: the Q8
  bullet rewritten to current state and this ruling's bullet added.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block; Q16's framing.
- **Task:** [T8](../../TASKS.md) — the must-pin list names the four kinds and
  the hook budgets.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Keep Q8 as ruled: one `death` with a hook, no budget | A hook in front of the removal can delay it for as long as its code runs. A looping `on_fault` holds off a redeploy until the unit dies; a looping `on_death` keeps a dead unit in the world. Q11 accepted both for one revision. |
| One `death` with a hook, hooks budgeted | Fixes the hold-off. The removal is still behind player code, bounded but not immediate, and a world cause that should be instantaneous has no way to be. |
| **Two kinds, `dying` with a budgeted hook and `death` with none** *(chosen)* | One more kind to specify, and `docs/02` must sort world causes into two bins. Buys an undelayable removal, last words that are bounded, and a home for "bad enough". |
| No last words at all: `death` with no hook, drop `on_death` | Simplest. Loses the one place a player can react to losing a unit — drop cargo, signal — which the fleet fantasy (Q2) wants. |

### How the answer took its shape

Q11's draft history is the argument. Its first draft gave every hook a total
budget, motivated by a redeploy hook that could loop; its second draft dropped
both the hook and the budget, and accepted that a looping `on_fault` could
block a redeploy. That acceptance was correct as far as it went — the priority
order does guarantee `death` gets through — but it left two things a player
could make arbitrarily bad with a two-line handler, and "the player did it"
is a weak answer in a game whose loop is fixing what the player did.

The split came from asking what the budget was actually protecting. For
`redeploy` it protects the fix; for death it protects the *removal*. The
removal wants a stronger guarantee than a bound — it wants to be immediate —
and the only way to have both last words and an immediate removal is for them
to be different interrupts. Once they are, the budget's job shrinks to bounding
hooks, which is exactly where the first draft had put it.

**Why a fault in `on_fault` goes to `dying` rather than `death`.** Straight to
`death` would be simpler by one rule and would deny a software failure the last
words a combat death gets. The chain terminates either way; `dying` is the kinder
choice at no cost to determinism.
