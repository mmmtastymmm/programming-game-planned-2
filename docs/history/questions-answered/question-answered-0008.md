*Closed record — see [../README.md](../README.md). Not spec.*

# Q8 — What happens when a program faults?

## Ruling

*Ruling (2026-09-06):* **an uncaught exception is an interrupt.** A unit runs in
one of two modes: **main flow**, which executes the player's program, and
**handler mode**, entered when an interrupt is delivered. Every handler is a
locked system **prologue**, then the player's code, then a locked system
**epilogue** — and the epilogue is unconditional: it runs whether the player's
code returned, faulted, ran out of budget, or was preempted. Interrupt kinds
form a closed, totally ordered set, and a higher-priority interrupt always takes
effect.

Two kinds exist today, and they are the only two:

| Kind | Raised by | Player hook | The epilogue guarantees |
|---|---|---|---|
| `death` (highest) | the world, when the unit is destroyed | `on_death()` | the unit leaves the world — `explode()` |
| `fault` | the unit's own code: an exception nothing caught | `on_fault(e)` | main flow restarts from the top, all variables cleared |

`try`/`except`/`raise` are in the language, Python-shaped, as Q13 said they
would be if an error model existed. An exception caught by an `except` is
ordinary control flow and dispatches nothing; only one that escapes the program
becomes a `fault`.

### The rules

Each of these is hash-affecting, so each is spec rather than an implementation
detail:

1. **A fault aborts main flow.** The faulting operation has no value to continue
   with, so main flow does not resume. The `fault` handler runs to completion,
   and its epilogue restarts main flow from the top with all variables cleared.
   This has the same shape as a redeploy under Q11's leaning, and is stated here
   independently so that it holds whatever Q11 decides.
2. **No handler, no default.** A unit with no `on_fault` **halts**: it stops
   executing, stays in the world, and is visibly faulted until the next redeploy
   restarts it. A system-supplied fallback would be a hidden second program the
   player has to learn — the cost that rejected the fallback option — so there
   is none.
3. **A fault inside `on_fault` escalates to `death`.** The fault handler's user
   code is abandoned and the `death` interrupt is delivered. There is no
   recursion into `on_fault` and no second handler for the second fault.
4. **A fault inside `on_death` is absorbed.** The handler's user code is
   abandoned and the epilogue runs. Death is terminal and its epilogue is
   unconditional, so nothing can keep a dead unit in the world.
5. **Higher priority preempts, and the preempted handler is abandoned.** When a
   higher-priority interrupt is delivered during a handler, that handler's user
   code stops where it is and never resumes; its epilogue runs, then the new
   handler's prologue runs. There is no handler stack. The one case that exists
   today — a unit destroyed while inside `on_fault` — runs `on_fault`'s epilogue
   (the main-flow reset, which is moot) and then `on_death`.
6. **Delivery points.** A `fault` is delivered at the operation that raised it.
   A world-raised interrupt is delivered at the unit's next operation boundary —
   within the same tick if the unit still has budget, otherwise at the start of
   its next slice. An interrupt of equal or lower priority than the running
   handler waits.
7. **One pending entry per kind.** A second `death` or `fault` arriving while
   one of the same kind is pending or running coalesces into it. The pending set
   is therefore bounded by the number of kinds; `docs/01` states the bound
   anyway, alongside every other limit (T8).
8. **Handler code is metered like main flow** and may span ticks. A dying unit
   is in the world until its epilogue runs. Prologue and epilogue are system code
   and cost the player nothing.
9. **Handlers are declared by name.** `on_fault` and `on_death` are a closed set
   of top-level function names, the same pattern as the closed dunder set for
   `class`. Q13 excluded decorators and nested `def`, so this is the only
   declaration shape that fits without a language change.
10. **`e` is what an `except` clause would see**, plus the source location of the
    faulting operation, which derives from the program bytes and is therefore
    deterministic. `docs/01` pins the exact shape.

### What each doc owns

The mechanism — the two modes, prologue/user/epilogue, the priority order,
preemption, delivery points, coalescing, the halt state — is `docs/01`'s. What a
given prologue and epilogue actually do, what a dying unit may still act on, and
what `explode()` leaves behind are game content and belong to `docs/02`.

### Diagnosis

The question's framing said a fault the player cannot see is one they cannot
fix. The in-game half is the handler: `on_fault` receives the exception and its
location. The out-of-game half — what a halted unit looks like, where the fault
record is shown — is the renderer's and waits on Q15. The sim's obligation is
that the fault record exists *in state*, so any renderer can show it and a
replay reproduces it.

### Consequences

- **No interrupt resumes anything.** A fault restarts main flow; death ends the
  unit. Nothing carries live state across an interrupt, which leaves Q13's
  boundary exactly where it is. That property is a consequence of scoping
  interrupts to these two kinds, and it is what widening the set would have to
  give up.
- **Events are deliberately not here.** Pushed world events — damage taken,
  enemy sighted — fit the same mechanism, but a unit would expect to *resume*
  main flow after handling one, and resumption is the live-state question this
  ruling chose not to carry. That is Q16.
- **Q14's overflow-faults option now has a defined landing.** An overflow that
  faults is a `fault` like any other, so the boundary that option worried about
  is rule 6 above.
- **Q11 gains a constraint.** A fault already restarts main flow with variables
  cleared; a redeploy that did anything else would make fault recovery and
  redeploy diverge, and a player would have to learn two recovery models.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section; the Q13
  bullet no longer defers `try`/`except`; the reserved-docs table and the
  paragraph after it. [CLAUDE.md](../../../CLAUDE.md) — crate layout.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block, and Q11's entry gained
  the constraint above.
- **Question:** [Q16](../../QUESTIONS.md) — event interrupts, and the
  resumption semantics they would bring.
- **Task:** [T8](../../TASKS.md) — waits on Q11 and Q14 only now, and the
  interrupt mechanism joins what `docs/01` must pin.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Hard fault — the unit stops dead | Brutal, legible, teaches fast. Fifty stopped units is a dramatic and readable signal to patch. **Survives as the no-handler case.** |
| Fault, then fall back to a default behavior | Forgiving, keeps a match alive. The fallback becomes a hidden second program every player must learn, and masks the signal that something is wrong. **Rejected, and the reason is why rule 2 supplies no default.** |
| **Faults are values — the program handles them** *(chosen, as an interrupt)* | Most expressive and most in the spirit of a programming game. Requires an error model in the language from day one — Q13 deferred `try`/`except` here rather than ruling on it, so this option is what admits it. |
| Static rejection — programs that can fault do not compile | Strongest guarantee a player can rely on. Demands real analysis in the toolchain, and a slow compile is punishing when editing under fire (Q3). Q13 made this materially harder: with user-defined classes and duck-typed attribute access, `x.foo()` needs type inference to check statically even though reflection is excluded. **Not rejected — a later widening under a new number, once `docs/01` exists to analyse against.** |

### How the answer took its shape

The chosen row is not one of the four as written. It is the third with the
*dispatch* specified, and the specification is what does the work: "faults are
values" says a program *may* handle a fault, and says nothing about the fifty
units whose program did not. Two ideas closed that gap.

**Interrupts, not return values.** A unit has two modes. In the first it reads
and runs its code; in the second it is handling an interruption. An exception
nothing caught is one such interruption, delivered through a priority queue
whose top entry is death. Modelling a fault this way rather than as a value the
faulting expression returns means main flow never has to be written defensively
— the fault goes somewhere named, and the player writes the somewhere.

**The locked prologue and epilogue.** The guarantee the game needs — a dead unit
leaves the world — cannot depend on player code, and the epilogue is how it does
not: it runs after the player's code no matter how that code ended. This is what
lets `on_death` exist at all without a second question about what a dying unit
may refuse to do; the answer is that it may refuse nothing, because `explode()`
is in the epilogue.

**Where the hard-fault row went.** The no-handler case — halt, visibly — is the
hard-fault option, preserved as what the player gets by writing nothing. For the
nested cases the escalation rules (a fault inside `on_fault` is death; a fault
inside `on_death` is absorbed) were chosen over halting because they are
terminal: a halt inside a handler would need a rule for what the epilogue does
with a halted unit, and death already has one.

**What "higher priority takes effect" cost.** Preemption means a handler is not
run-to-completion. The alternative — a handler stack, where the preempted
handler resumes afterwards — was rejected for the same reason events were:
resumption is live state carried across an interrupt. Abandoning the preempted
handler is the rule that has no state to carry.

**Why events were left out.** Discussed and parked. Events would resume main
flow, which is exactly the live-state case above; keeping the set at two kinds,
neither of which resumes, is what keeps this ruling from touching Q13's boundary
or pre-empting Q11. The parked design — priorities, an event list, and what
resumes afterwards — is Q16, opened by this ruling so it is not lost.
