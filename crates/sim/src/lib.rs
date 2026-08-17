//! The deterministic world simulation.
//!
//! **Read CLAUDE.md's determinism rules before changing anything in this
//! crate.** Everything here must be bit-for-bit reproducible across machines,
//! processes and runs; violations surface as multiplayer desyncs, which are
//! miserable to debug and cheap to prevent. In short: no ECS, no floats, no
//! hash-order iteration, no wall clock, no OS randomness, all input as ordered
//! commands.
//!
//! `tests/no_floats.rs` and `tests/golden.rs` are the mechanical half of that
//! guarantee. They are not a substitute for the rules — a determinism bug can
//! reproduce perfectly on one machine — but they catch the common cases
//! automatically and for free.

pub mod hash;
pub mod map;
pub mod replay;
pub mod rng;
pub mod sim;
pub mod world;

pub use map::{MapSpec, TilePos};
pub use replay::{Replay, TimedCommand};
pub use sim::{Command, CommandError, Sim};
pub use world::{Entity, EntityId, World};
