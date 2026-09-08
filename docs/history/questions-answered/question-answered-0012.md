*Closed record — see [../README.md](../README.md). Not spec.*

# Q12 — How is a program update scheduled in lockstep?

## Ruling

*Ruling (2026-09-08):* **a fixed delay, and a stall for a late peer.** A
command submitted while the sim is at tick `t` is agreed for tick
`t + delay`, where `delay` is a tuning constant in ticks; every peer applies
it on that tick. A peer that has not received every other peer's commands
for tick `t + 1` by the time it would run that tick **stalls** — issues no
tick — until it has. Nothing is dropped and nothing desyncs; a slow peer
slows everyone, which is the cost lockstep pays for having no other state to
reconcile.

### The rules

1. **Every tick has a command set** from every peer, possibly empty. A peer
   sends, for each tick it reaches, the commands it has for that tick, or an
   explicit "none". A tick may not run until every peer's set for it is in
   hand.
2. **Submission targets `now + delay`.** A command entered at wall-clock
   time while the peer's sim is at tick `t` is stamped for tick
   `t + delay` and sent. `delay` is in ticks (Q6), so it is a long time at a
   slow speed and a short one at a fast speed, and it is one number in data.
3. **The stall is total.** A stalled peer runs no ticks, and since every
   peer waits on every other, a stall on one is a stall on all. The renderer
   keeps drawing the last completed tick. A stall longer than a limit in
   data is reported to the player; what then — wait, or resign the absent
   peer — is a `docs/06` matter and not a sim one.
4. **Single-player is lockstep with one peer**, so the same code runs with
   `delay` still applied: a deploy pressed at tick `t` lands at
   `t + delay` in single-player too. That is the rule CLAUDE.md states, and
   it means the delay is felt and tuned from the first session.
5. **Ordering within a tick** is by sender then by submission order within
   the sender, so two peers' commands for one tick apply in a fixed order
   on every peer.
6. **Rate-limiting is deferred with PvP** (Q4). The hook is Q10's: every
   command carries its sender and its tick, so a per-team budget per tick
   is a filter at submission, not a format change.
7. **The delay is spec for a match**: it is fixed at match start from data
   and never changes mid-match, since a change would itself have to be
   agreed for a tick under the old delay.

### Consequences

- **T7's `Command` and the tick loop are shaped**: a set per tick per peer,
  a stamp of `(sender, tick)`, and a driver that waits.
- **Speed and delay trade off in the player's hands** (Q6): a player who
  wants snappier deploys can slow the sim, and the delay in real seconds
  shrinks with the tick.
- **Q12's own "what if a peer misses" is answered by the stall**, and the
  register's third option, the barrier, turns out to be what the stall is
  once the delay has been spent — the delay buys the slack, the barrier
  guards the rest.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, held
  there until `docs/06` exists. [QUESTIONS.md](../../QUESTIONS.md) — the
  status block.
- **Task:** [T9](../../TASKS.md) — Q12 leaves its list. [T7](../../TASKS.md)
  — `Command`'s stamp.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Fixed turn delay (apply at tick `now + N`), stall on a late peer** *(chosen)* | The classic RTS answer: simple, predictable, and the delay is a tunable felt directly as input lag. Picking `N` trades responsiveness against tolerance for a slow peer; the stall is the tolerance running out. |
| Delay negotiated from measured latency | Feels better on good connections and adapts to bad ones. The negotiation itself becomes shared state that must be deterministic. |
| Lockstep barrier — the tick does not advance until every peer has acked | No input lag and no wrong guesses, but one slow peer stalls everyone, which is the failure mode lockstep games are most hated for. |

### How the answer took its shape

The user's call, and the conventional one. The alternative to a stall on a
late peer is to guess — run the tick without the missing commands and
reconcile later — and reconciliation is a second copy of the world with a
rollback, which is exactly the state this design refuses to have. A fixed
delay makes the guess unnecessary most of the time; the stall makes it
unnecessary the rest of the time. What the delay costs is felt, and Q6 puts
the knob that adjusts the feel in the players' hands.
