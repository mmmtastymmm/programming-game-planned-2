*Closed record — see [../README.md](../README.md). Not spec.*

# Q18 — Amend Q8 and Q17: the epilogue rewards success, and a unit is never halted or idle

## Ruling

*Ruling (2026-09-06):* three amendments to the interrupt model of
[Q8](question-answered-0008.md) and [Q17](question-answered-0017.md), made
under a new number because both files are closed records. Everything in Q8,
Q11 and Q17 not named here stands.

**The epilogue runs only when the hook returns.** A handler is a locked
prologue, then the player's hook if the kind has one, then a locked epilogue —
and the epilogue is the last step of a *successful* run, not a guarantee. If
the hook raises an exception that escapes, or exhausts its budget, the epilogue
does not run: control **escalates** to the next kind up. `fault`'s hook
escalates to `dying`; `dying`'s hook escalates to `death`. `death` has no hook,
so its epilogue always runs — it is the one unconditional epilogue in the
model, and the reason `death` has no hook. Q8 had every epilogue unconditional,
which was a stronger claim than the split in Q17 needed.

**Every kind's handler exists whether or not its hook is defined.** A handler
with no hook is prologue then epilogue. So a `fault` in a unit with no
`on_fault` runs the fault prologue, which records the fault, and the fault
epilogue, which restarts main flow. **There is no halt state.** Q8's "halts,
visibly, until the next redeploy" is withdrawn: the visible signal is the
fault record in state and the unit restarting, and the restart is an epilogue
rather than a hidden second program, so the cost that rejected the fallback
option is not incurred.

**Main flow that runs off the end of `main.py` starts again from the top,**
all variables cleared, with no interrupt involved. **There is no idle state.**
A unit is always in main flow or in a handler; a program is a body the unit
runs over and over, and one that wants memory across runs writes a
`while True:` loop.

### The rules

Replacing or adding to Q8's list. Each is hash-affecting.

1. **Escalation is the failure path of a hook.** An exception escaping the
   hook, or the hook's budget running out, abandons the hook and the epilogue
   and delivers the next kind up at once. Budget exhaustion is not an exception
   and cannot be caught.
2. **Preemption abandons the whole handler, epilogue included.** A higher
   interrupt delivered during a handler drops that handler where it stands.
   `death` during `on_dying` simply runs `death`'s prologue and epilogue; there
   is no `dying` epilogue left to raise a second `death`.
3. **A missing hook is an empty hook.** The prologue and epilogue run
   back-to-back.
4. **Main flow's end is a restart**, identical in effect to a `fault`'s
   epilogue — every global cleared, `main.py` from its first statement — but
   raised by nothing and delivered at the boundary after the last statement.
5. **The fault record is written by the `fault` prologue**, before any hook,
   so it exists whether the hook runs, fails, or is absent. It is the unit's
   last fault: exception, file, line, tick. It survives restarts and is cleared
   by a `redeploy`.
6. **Hooks are bound at load, not when their `def` runs.** The loader finds
   `on_fault` and `on_dying` among `main.py`'s top-level `def` statements and
   binds them before any statement executes; the binding survives every
   variable clear, so a fault on line 1 still runs a hook defined on line 50,
   and rebinding the name at runtime changes a global and not the hook.
   Whether a hook exists is a property of the bundle, fixed at load — which
   rule 3 needs, since an empty hook and an absent one must be the same thing
   at every point in the program's run.

### Consequences

- **Q11's halted-unit clause is moot.** A `redeploy` reaches every unit at its
  next operation boundary; there is no unit with nothing running.
- **The states are two.** Main flow and handler. Lifecycle is simpler to hash,
  and no state depends on whether a hook was written.
- **A fault-restart loop is legal and visible.** A program that faults on its
  first line restarts every time, spending its tick budget on the loop. The
  record is in state and a renderer can show a unit that is faulting every
  tick; the sim does not stop it, because stopping it was the halt state.
- **`docs/01` changes in place**, since it is uncommitted at the time of this
  ruling, and carries this ruling in its Decided section beside Q8 and Q17.

## Outcome

- **Docs:** [01-language/execution.md](../../01-language/execution.md) —
  Decided section, the lifecycle, the interrupt rules, the limits table.
  [01-language/syntax.md](../../01-language/syntax.md) — the divergence list.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T8](../../TASKS.md) — its must-pin list no longer names a halt
  state.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| the epilogue | Unconditional, as Q8 ruled | A hook that faults still gets its epilogue, so `on_fault` faulting restarts main flow *and* escalates to `dying` — two things happen for one failure, and the second makes the first moot. |
| the epilogue | **Only on success** *(chosen)* | A failing hook gets nothing but escalation. `death` alone is unconditional, because it alone has no hook — which is exactly the structure Q17 built. |
| no `on_fault` | Halt, as Q8 ruled | A distinct state to hash and to render, and the one case where writing nothing behaves differently from writing `def on_fault(e): pass`. |
| no `on_fault` | **Empty hook, restart** *(chosen)* | Writing nothing and writing an empty hook are the same thing. The signal survives as the fault record and the visible restart loop. |
| end of `main.py` | Idle, as `docs/01` first drafted | A third state, and a unit that silently does nothing after a program the player thought was continuous. |
| end of `main.py` | **Restart from the top** *(chosen)* | A program is a body run over and over; memory across runs is an explicit loop. Nothing silent. |
| end of `main.py` | Restart keeping globals | Top-level assignments re-run and reset most of them anyway, so what survives is exactly the globals a program did not assign at top level — a rule nobody would predict. |

### How the answer took its shape

Q8's "unconditional epilogue" was introduced to make one guarantee — a dead
unit leaves the world — impossible for player code to skip. Q17 then made that
guarantee structural by giving `death` no hook at all, and at that point the
unconditional epilogue was doing no work anywhere else: for `fault` and `dying`
it produced a second effect on top of escalation. Reading the model as "the
epilogue is what a hook earns by returning" removes the redundancy and leaves
`death` as the single unconditional step, which is what it was always meant to
be.

The halt state fell to the same reading. If every kind's prologue and epilogue
always run, then a unit with no `on_fault` still has a fault handler; it is the
empty one. Idle fell to the observation that a unit with nothing to do is a
unit whose program the player must have meant to loop.
