*Closed record — see [../README.md](../README.md). Not spec.*

# T7 — Replace the placeholder sim in `crates/sim` with the real world model, and split out `lang`, `net` and `replay`

The world model is `crates/sim`: the tables, the map with its validity
rules, tiles and deposits, machines with their actions, senses and memory
(Q21), the eight-step tick, the state hash in `docs/06`'s order, the
snapshot, and scripted teams, with the game builtins as the language's
host. `crates/net` holds the command log, its canonical bytes, and the
exchange between peers (Q12): a set per tick per peer, a hash per tick, a
handshake on the map and the delay, and TCP as a star the host relays
through. `crates/replay` is the headless driver and the golden replay —
the first map, a player against the Fool, a mid-match redeploy (Q11), a
speed change and marks — whose hash stream is the fixture CI and the
cross-architecture battery check. The renderer's driver (T16) is a peer
of the exchange: it sends its sets and hashes, stalls on a missing set,
drops a peer that leaves, and stops on the first tick two peers hash
differently.

The placeholder's `Spawn` / `SetGoal` / `Despawn` are gone with it; the
`Command` is the ordered per-tick log Q3 shapes, its principal variant a
deploy.

`check-checks.mjs --rust` extends the check on the checks to the Rust
gates, as `docs/06`'s testing table asks: each gate seeded with a defect
it must catch, from the Rust half of `scripts/ci.sh`. Its first run found
the battery's new compare mode rejecting a good hash file.

With T7 closed, milestone M0 — scaffolding — is finished, and no task is
open: every milestone the register has held has left the file.

Completed in `4717c5c`, after `2f92b25` (the renderer) and the sim, net
and replay commits before it.
