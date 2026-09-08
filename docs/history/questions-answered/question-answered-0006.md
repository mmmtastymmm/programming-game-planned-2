*Closed record — see [../README.md](../README.md). Not spec.*

# Q6 — What is the tick, and how many ticks per second?

## Ruling

*Ruling (2026-09-08):* **the sim has no rate. A tick is the unit of
simulated time, every duration in the corpus is a count of ticks, and how
many ticks pass per real second is a *speed* the players choose together
during the match** — a command in the log like a deploy, agreed for a tick
(Q12) so every peer changes speed on the same tick and the experience stays
shared. The sim never reads the speed: it advances one tick when told to and
knows nothing about wall-clock time (determinism rule 4). The renderer (Q15)
draws between ticks by interpolation, so a slow speed looks smooth and a
fast one looks fast.

### The rules

1. **A tick is one pass of the tick loop** (`docs/06`): every machine's
   slice in order, then regrowth, then the vision pass, then the state hash.
   Nothing in the sim is finer than a tick, and nothing in it is a second.
2. **Speed is a command**, `SetSpeed(ticks_per_second)`, in the command log
   beside deploys, agreed for a future tick under Q12's delay, and applied
   by every peer's *driver* — the loop outside the sim that decides when to
   call the next tick — on that tick. The value is a `num` from a closed
   list in data, so that "pause" (`0`) and each step are the same on every
   peer, and a replay records the speed changes with everything else,
   though it may play them back at any speed it likes.
3. **Any player may change the speed** in PvE. In PvP that is a fairness
   question and is deferred with PvP (Q4); the hook is that the command
   exists and carries its sender.
4. **The starting speed is the map's** (`docs/03`), a tuning constant, and
   a match at speed `0` is paused: the driver issues no ticks, the command
   log still accepts commands for a future tick, and the first speed change
   above zero resumes.
5. **Nothing in the sim depends on real time**: no builtin reads a clock,
   no limit is in seconds, and the tick budget (`docs/01`) is per tick
   whatever the speed. A faster speed is more sim work per real second and
   nothing else; the spike's measurement says that work is a few percent of
   a core at sixty ticks per second and two hundred machines.

### Consequences

- **Every duration in data is already right.** A 30-tick print is three
  seconds at ten ticks per second and half a second at sixty; the number
  does not move, the players' choice does.
- **Q12's window is in ticks too**, so at a slow speed the fixed delay is a
  long time and at a fast one a short time; the delay does not adapt, the
  players do.
- **Q15's renderer must interpolate**, since at a slow speed a machine
  would otherwise jump a tile at a time; interpolation reads two completed
  snapshots and draws between them, and feeds nothing back.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, held
  there until `docs/06` exists. [QUESTIONS.md](../../QUESTIONS.md) — the
  status block.
- **Task:** [T9](../../TASKS.md) — Q6 leaves its list.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Slow tick (5–10/s), one action per tick | Cheap to simulate, easy to reason about, and a program's cost is legible. Movement looks stepped unless the renderer interpolates. |
| Medium tick (20–30/s) | Conventional and smooth. More sim work per second — but not more than the sim can afford. |
| Fast tick (60/s) | Renderer and sim agree; no interpolation needed. The most sim work per second, and the tightest window for Q12's scheduling. |
| Decouple: slow sim tick, interpolating renderer | Best of both, at the cost of a rendering layer that must never feed anything back into the sim. |
| **No fixed rate: the players choose the speed, as a synchronised command** *(chosen)* | Every row above becomes a setting. Costs the interpolating renderer of the fourth row, a driver outside the sim that owns the clock, and one more command kind. |

### How the answer took its shape

The register's four options all assumed the sim had a rate. The observation
that broke that was that the sim is written entirely in ticks and never
needs one: a rate is the driver's business, and a driver that takes its rate
from the command log makes speed a shared, replayable choice rather than a
constant. That is the fourth option — decoupled sim and renderer — with the
constant removed, and it costs nothing the fourth option did not already
cost. Making speed a *command* rather than a local setting was the user's
call, so that a slow-motion moment or a pause is something both players
experience, and so a replay can show it.
