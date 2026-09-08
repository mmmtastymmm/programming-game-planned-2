*Closed record — see [../README.md](../README.md). Not spec.*

# Q10 — What else, besides program updates, may enter the sim mid-match?

## Ruling

*Ruling (2026-09-08):* **the player may mark the map, and everything the
player does enters the same ordered command log.** Besides a deploy (Q3),
the command log carries **tile marks** — a blueprint, a paint color, or a
layer placed on a tile by the player pointing at it — the **speed** (Q6),
and **match control**. A mark changes what programs can *read* about a
tile and never what a machine *does*: no command moves, builds, prints or
targets. That is the line Q3 drew, restated for a wider input surface: the
player may author programs and mark the map; the player may not command a
machine.

### The rules

Each is hash-affecting.

1. **One log, one ordering.** Every command — `Deploy`, `Mark`, `Unmark`,
   `SetSpeed`, `Resign` — is agreed for a future tick under Q12's delay and
   applied by every peer on that tick, in the order the log gives them.
   There is no second channel, and a replay is `(map, command log)` as
   `docs/03` says.
2. **A mark is a per-tile, per-team layer** with three kinds, each a slot a
   tile has for each team:
   - a **blueprint**: a building model, meaning "build this here". A bot's
     `build` (`docs/02`) may name a tile that carries its team's blueprint
     for the same model, or any tile as today; and a new builtin
     `blueprints()` returns the team's blueprint tiles, sorted by row then
     column, so a program can build what the player drew. A blueprint is
     removed when the building completes, or by `Unmark`.
   - a **paint**: a color from the deployment color list, meaning whatever
     the program that reads it decides — a rally point, a no-go zone, a
     route. `tile()` and `tiles()` (`docs/02`) return it, and `painted(color)`
     returns the team's tiles of that color.
   - a **layer**: a `str` label from a closed list in data, meaning whatever
     the program decides, for the cases a color is too few for. Read the
     same way; `layered(label)` returns the tiles.
3. **Marks are visible to their team only**, are not sensed, are not part of
   memory, and are never seen by another team. They are world state, in the
   hash, because a program reads them.
4. **A mark on an unknown tile is legal.** The player may mark fog; the
   program decides what to do with a blueprint it cannot yet reach.
5. **Match control is `Resign`**, which ends the match for the sender's team
   on the agreed tick: its machines are removed, its marks cleared, and the
   sim continues for the rest. An agreed draw is a widening if PvP wants it.
6. **The command format carries its sender** (a team) and the tick it is
   agreed for, on every kind, so rate-limiting and fairness (Q12, PvP) have
   their hook without a format change.
7. **A command that cannot apply on its tick is dropped**, not deferred: a
   deploy to a locked deployment is the one exception `docs/02` already made
   (it waits for the unlock), a mark on a tile off the map is refused at
   submission, and a `Resign` from a team already out is ignored. Nothing
   faults, since no program is involved.

### What each doc owns

`docs/06` owns the command log's format and the tick it applies on. `02`
gains the three read builtins and the blueprint clause on `build`. `03`
gains the mark layers as a per-team field of a tile. The renderer (Q15) owns
how a player draws a mark and how it is shown.

### Consequences

- **The line Q3 drew holds, restated.** Author programs, mark the map,
  never command a machine. A blueprint is an instruction to a *program*, not
  to a bot; a bot that ignores blueprints is a program the player can fix.
- **Marks are the first per-team world state that is not a machine**, and
  they are hashed, so a replay reproduces them.
- **`docs/02` and `03` gain rows.** Three builtins with `[game]` costs, a
  clause on `build`, and a field on the tile record.
- **T7's `Command` is now shaped**: five kinds, a sender and a tick.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, held
  there until `docs/06` exists, and the Q3 bullet, whose open pointer to
  this question is now its answer. [02-machines.md](../../02-machines.md) —
  the builtins and `build`. [03-world.md](../../03-world.md) — the tile's
  mark field. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T9](../../TASKS.md) — Q10 leaves its list. [T7](../../TASKS.md)
  — `Command`'s shape.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Nothing else — program updates are the whole input surface | Cleanest. Leaves no way to resign and no way to end a stalemate early, and no way for the player to point at the map. |
| Plus match-control only (resign, agreed draw) | Keeps every sim-affecting input a program update while remaining playable. A second command category with different rules is a permanent small complication. |
| Plus spectator-visible annotations | Nice for streaming and teaching. Anything visible risks becoming load-bearing, and then it is machine-level live input by another name — which Q3 forbids. |
| **Plus tile marks — blueprints, paint, layers — that programs read; plus speed and resign** *(chosen)* | The player gets a way to point. Marks are load-bearing on purpose, and safe because they reach a machine only through the program that reads them; the cost is three layers of per-team tile state and the builtins to read them. |

### How the answer took its shape

The third option in the register was rejected for the reason it states:
anything visible becomes load-bearing. The user's proposal accepted that
and made it the point — a mark *should* be load-bearing, because the
player needs some way to say "here", and the only question is whether it
reaches the machine through a program or around one. Through a program is
Q3's line exactly: a blueprint the program ignores builds nothing, so the
skill being tested is still the code. Speed joined the log from Q6 and
resign from the register's second option, and the format carrying a sender
and a tick on every kind is what leaves PvP fairness a hook.
