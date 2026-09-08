*Part of [01-language](../01-language.md).*

# Execution

How a machine runs its program: the lifecycle, the metering that spreads a
program across ticks, the interrupts that cut into it, and the limits that
bound all of it. Every rule here is hash-affecting.

## Decided

- **An uncaught exception is an interrupt (Q8, as amended by Q18).** A machine
  runs in main flow until an interrupt is delivered. A handler is a locked
  system prologue, the player's hook if the kind has one, then a locked system
  epilogue. Interrupt kinds are a closed, totally ordered set, and higher
  priority always takes effect: the preempted handler is abandoned, never
  resumed, so there is no handler stack. The kinds, highest first, are `death`
  and `dying` (Q17), `fault` (this ruling) and `redeploy` (Q11). `fault` is an
  exception nothing caught; its hook `on_fault(e)` runs, and its epilogue
  restarts main flow from the top with all variables cleared. There is no
  system default handler: a machine with no `on_fault` runs the fault prologue and
  epilogue back to back, and Q8's halt state is withdrawn by Q18. A fault
  inside `on_fault` escalates to `dying`. `try`/`except`/`raise` are in,
  Python-shaped. Nothing resumes across an interrupt, which is what keeps Q13's
  boundary intact; pushed world events, which would resume, were Q16, which
  ruled them out. The
  mechanism is this doc's; what each prologue and epilogue does is `docs/02`'s.
- **Death is two kinds, and every hook has a budget (Q17, as amended by
  Q18).** `dying` is where last words live: its hook `on_dying()` may act
  within what `docs/02` allows, and its epilogue raises `death`. `death` has no
  hook — prologue, epilogue, and the machine leaves the world — so nothing the
  player writes can delay or handle it. The world raises `dying` in the
  ordinary case and `death` directly when things are bad enough; which causes
  are which is `docs/02`'s list. Every hook — a handler's player code — has a
  total budget per invocation in cost units, a tuning constant; exhausting it
  escalates exactly as a fault in that hook would. So a handler holds off a
  lower-priority interrupt by at most its budget.
- **A redeploy is an interrupt (Q11)** — the lowest kind, `redeploy`, below
  `fault`, with no player hook, like `death`. The role's program slot changes on
  the tick Q12 agrees; each machine takes the interrupt at its own next operation
  boundary, and the boundary before the first operation counts. The epilogue is
  the swap: the machine takes its role's current program, every variable is
  cleared, and main flow restarts from the top. A machine inside a handler finishes
  that handler first, which Q17's hook budget bounds. No program state
  survives a swap — the deficit, the pending set and the fault record belong
  to the machine, not the program — which discharges the dependency Q13's
  boundary carried.
- **The epilogue rewards success, and a machine is never halted or idle (Q18).**
  The epilogue is the last step of a hook that *returned*. A hook that raises
  an exception nothing catches, or exhausts its budget, gets no epilogue:
  control escalates to the next kind up — `on_fault` to `dying`, `on_dying` to
  `death` — and preemption abandons the whole handler, epilogue included.
  `death`, having no hook, is the one kind whose epilogue nothing can skip.
  Every kind's handler exists whether or not its hook is defined, so a missing
  hook is an empty one and there is no halt state. Main flow that runs off the
  end of `main.py` starts again from the top with all variables cleared, with
  no interrupt involved, so there is no idle state either: a machine is always in
  main flow or in a handler. Hooks are bound at load, before any statement
  runs, and the binding survives every variable clear; rebinding the name at
  runtime changes a global, not the hook.
- **No events: the world reaches a program only through the queries it polls,
  and the interrupt kinds stay four (Q16).** Vision and sound are queries
  (Q7), and so is anything else `docs/02` gives a machine; a program that wants
  to react to the world reads it, in a loop it writes. Adding a kind — an
  event, a timer, a message — is a new question number. The list a program
  reads is live: what the colony senses now, and nothing it sensed before,
  which Q21 ruled: tiles are remembered, machines are not.
- **The fault record is written by every failure and survives a redeploy
  (Q20).** An exception escaping main flow or a hook, a hook's budget running
  out, and an exception whose unwinding an interrupt cuts off each write it —
  with the bundle version it belongs to — and nothing but the next write
  replaces it. Q18's "cleared by a `redeploy`" is withdrawn.

## Lifecycle

A machine is always in exactly one of two states, and the state is part of the
world and therefore of the state hash:

| State | Meaning | Leaves it when |
|---|---|---|
| **main flow** | executing `main.py`, top to bottom, across as many ticks as it takes; on reaching the end, starting again from the top | an interrupt is delivered |
| **handler** | executing one interrupt's prologue, hook and epilogue | the epilogue completes; the hook escalates; or a higher-priority interrupt preempts |

There is no idle state and no halted state (Q18). A "dying machine" is one in
the `dying` handler. A machine's **fault record** is world state, so any renderer
can show a machine that is faulting and a replay reproduces it; how it is shown is
the renderer's, once Q15 picks one. It is written by every failure (Q20): by
the `fault` prologue when an exception escapes main flow; by the escalation
when an exception escapes a hook or a hook's budget runs out; and by the
abandonment when an interrupt cuts off an exception's unwinding, before that
interrupt's prologue. It holds the exception's class and arguments — or the
kind of hook whose budget ran out — the `file` and `line` of the raising
operation, or of the last operation the hook completed, the tick of the raise,
and the version of the bundle it belongs to. Nothing clears it: not a restart,
not a `redeploy`, not `dying`. The next write replaces it.

**Starting main flow** — at match start, after main flow's last statement,
after a `fault`'s epilogue, or after a `redeploy`'s — means: every global of
every module is cleared, the set of modules already run in this run is
emptied, and `main.py` begins at its first statement. Imports re-run as they are reached
([syntax](syntax.md#import-and-modules)). A program is therefore a body the
machine runs over and over; one that wants memory across runs writes a
`while True:` loop.

## Metering

Main flow does not run to completion inside a tick; it runs until it has spent
the machine's **tick budget** and then yields, resuming from the same operation
boundary next tick with every variable intact. This is the one resumption the
language has (doorway invariant 3) and the one divergence a Python programmer
has to learn rather than avoid.

- **An operation** is one evaluated expression node or one executed statement:
  every name, literal, call, subscript, attribute and operator, and the
  statement that contains them. Each has a row in [costs](costs.md).
- **Budgets are denominated in cost units** — the sum of what the cost table
  charges — not in operations. With every `op.*` constant at one, an
  operation costs one and `a + b * c` as an expression statement costs six
  (three names, two operators, one statement); that is a reading those
  constants make true, not the definition, and a tuning change moves it.
- **The boundaries** are: the point between any two operations; the point
  before the first operation, and after the last, of main flow or of a hook;
  the point after any prologue, before the hook or the epilogue; and an
  escape, which is a boundary at which only the `fault` is delivered
  ([delivery](#delivery) rule 1). Budget checks and interrupt delivery happen
  only at boundaries; an operation, once begun, completes, and is charged in
  full. A game builtin may be a **waiting operation** (`docs/02`): it begins
  an action and the machine yields at the boundary after it until the action
  completes, interrupts delivering at the boundaries inside the wait as
  anywhere. Main flow restarts at most once per tick: a `main.py` with no
  operations reaches its end boundary, restarts, and yields.
- **The tick budget** is a per-machine tuning constant. Each tick, every machine runs
  its slice in turn until its budget is spent or it yields at a restart. The
  order of slices is the tick loop's, which `docs/06` will own; this doc
  assumes ascending entity id and nothing here depends on more than the order
  being fixed. Unspent budget does not carry over; **overspend does**, and
  this is the one statement of the rule: an operation that completes past the
  budget leaves a deficit, each following tick's budget is reduced by what
  remains of it until it is paid, and nothing clears it — not a `fault`, not
  a `redeploy` — so a large traversal costs its full price however the ticks
  fall. A slice whose budget opens at or below zero still opens with its
  boundary, so interrupts are delivered to a machine in deficit; only operations
  wait. Inside a hook the same cost is debited in full from the hook budget as
  well, and at a boundary inside a hook the hook budget is checked before the
  tick budget, so exhaustion escalates before a yield could hide it.
- **Yielding is invisible to the program.** No variable changes, no exception
  is raised; the next tick continues. World state may have changed in between,
  which is the point: a program that reads the world once and acts for a
  hundred ticks acts on stale data, and that is the player's problem to solve
  with a loop, not the sim's.

## The cost model

The lever Q5 chose to own. [costs](costs.md) holds one row for every
operation, builtin and method, each a formula over named constants; the
constants' values live in `data/language/costs.toml`, which the language
crate loads, so a cost can be tuned without touching the spec and every tuning
is hash-affecting in the ordinary way. What is fixed here is the shape:

- Every operation has a base cost, and one that traverses, copies or sorts a
  collection or string pays per element or scalar on top of it.
- A call pays per argument bound; a class instantiation pays its `__init__` as
  a call; a dunder dispatch is the call it is.
- **Game builtins** — sensing, moving, acting — have rows in the same file,
  shaped the same way, which `docs/02` fills; a sensing query pays per element
  of the sorted result it returns.
- Prologues and epilogues cost the player nothing.

The budget is what makes cost a design lever: a machine that senses expensively
acts less often per tick than one that does not, and the trade is legible to
the player because the costs are published numbers.

## Interrupts

A machine runs in **main flow** until an interrupt is **delivered**, at which point
it runs that interrupt's **handler**: a locked system **prologue**, then the
player's **hook** if the kind has one, then a locked system **epilogue**. The
epilogue is what a hook earns by returning: a hook that escapes or runs out of
budget gets escalation instead, and a preempted handler gets nothing. Only
`death`, which has no hook, has an epilogue nothing can skip. A kind whose hook
is not defined still has its handler — prologue then epilogue, back to back.

### Kinds

Highest priority first. The set is closed; adding a kind is a new question.

| Kind | Raised by | Hook | Budget | Epilogue |
|---|---|---|---|---|
| `death` | the world, when things are bad enough (`docs/02`); `dying`'s epilogue; a fault inside `on_dying` or its budget running out | none | — | the machine leaves the world |
| `dying` | the world, when the machine is destroyed in the ordinary way (`docs/02`); a fault inside `on_fault` or its budget running out | `on_dying()` | yes | raises `death` |
| `fault` | the machine's own code: an exception nothing caught | `on_fault(e)` | yes | main flow starts over, all variables cleared |
| `redeploy` | the command log: a program update agreed for a tick (Q12) | none | — | the machine takes its role's current bundle, all variables cleared, main flow starts over |

What each prologue and epilogue does to the machine's *body* — whether a dying
machine stops moving, whether a redeploying one drops what it carries — is
`docs/02`'s. This part fixes only what they do to *execution*.

### Hooks

A hook is a top-level `def` in `main.py` with one of the two closed names.
Defining either is optional. Any other `on_` name is an ordinary function.
`on_fault` takes exactly one parameter and `on_dying` none; a hook of another
arity, or two top-level `def`s of one hook name, is a load error.

**Hooks are bound at load, not when their `def` runs.** The loader finds the
two names among `main.py`'s top-level `def` statements and binds them before
any statement executes, and that binding survives every variable clear — a
fault on line 1 of `main.py` still runs an `on_fault` defined on line 50, and
a restarted main flow has its hooks before it has anything else. Rebinding the
name at runtime (`on_fault = other`) changes a global and not the hook.
Whether a hook exists is therefore a property of the bundle, fixed at load,
and a machine's lifecycle state never depends on how far its program has run.

```python
def on_fault(e):
    # e: the exception; str(e), e.file, e.line, e.tick, e.args
    report(f"{e.file}:{e.line}: {e}")     # report() is a game builtin

def on_dying():
    drop_cargo()                          # whatever docs/02 lets a dying machine do
```

- `on_fault(e)` receives the exception object exactly as an `except` clause
  would have, with `file` and `line` locating the operation that raised it.
- Hooks see globals as they were when the interrupt was delivered. Main flow's
  locals are gone: a `fault` abandoned the frame that held them.
- A hook's code is metered like main flow and may span ticks; its **hook
  budget** is a total per invocation, a tuning constant per kind, debited in
  full for every operation alongside the tick budget. It is exhausted when the
  total debited **exceeds** it, checked at every boundary including the one
  after the hook's last operation — a hook whose total equals its budget has
  returned; that abandons the hook and escalates
  exactly as an escaping exception would ([escalation](#escalation)). It is not
  an exception and cannot be caught.

### Delivery

1. **A `fault` exists only when an exception escapes.** Until then an
   exception is ordinary control flow: it unwinds through every enclosing
   `except` and `finally` ([syntax](syntax.md#exceptions)) as ordinary
   operations, metered and interruptible like any other. A caught exception
   was never an interrupt. Any interrupt — a `redeploy` included — may be
   delivered at a boundary inside the unwinding and abandons it like any
   code; when that happens the exception being unwound is **written to the
   fault record as if it had escaped**, and no hook runs for it, so nothing
   is silent and nothing holds an interrupt off. When nothing has caught it
   and the outermost frame of main flow (or of the running hook) is gone, the
   `fault` is delivered **at that instant, before any pending interrupt is
   considered** — the escape is a boundary at which only the `fault` is
   delivered — located at the
   operation that raised it, with the record's tick the tick of the raise,
   which the exception carries. A pending `dying` or `death` then preempts at
   the fault handler's first boundary, after the prologue has written the
   record. A `finally` that raises replaces the exception being unwound, as in
   Python.
2. **A world-raised interrupt** (`dying`, `death`) **and a `redeploy`** are
   delivered at the machine's **next operation boundary** — within the same tick
   if the machine still has budget, otherwise at the start of its next slice. The
   boundary before the first operation counts, so a machine whose main flow was
   just restarted takes a pending `redeploy` before executing anything. Every
   machine is always running something, so every machine has a next boundary.
3. **Priority.** If a handler is running, an interrupt of higher priority
   **preempts** it: the running handler is abandoned where it stands, hook and
   epilogue both, and the new handler's prologue runs. There is no handler
   stack. A hookless handler has a boundary after its prologue, so a pending
   higher kind preempts a `fault` handler there — after the record is written,
   before the epilogue restarts anything. An interrupt of equal or lower priority
   **waits** until the running handler's epilogue completes, then is delivered
   at the next boundary. An exception raised by the running hook's *own* code
   is not an arriving interrupt: it is delivered at once by rule 1 and goes
   where [escalation](#escalation) says, whatever its rank against the hook.
4. **Coalescing.** At most one pending entry per kind: a second `dying`,
   `death` or `redeploy` arriving while one is pending or running merges into
   it. A `redeploy` carries no bundle of its own — its epilogue loads whatever
   the role's slot holds at swap time — so coalescing cannot deliver a stale
   version. A `fault` is never pending: it exists at the moment an exception
   escapes and is delivered at that moment (rule 1).
5. **The pending set survives everything except `death`.** A `fault`'s
   epilogue and a `redeploy`'s epilogue clear variables, not pending
   interrupts. Since nothing ranks below `redeploy`, a pending entry at a
   `redeploy` is always higher and takes effect at the next boundary.

### Escalation

Escalation is the failure path of a hook: an exception that escapes it, or
its budget running out, abandons the hook *and its epilogue* and delivers the
next kind up at once. The chain fault → dying → death is strictly increasing,
so it terminates.

- A fault inside **main flow** delivers `fault`. The prologue records the
  fault; `on_fault` runs if it is defined; the epilogue restarts main flow.
  With no `on_fault`, prologue and epilogue run back to back. There is no
  system default handler and no halt state: what the player gets by writing
  nothing is a restart and a fault record, and a program that faults on its
  first line restarts every time, visibly.
- A fault inside **`on_fault`**, or its budget running out, escalates to
  **`dying`**. The fault record is rewritten for it (Q20); `fault`'s epilogue
  does not run — the machine is dying, and a software failure still gets last
  words.
- A fault inside **`on_dying`**, or its budget running out, escalates to
  **`death`**. The record is rewritten for it; `dying`'s epilogue does not
  run, since it would only have raised `death`, which is now delivered
  directly.
- `death` has no hook, so nothing can fail in it; nothing can skip its
  epilogue.
- **`death` during `on_dying`** preempts it: `on_dying` and `dying`'s epilogue
  are abandoned, and `death`'s prologue and epilogue run.
- **`redeploy` during `on_fault`** waits. `on_fault` returns, its epilogue
  restarts the old main flow, and delivery rule 2 delivers the swap at the
  boundary before the first operation, so the old program executes nothing
  further. If `on_fault` escalates instead, the machine is dying and the
  `redeploy` is moot.

### Worked example

A machine is in main flow with three cost units of budget left when its role is
redeployed on this tick and, on the same tick, it takes lethal damage.

1. It completes the operation in progress. At the boundary, two interrupts are
   pending: `redeploy` and `dying`. `dying` is higher and is delivered.
2. `dying`'s prologue runs. `on_dying()` runs, spending the machine's remaining
   budget this tick and continuing next tick.
3. If `on_dying` returns, `dying`'s epilogue raises `death`; if it escapes or
   exhausts its hook budget, `death` is delivered by escalation and the
   epilogue is skipped. Either way `death`'s epilogue removes the machine, and the
   pending `redeploy` is discarded with it.

Had the damage not been lethal, step 1 would have delivered `redeploy`
instead: prologue, epilogue, and main flow starts at the top of the new
`main.py` — this tick, if budget remains.

## Limits

Every limit is spec; every value is a tuning constant in
[data/language/limits.toml](../../data/language/limits.toml), one key per row
below that has a value, which the language crate loads beside the cost table
(T10). Exhausting one has exactly the effect listed. There are no others.

| Limit | Bounds | On exhaustion |
|---|---|---|
| tick budget | cost units per machine per tick, main flow or hook; a deficit carries to the next tick | yield; resume next tick |
| hook budget, per kind | cost units per hook invocation, total across ticks, debited in full for every operation | abandon the hook and its epilogue; escalate — `on_fault` to `dying`, `on_dying` to `death` |
| call depth | nested calls, including recursion and `__init__` | `RecursionError` |
| nesting depth | how deep a walk may descend. A non-container has depth 0 and a container one more than its deepest element; the operations that walk — `==`, `!=` and `<` on containers, `in` on a container, `list.index`, `list.count`, `list.remove`, `tuple.index`, `tuple.count`, sorting and `min`/`max` through their comparisons, `str`, a `tuple` used as a `dict` or `set` key, and a sequence or mapping `match` pattern — raise when they would descend past it. Building a container never walks: a display, `append` and assignment write references without descending. Brackets and blocks nested in source have the same bound | `LimitError` at the operation that would descend past it; a parse error at load for source |
| collection size | elements in any one list, tuple, dict or set; scalars in any one `str` | `LimitError` at the operation that would grow it, raised before its first write — the target is unchanged and only the base cost is charged |
| live values | elements and scalars reachable from the machine's globals and frames. A list, dict, set or instance is an object counted once however many names or containers reference it; a `str` or `tuple` is a value, counted once per variable or container slot that holds it, so `[s] * 10` holds ten copies of `s`. Measured at every operation that creates a value or stores a reference — an assignment, an `append`, a display | `LimitError` at that operation |
| bundle size | files per bundle, which is also the module limit; bytes per file | refused at load |
| pending interrupts | one entry per kind, by construction; no value, since nothing can tune it | cannot be exhausted |

A self-referential container — `a = []; a.append(a)` — is legal under the
live-values rule; any operation that walks it hits the nesting depth and
raises `LimitError`, so no walk the interpreter performs is unbounded.

`LimitError` and `RecursionError` are ordinary exceptions: a program may catch
them, and one that does not faults like any other. Budgets are not exceptions;
they yield or abandon, and no code observes them.
