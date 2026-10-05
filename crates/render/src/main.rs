//! `cargo run -p render -- [--play] [--map FILE] [--opposition DIR]...
//!     [--programs DIR | --no-programs] [--host ADDR | --join ADDR]`
//!
//! With no argument the start screen (Q36) picks the match; any flag skips
//! it and starts at once, and `--play` alone starts the defaults:
//! `data/maps/first.toml`, the Fool as the opposition, and the player's
//! programs from `data/starter`. The player is team 0.
//!
//! A match between peers (`docs/06`, Q12): one player hosts with
//! `--host 0.0.0.0:7777` and seats the scripted opposition, if any, on
//! the teams after its own; the host waits for a peer per remaining team.
//! The others `--join HOST:7777` with the same map and take the team the
//! host assigns. A joiner passes no `--opposition`.

// `arithmetic_side_effects` is the workspace's guard against integer
// overflow that differs between build profiles, which is a desync in `sim`
// and `lang`. This crate is the one docs/06 allows floats in: its arithmetic
// is interpolation and layout that nothing reads back, so the lint is
// allowed here and nowhere else.
#![allow(clippy::arithmetic_side_effects)]
use render::launch::{Setup, data_root};

fn main() {
    let (setup, skip) =
        Setup::from_args(std::env::args().skip(1), &data_root()).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
    render::app::run(setup, skip);
}
