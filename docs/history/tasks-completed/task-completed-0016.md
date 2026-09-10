*Closed record — see [../README.md](../README.md). Not spec.*

# T16 — The `render` crate: Bevy, the driver, the snapshot, the mark tools

`crates/render` is the fifth crate of `docs/06` and the only one allowed a
float or a Bevy dependency. Its driver is Bevy-free and tested headless:
the speed from the clock, a paused driver reaching the next command (Q32),
the peer wait and its stall report (Q12), commands agreed for `tick +
delay`. The sim is a non-send resource one system owns, touched only
through `step`, `snapshot` and `state_hash` — the review rule against any
other arrow from `render` to `sim` applies from here on.

The view is built from the completed-tick snapshot and drawn the
predecessor's way: its SVG art under `assets/art`, baked by `build.rs`
into untracked textures; textured slab tiles with autotiled water, ore
and rock blocks, rebuilt when a tile's look changes; atlas-cube bots and
printers in per-team palette swaps, faced along their movement and
interpolated between the last two snapshots in floats nothing reads back;
fog as an opaque cover over the unknown and a cold twin of every material
for the remembered (Q21); an orbit camera whose cursor ray walks the tile
heightfield. Input becomes `Deploy`, `Mark`, `Unmark`, `SetSpeed` and
`Resign` commands (Q10) the renderer submits and forgets; the panels are
tools left, inspector right, log below. `data/starter/` holds the
player's first programs, and `cargo run -p render` plays the first map
against the Fool.

CI's test job builds Bevy on Ubuntu with its build libraries and a cache
saved even on failure; the determinism jobs never build it. The peer
exchange stays under T7.

With T16 closed, milestone M4 — the renderer — is finished: every task
under it has left the file.

Completed in `2f92b25`.
