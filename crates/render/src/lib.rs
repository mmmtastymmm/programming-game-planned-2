//! The renderer (`docs/06`, Q15): a Bevy app that owns the sim as a
//! resource inside one driver system, builds its own entities from the
//! completed-tick snapshot the sim publishes, and turns the player's input
//! into commands it submits to the log and forgets.
//!
//! **The one crate that may hold a float or a Bevy dependency, and the one
//! nothing depends on.** The compiler keeps `sim` free of `render`; the
//! review rule in CLAUDE.md — flag any code feeding sim state from the ECS
//! side — keeps the other direction. In this crate that rule reads: only
//! [`driver::Driver`] touches [`sim::Sim`], and only through `Sim::step`
//! with a tick's command set, `Sim::snapshot` and `Sim::state_hash`.
//!
//! [`driver`] is plain Rust with no Bevy type in it, so the loop `docs/06`
//! specifies — speed, the paused driver still reaching a command (Q32),
//! the peer wait (Q12) — is tested headless. [`app`], [`view`], [`input`]
//! and [`ui`] are the Bevy half.

// Bevy systems take one parameter per resource they touch, and a filtered
// query type is complex by construction; neither lint earns its keep here.
// `arithmetic_side_effects` is the workspace's guard against integer
// overflow that differs between build profiles, which is a desync in `sim`
// and `lang`. This crate is the one docs/06 allows floats in: its arithmetic
// is interpolation and layout that nothing reads back, so the lint is
// allowed here and nowhere else.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod app;
pub mod driver;
pub mod input;
pub mod programs;
pub mod ui;
pub mod view;
