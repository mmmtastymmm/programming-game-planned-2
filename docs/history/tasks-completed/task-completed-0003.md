*Closed record — see [../README.md](../README.md). Not spec.*

# T3 — Determinism kit

FNV-1a state hash, named seeded SplitMix64 RNG streams, the replay artifact, a
checked-in golden fixture with `UPDATE_GOLDEN=1` regeneration, a cross-process
replay check, and a syntactic scan of `crates/sim` for floats, hash-order
iteration and wall-clock use.

The cross-process half is the part that matters: a same-process pair cannot see
an address-dependent iteration order or a per-process hasher seed.

Completed in `feaa794`.
