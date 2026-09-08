*Closed record — see [../README.md](../README.md). Not spec.*

# Q25 — Combat: what deals damage beyond a fault, whether bots attack, whether a building defends, and whether anything repairs

## Ruling

*Ruling (2026-09-08):* **bots attack, and nothing else does.** A bot has an
`attack` action with a **range** and a **damage**, both tuning constants per
model in data and both starting at one: a bot hits an adjacent machine for
one health. Any machine with health — a bot, a building, a site — can be
hit, on any team, the attacker's own included. No building defends by rule,
nothing repairs, and terrain and the world deal no damage; each of those is
a widening under a new number if the opposition (`docs/04`) or play asks for
it. This closes the cause list Q24 left open and unblocks `docs/04`.

### The rules

Each is hash-affecting.

1. **`attack(id)`** is a bot action (`02`): `id` names a machine the team can
   currently see (Q7) — a sighting's `id`, so a program cannot shoot what the
   colony cannot see — whose position is within `attack_range` of the
   attacker by squared Euclidean distance in `num`, and in the attacker's
   line of sight by `03`'s ray-walk. Any of those failing is a `ValueError`
   before anything begins.
2. **An attack is a waiting action** taking `attack_ticks`. When it
   completes, if the target still exists and is still within range and
   sight, `attack_damage` is subtracted from its health in the tick's damage
   step (`06`, step 4); if not, the attack completes with no effect. The
   attacker is not moved and its target is not stopped.
3. **Damage lands in the damage step**, after every action completion of
   the tick, so two bots that hit each other on the same tick both take the
   hit, and a machine's `dying` or `death` (Q24) is raised there.
4. **Anything with health is a target**: bots, buildings and sites, of any
   team. Attacking a teammate is legal — the sim has no reason to forbid it,
   and a program that does it is a program the player can fix.
5. **An attack is loud.** It emits a sound with cause `"attack"` at the
   attacker's position when it begins, with a loudness in data, so a fight
   is heard before it is seen.
6. **A dying machine cannot attack**: `on_dying` may only sense, log, wait,
   rename and drop (`02`), and that list does not change.
7. **Range and damage are per model**, so a second bot model (a new
   question number) can differ; today the one model has both at one.
8. **Order**: attacks begin in slice order and complete in ascending entity
   id like every action; damage from every attack completing on a tick is
   summed per target before the death rules run, so a target that took two
   hits of one crosses zero once.

### Consequences

- **`docs/04` is unblocked.** The opposition can be bots on another team,
  running programs the game ships, attacking by this rule.
- **Combat is programming.** Whom to attack, when to flee and where to
  stand are the program's, and the numbers are small enough that a fight is
  long enough to patch a program during (Q3).
- **Deconstruction remains the other way to hurt a rival** (Q28), and it is
  faster against a building than attacking it one health at a time; attack
  is for what deconstruction cannot reach — a bot, or a building guarded by
  bots.
- **`02`'s attribute table gains nothing** — health is already there — and
  its action table gains one row, its sound table one cause.

## Outcome

- **Docs:** [02-machines.md](../../02-machines.md) — the action, the sound
  cause, the interrupt table's causes, the tree. [00-overview.md](../../00-overview.md)
  — the reserved-docs table. [QUESTIONS.md](../../QUESTIONS.md) — the
  status block.
- **Task:** [T9](../../TASKS.md) — Q25 leaves its list; `docs/04` is
  writable.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Bots attack: an `attack` action with a range, a damage, and a duration** *(chosen, at range one and damage one)* | The smallest combat that makes programs matter — targeting is code. Damage per hit and range per model, and the first tuning that decides matches. |
| Buildings defend: a turret model that fires by rule, no program needed | Territory becomes defensible without writing a fighter. Passive, so defence is placement — the trade the design has declined twice already. A widening. |
| Both, with repair as a bot action that spends ore | The full surface: every number above, twice, plus a repair rate and cost, and a balance problem Q4 chose to defer. |
| Neither yet — faults are the only damage until PvE needs more | Nothing to tune, and `dying` fires only from bugs. Defers the failure mode the fleet fantasy wants until `docs/04` demands it. |

### How the answer took its shape

The user's call: bots attack, adjacent, one damage. The ruling's work was to
fit it to what existed. Targeting by sighting id makes fog matter to combat
without a new rule; the damage step in `06`'s tick already existed for
faults and makes simultaneous kills symmetric; and the small numbers are
deliberate — a fight that takes many ticks is a fight a player can watch go
wrong and redeploy into, which is the game. Turrets lost for the reason
they lost before: a thing that fights without a program is a thing the
player did not write.
