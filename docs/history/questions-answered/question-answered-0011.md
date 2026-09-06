*Closed record — see [../README.md](../README.md). Not spec.*

# Q11 — What happens to a unit mid-execution when its program is swapped?

## Ruling

*Ruling (2026-09-06):* **a redeploy is an interrupt** — the third kind, below
`fault`, in the mechanism Q8 built. It is delivered like any world-raised
interrupt and has a locked prologue and epilogue like the others, but **no
player hook**: there is no `on_redeploy`, and the epilogue is the swap. The unit
takes its role's current program, every variable is cleared, and main flow
restarts from the top of the new program. Nothing survives a swap, which is the
property Q13's boundary assumed and Q8 already gave faults.

The kinds, in priority order, are now:

| Kind | Raised by | Player hook | The epilogue guarantees |
|---|---|---|---|
| `death` (highest) | the world, when things are bad enough; `dying`'s epilogue (Q17) | *none* | the unit leaves the world |
| `dying` | the world, when the unit is destroyed; a fault inside `on_fault` (Q17) | `on_dying()` | `death` is raised |
| `fault` | the unit's own code: an exception nothing caught | `on_fault(e)` | main flow restarts from the top, all variables cleared |
| `redeploy` (lowest) | the command log: a program update agreed for a tick (Q12) | *none* | the unit takes its role's current program, all variables cleared, main flow restarts from the top |

### The rules

Q8's rules apply unchanged; these are the ones a third kind adds. Each is
hash-affecting.

1. **The role's program slot changes on the agreed tick; each unit swaps at
   its own delivery point.** The command log updates which program a role runs
   on the tick Q12 agrees. From that tick, every unit of the role has a
   `redeploy` pending, delivered at its next operation boundary (Q8, rule 6).
   Until it is delivered the unit runs the old version, and determinism rule 7
   records which — the state hash sees the swap per unit, not per command.
2. **The boundary before the first operation counts.** A unit whose main flow
   has just been restarted — by a fault's epilogue, say — takes a pending
   `redeploy` before executing anything. The old program never runs a further
   operation once the swap is due and nothing higher is running.
3. **No player code runs on a redeploy.** Prologue, then epilogue, then the new
   program. The old program gets no last word: the player is replacing it, and
   the one thing a redeploy must not depend on is the code being replaced.
4. **Lowest priority, so a unit inside a handler finishes that handler first.**
   A `redeploy` arriving during `on_fault` waits; `on_fault` completes, its
   epilogue restarts the old main flow, and rule 2 delivers the swap before that
   flow runs an operation. During `on_dying` it waits and is moot. A second
   `redeploy` while one is pending coalesces (Q8, rule 7) — and since the
   epilogue loads whatever the slot holds *at swap time*, coalescing cannot
   deliver a stale version.
5. **A halted unit is not executing, so its next operation boundary is the
   start of its next slice.** Delivery then proceeds as for any unit. This is
   how Q8's "halts until the next redeploy" is implemented: a halted unit is a
   unit with nothing to abandon.
6. **The pending set is left alone.** Nothing ranks below `redeploy`, so
   anything pending is higher and would already have preempted; there is nothing
   for the epilogue to clear.

### Local state

Discarded, entirely, by the epilogue. The question asked whether a swap should
preserve variables and what type-compatibility the new program would then owe
the old one's state. Clearing dissolves that: there is no state to be compatible
with, and the language carries no notion of migrating it.

### What each doc owns

The mechanism is `docs/01`'s, with the rest of the interrupt model. When the
command tick is, and what happens when a peer is late, is Q12's. What the
prologue and epilogue of a redeploy do to the unit's body — whether it stops,
whether it keeps what it carries — is `docs/02`'s, like every other prologue
and epilogue.

### Consequences

- **Fault, redeploy and halt-recovery are one mechanism.** A player learns one
  recovery model: a handler runs if there is one, then the program starts from
  the top. Q8's constraint is satisfied by construction rather than by
  coordination.
- **Q13's dependency is discharged.** The boundary was sound *if* a swap
  cleared all variables; it does. `class` carries nothing across a swap.
- **A handler holds off a redeploy by at most its hook's budget.** `redeploy`
  is lowest, so a unit inside `on_fault` finishes that first — and since every
  hook has a total operation budget (Q17), "finishes" is bounded. A draft of
  this ruling had no such bound and accepted that a looping `on_fault` could
  block a redeploy until the unit died; Q17 retired that (worksheet).
- **T7's golden fixture must exercise a redeploy.** Per-unit delivery at the
  next operation boundary is the hash-affecting path most likely to differ
  between peers, and it is now specified well enough to test.
- **Q16's caution is sharpened.** Three kinds exist and none resumes anything.
  Events would be the first that did.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, the Q13
  bullet's dependency discharged, the reserved-docs table and the paragraph
  after it. [CLAUDE.md](../../../CLAUDE.md) — crate layout.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block; Q12 gained what
  "apply" now means; Q16's framing.
- **Task:** [T8](../../TASKS.md) — waits on Q14 alone now, and the must-pin
  list names the swap. [T7](../../TASKS.md) — no longer blocked on this
  question; the fixture requirement above. [T9](../../TASKS.md) — the sweep
  and the Q11/Q13 sequencing note.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Restart from the top, clearing all variables** *(chosen, as an interrupt)* | Trivially defined, easy to explain, the option Q13's boundary assumes, and what Q8 already does after a fault. A unit halfway home drops everything and starts over, so a late patch can be worse than no patch. |
| Resume at the same instruction offset | Feels continuous, and is meaningless the moment the edit changes the program's shape — offset 12 of the new text is not the old offset 12. |
| Resume at a named re-entry point the program declares | Predictable and authorable, and gives the player real control over patch cost. Outside Q13's boundary; a widening under a new number if wanted later. |
| Finish the current action, then restart | A compromise that keeps in-flight work. "Current action" must then be a precisely defined boundary in the sim, which is a rule-7-grade specification burden. |

### How the answer took its shape

By the time this was ruled, two answered questions had already chosen for it.
Q13 had widened the language on the promise that a swap clears everything, and
Q8 had made fault recovery a restart from the top. Anything but clearing would
have reopened the first and made the second one of two recovery models. The
open part was not *whether* to clear but *how to say it*.

**As an interrupt, because the mechanism already existed.** Q8 built prologue,
epilogue, a priority order and a delivery rule. A redeploy needs every one of
those: something the world raises, a point at which it takes effect, a guarantee
the player cannot skip, and an answer for what happens if it arrives
mid-handler. Making it a kind rather than a separate path means all of that is
inherited rather than restated.

**Lowest priority, because a fix should not interrupt a fault being handled.**
The alternative — a redeploy preempting `on_fault`, on the argument that the
player is fixing the fault — would abandon handler code the player wrote to
run. Waiting costs nothing in the common case: rule 2 delivers the swap before
the old main flow executes an operation.

**No `on_redeploy`, considered and dropped.** A draft gave the outgoing program
a hook — a last word to stop or drop cargo, which would have absorbed the
"finish the current action" option in authorable form. It was dropped because
it runs the code being replaced: a redeploy that depends on the old program
behaving is a redeploy that a broken old program can spoil, and the fix for a
broken program is the redeploy. With no player code in it, a redeploy is
prologue, epilogue, new program, and nothing can go wrong in between.

**The handler budget: dropped here, restored by Q17.** The same draft bounded
every handler's user code so old code could not hold off its replacement. With
`on_redeploy` gone, the case that motivated it — the redeploy's own handler
looping — no longer existed, and the budget was dropped with it, accepting that
a looping `on_fault` could block a lower-priority redeploy and a looping death
hook could linger. That acceptance lasted one revision: Q17 restored the budget,
scoped to hooks, and split death so the removal itself sits behind no player
code at all. This ruling now depends on Q17 for its bound and says so above.

**What was not decided here.** When the command tick is, how far ahead it is
agreed, and what a late peer does are Q12's, untouched. The ruling depends only
on there *being* an agreed tick.
