//! The deterministic world simulation (`docs/02`, `docs/03`, `docs/06`).
//!
//! **Read CLAUDE.md's determinism rules before changing anything in this
//! crate.** Everything here must be bit-for-bit reproducible across
//! machines, processes and runs: no ECS, no floats, no hash-order
//! iteration, no wall clock, all input as ordered commands.
//!
//! Layout: [`data`] the tuning tables, [`map`] the rectangle and its
//! terrain, [`world`] the state, [`sense`] line of sight and vision,
//! [`records`] what programs read, [`host`] the game builtins as the
//! language's host, [`tick`] the eight steps and the state hash,
//! [`snapshot`] what the renderer reads, [`script`] scripted teams, and
//! [`command`] the one way in.

pub mod command;
pub mod data;
pub mod hash;
pub mod host;
pub mod map;
pub mod records;
pub mod rng;
pub mod script;
pub mod sense;
pub mod snapshot;
pub mod tick;
pub mod world;

pub use command::{Bundle, Command, CommandKind, PlanKind, TeamId};
pub use data::Data;
pub use map::{Map, TilePos};
pub use snapshot::Snapshot;
pub use tick::{Sim, StepReport, bundle_of, load_bundle};
pub use world::{EntityId, World};
