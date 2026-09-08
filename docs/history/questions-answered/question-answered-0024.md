*Closed record — see [../README.md](../README.md). Not spec.*

# Q24 — What destroys a machine: health, damage, and the causes of `dying` and `death`

## Ruling

*Ruling (2026-09-08):* **every machine has health; damage lowers it; a
machine whose health reaches zero is `dying`, and one whose health falls
past a threshold below zero, or whose tile is destroyed, is `death`. A fault
costs one health.** What deals damage beyond that — combat, terrain, the
opposition — is left open under Q25, so this ruling gives `docs/02` its
attribute and its causes without deciding how machines fight.

This ruling also **amends [Q23](question-answered-0023.md)**: the fleet is
bounded not by printer throughput alone but by a **cap of ten bots per
printer**, per team, and each printer **unlocks a deployment**. Made under
this number because Q23's file is a closed record.

### The rules

Each is hash-affecting; every number is tuning in data.

1. **Health** is an attribute of every machine, a `num`, starting at the
   kind's maximum and never above it. Sites have health too, so a site can
   be destroyed before it completes.
2. **Damage** is the one mechanism: a cause names an amount, the amount is
   subtracted, and what follows depends only on the result. Causes today
   are one — a **fault costs one health**, charged by the `fault` prologue
   beside the record it writes, so a fault-restart loop is finite and fifty
   faulting bots die rather than spin. Every other cause is Q25's.
3. **`dying` is raised when health reaches zero or below**, at the boundary
   where the damage lands. **`death` is raised directly** when health falls
   to or below the kind's death threshold — a negative number in data, so
   overkill skips last words — or when the machine's tile is destroyed
   (`docs/03` says whether that can happen). A machine already in `dying`
   whose health crosses the threshold takes `death` by preemption, as Q17
   ruled.
4. **Health does not regenerate** by itself. Repair, if it exists, is a
   Q25 action.
5. **A dying machine's health is frozen** at what it was when `dying` was
   raised, so a hook cannot be pushed into `death` by further faults of its
   own: a fault inside `on_dying` escalates as Q18 rules and deals no damage.
6. **The bot cap.** A team may have at most `bots_per_printer` bots per
   printer it owns, sites excluded. A `print` when the team is at its cap is
   a `ValueError` fault. Losing a printer lowers the cap; bots above it are
   kept and simply cannot be replaced until the cap rises.
7. **Deployments are unlocked by printers.** A team has as many bot
   deployments as it has printers, plus one deployment per building kind,
   which are not counted. Bot deployments are named by a fixed color list in
   data, in order — the first printer unlocks the first color — and losing a
   printer removes the *last* unlocked deployment; bots on it keep running
   its bundle until they die, and `print` of a locked deployment is a
   `ValueError`. Deploying to a locked deployment is accepted into the
   command log and takes effect when it unlocks.

### What each doc owns

`docs/02` owns health, the cap and the deployment rules as attributes, a
precondition on `print`, and a table. `03` says whether a tile can be
destroyed. `04` and Q25 own what fights.

### Consequences

- **`docs/02` gets its attribute** and its interrupt table gets its first
  world cause: a fault, which is not the world but the program, and is
  therefore the one cause every machine faces from tick one.
- **Q23's bound is replaced.** "Printers and time" becomes "printers, times
  ten"; the time bound still holds inside it. A second printer still costs
  ore, so the economy still bounds the fleet one step removed.
- **Q25 is opened** for combat: what deals damage, whether machines attack,
  whether buildings defend, and repair.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — Decided section,
  attributes, actions, the interrupt table, and the deployment section.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Question:** [Q25](../../QUESTIONS.md) — combat.
- **Task:** [T9](../../TASKS.md) — Q24 leaves its list and Q25 joins it.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| No combat — colonies compete on economy and territory only | Nothing to specify, and `dying` never fires from the world. The fleet fantasy loses its failure mode, and PvE has no teeth. |
| Bots attack: an `attack` action with a range, a damage, and a cooldown | The smallest combat that makes programs matter. Deferred to Q25, not rejected. |
| Buildings defend: a turret kind that fires by rule | Territory becomes defensible without writing a fighter. Deferred to Q25. |
| **Health and damage as a mechanism now; causes later** *(chosen)* | `02` gets its attribute and its first cause today, and the combat design gets its own number rather than riding on a doc pass. |

### How the answer took its shape

The user's feedback on `docs/02` asked for health on everything and a fault
that costs health, which together are the mechanism half of this question
without the combat half. Splitting them lets `02` be complete now and keeps
combat — the largest tuning surface the game will have, and the one Q4
deferred PvP over — from being ruled in passing. The cap and the deployment
unlock came from the same feedback; they belong here because they are what
makes a printer worth defending, which is what combat will be about.
