//! `cargo run -p render -- [--map FILE] [--opposition DIR]... [--programs DIR]
//!     [--host ADDR | --join ADDR]`
//!
//! Defaults: `data/maps/first.toml`, the Fool as the opposition, and the
//! player's programs from `data/starter`. The player is team 0.
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
use render::driver::Driver;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut map = root.join("maps/first.toml");
    let mut opposition: Vec<PathBuf> = Vec::new();
    let mut programs = Some(root.join("starter"));
    let mut host: Option<String> = None;
    let mut join: Option<String> = None;
    let mut opposition_given = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--host" => host = Some(args.next().expect("--host ADDR")),
            "--join" => join = Some(args.next().expect("--join ADDR")),
            "--map" => map = PathBuf::from(args.next().expect("--map FILE")),
            "--opposition" => {
                opposition_given = true;
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
    if host.is_some() && join.is_some() {
        eprintln!("--host and --join are exclusive");
        std::process::exit(2);
    }
    // Single-player defaults to the Fool; a host seats only what it is
    // given, and a joiner seats nothing.
    if opposition.is_empty() && host.is_none() && join.is_none() {
        opposition.push(root.join("opposition/fool"));
    }
    if join.is_some() && opposition_given {
        eprintln!("a joiner seats no opposition; drop --opposition");
        std::process::exit(2);
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
    let driver = match (&host, &join) {
        (Some(addr), _) => Driver::host(&map_text, &dirs, opening, addr, |line| {
            eprintln!("{line}");
        }),
        (_, Some(addr)) => Driver::join(&map_text, opening, addr),
        _ => Driver::new(&map_text, &dirs, opening),
    }
    .unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    render::app::run(driver, programs, deployed);
}
