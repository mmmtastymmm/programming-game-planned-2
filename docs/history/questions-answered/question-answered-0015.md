*Closed record — see [../README.md](../README.md). Not spec.*

# Q15 — What renders the game, and on what stack?

## Ruling

*Ruling (2026-09-08):* **Bevy, in its own crate, in the sim's process,
reading only completed-tick snapshots and writing only to the command
log.** A renderer crate exists from the first playable build. It is a Bevy
app; the sim is a plain resource it owns; and the two meet at exactly two
points, both one-directional: after each tick the renderer reads a snapshot
the sim publishes, and the player's actions become commands the renderer
submits to the log. Nothing ECS-side touches sim state, which is
determinism rule 1's condition for an ECS in the process, and the review
rule CLAUDE.md states — flag any arrow from renderer to sim, every time —
is how that condition is kept.

### The rules

1. **Two crates.** `sim` stays plain Rust with no Bevy dependency and no
   float. A new `render` crate depends on `sim` and on Bevy; `sim` never
   depends on `render`. A build of `sim` alone, for CI's determinism gate,
   never compiles Bevy.
2. **The driver lives in `render`.** The loop that owns the wall clock,
   applies the speed (Q6), waits on peers (Q12) and calls the sim's next
   tick is a Bevy system. The sim is a resource that system owns; no other
   system holds a reference to it.
3. **The snapshot.** After every tick the sim publishes a **completed-tick
   snapshot**: the machines, the tiles as each team sees them (Q21), the
   sounds, the marks (Q10), the fault records, the logs. It is a plain
   value, cloned out of the sim, and the renderer's ECS is built from it —
   entities for machines and tiles are the renderer's own and are rebuilt or
   diffed from the snapshot, never shared with the sim.
4. **Interpolation** (Q6) draws between the last two snapshots by the
   fraction of the current tick that has elapsed in real time, in floats,
   which is where floats are fine. Nothing interpolated is ever read back.
5. **Input becomes commands.** A click on a tile with a mark tool is a
   `Mark`; a deploy button is a `Deploy`; the speed control a `SetSpeed`.
   The renderer submits them and forgets them; it never applies one itself,
   so what the player sees is always the sim's answer, `delay` ticks later.
6. **Headless is still a build.** `sim` plus a driver with no Bevy — the
   replay tool, the tests, the cross-architecture check (T15) — is the
   configuration CI runs, so a renderer bug cannot hide a sim bug.

### Consequences

- **The boundary is a crate boundary**, which the compiler enforces in one
  direction, and a review rule in the other. The ECS the register worried
  about exists, and it holds copies.
- **The workspace's opt-level for dependencies** already anticipated Bevy
  (`Cargo.toml`), so the dev build stays usable.
- **`docs/06` owns the driver, the snapshot's shape, and the crate
  layout**; this ruling names them and `06` pins them.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, held
  there until `docs/06` exists. [CLAUDE.md](../../../CLAUDE.md) — the crate
  layout. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T9](../../TASKS.md) — Q15 leaves its list, and `docs/06` is
  writable. [T16](../../TASKS.md) — the render crate.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Bevy — full engine, ECS, its own scheduler** *(chosen)* | Most given for free: assets, input, windowing, UI. Brings a second world model into the process, which determinism rule 1 permits — but only for as long as nothing ECS-side reaches sim state except through ordered `Command`s, which is the boundary CLAUDE.md asks reviewers to flag every time, and which then has to be defended in review forever. |
| macroquad / miniquad — immediate-mode 2D | Small, no ECS and no scheduler of its own, so drawing from sim state is a plain read. Little given for free above drawing: UI, input mapping and asset handling are all ours. |
| `wgpu` directly | No opinions imposed and no engine to fight; the crate boundary is trivially safe. The most work by a wide margin, and none of it is game design. |
| Headless for now — no renderer crate until the sim earns one | Costs nothing today and keeps the corpus honest about what is built. A sim nobody watches hides the problems only visible in motion, and the renderer's needs then arrive late, as sim changes. |

### How the answer took its shape

The user chose Bevy; the ruling's work was to make the choice safe. The
register's worry was the ECS as a second world model, and the answer is
that the second model holds *copies*: a snapshot is a value the sim hands
out, the ECS is built from it, and the sim is a resource one system owns.
The crate split makes the sim-never-depends-on-render direction a compile
error, and the review rule covers the other. Interpolation and the speed
control, both Q6's, need exactly this shape — a driver outside the sim
drawing between snapshots — so Bevy's scheduler is doing work the design
already asked for.
