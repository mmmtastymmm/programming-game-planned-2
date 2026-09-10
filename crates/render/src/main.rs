//! `cargo run -p render -- [--map FILE] [--opposition DIR]... [--programs DIR]`
//!
//! Defaults: `data/maps/first.toml`, the Fool as the opposition, and the
//! player's programs from `data/starter`. The player is team 0.

// `arithmetic_side_effects` is the workspace's guard against integer
// overflow that differs between build profiles, which is a desync in `sim`
// and `lang`. This crate is the one docs/06 allows floats in: its arithmetic
// is interpolation and layout that nothing reads back, so the lint is
// allowed here and nowhere else.
#![allow(clippy::arithmetic_side_effects)]
use render::driver::Driver;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut map = root.join("maps/first.toml");
    let mut opposition: Vec<PathBuf> = Vec::new();
    let mut programs = Some(root.join("starter"));
    while let Some(a) = args.next() {
        match a.as_str() {
            "--map" => map = PathBuf::from(args.next().expect("--map FILE")),
            "--opposition" => {
                opposition.push(PathBuf::from(args.next().expect("--opposition DIR")))
            }
            "--programs" => programs = Some(PathBuf::from(args.next().expect("--programs DIR"))),
            "--no-programs" => programs = None,
            other => {
                eprintln!("unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    if opposition.is_empty() {
        opposition.push(root.join("opposition/fool"));
    }
    let map_text = std::fs::read_to_string(&map).unwrap_or_else(|e| {
        eprintln!("{}: {e}", map.display());
        std::process::exit(1);
    });
    let (opening, deployed) = match &programs {
        Some(dir) => match render::programs::read_all(dir) {
            Ok(bundles) => (
                render::programs::deploys_for(&bundles, &Default::default()),
                bundles,
            ),
            Err(e) => {
                eprintln!("{e}");
                (Vec::new(), Default::default())
            }
        },
        None => (Vec::new(), Default::default()),
    };
    let dirs: Vec<&std::path::Path> = opposition.iter().map(PathBuf::as_path).collect();
    let driver = Driver::new(&map_text, &dirs, opening).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    render::app::run(driver, programs, deployed);
}
