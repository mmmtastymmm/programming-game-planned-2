//! How many bots the sim can run: a scaling benchmark, not a gate.
//!
//! Builds an open map large enough for `n` bots, deploys the starter tree
//! (`data/starter`) to every team, places `n` bots by fiat split evenly
//! across the teams, and times `Sim::step` and `Sim::state_hash` over a
//! window of ticks — what the headless driver pays per tick. With no plans
//! marked the starter bots patrol, which exercises sensing, navigation and
//! movement, the per-tick work a real match does.
//!
//! ```sh
//! cargo bench -p replay --bench scale                 # the default series
//! cargo bench -p replay --bench scale -- 50 500 5000  # chosen bot counts
//! cargo bench -p replay --bench scale -- --teams 8 1000
//! cargo bench -p replay --bench scale -- --hash-every 0 2000   # step only
//! ```
//!
//! Columns: `step ms` and `hash ms` are per call; `ticks/s` is the window's
//! wall-clock rate with hashing as configured; `bot-ticks/s` is that times
//! the bots; `vm KB` is the size of every bot's serialized VM state, what
//! the hash walks; `moved` is how many of the placed bots changed tile
//! during the window, so a run whose programs did nothing shows as such.
//!
//! Wall-clock and floats are fine here: this file is outside `src/`, and
//! nothing it computes reaches a hash. The arithmetic lint is relaxed for
//! the same reason — a bench that overflows is a bench, not a desync.
#![allow(clippy::arithmetic_side_effects)]

use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Ticks to warm up before the window, so deploys and first slices settle.
const WARMUP: u64 = 5;
/// The window: this many ticks, or `BUDGET` of wall-clock, whichever first.
const WINDOW: u64 = 200;
const BUDGET: Duration = Duration::from_secs(10);

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// An open square map with a deposit every sixth tile and `teams` starts
/// spread on a grid, each on plain ground.
fn map_text(side: i32, teams: usize) -> String {
    let half = side / 2;
    let is_deposit = |x: i32, y: i32| (x + half) % 6 == 3 && (y + half) % 6 == 3;
    let mut terrain = String::new();
    for row in 0..side {
        let y = half - 1 - row;
        for col in 0..side {
            let x = col - half;
            terrain.push(if is_deposit(x, y) { 'o' } else { '.' });
        }
        terrain.push('\n');
    }
    let cols = (teams as f64).sqrt().ceil() as i32;
    let rows = (teams as i32 + cols - 1) / cols;
    let mut starts = String::new();
    for t in 0..teams as i32 {
        let (c, r) = (t % cols, t / cols);
        // Evenly spaced, inset from the edge; nudged off a deposit tile.
        let mut x = -half + 2 + (side - 4) * (2 * c + 1) / (2 * cols);
        let y = -half + 2 + (side - 4) * (2 * r + 1) / (2 * rows);
        if is_deposit(x, y) {
            x += 1;
        }
        starts.push_str(&format!("[[team]]\nstart = [{x}, {y}]\n"));
    }
    format!(
        "name = \"scale\"\nspeed = 10\nwidth = {side}\nheight = {side}\n\
         terrain = \"\"\"\n{terrain}\"\"\"\n\n\
         [deposit_default]\ncap = 500\nregrowth = 1\namount = 500\n\n{starts}"
    )
}

struct Row {
    teams: usize,
    bots: usize,
    side: i32,
    ticks: u64,
    step: Duration,
    hash: Duration,
    hashes: u64,
    vm_bytes: usize,
    moved: usize,
}

fn bench(teams: usize, n: usize, hash_every: u64) -> Row {
    // Room to patrol: about four tiles per bot, never smaller than the first map.
    let side = ((4 * n) as f64).sqrt().ceil() as i32 + 8;
    let side = side.max(24);
    let data = Data::load().expect("data");
    let text = map_text(side, teams);
    let map = Map::parse(&text, &data).unwrap_or_else(|e| panic!("bench map: {e}"));
    let mut sim = Sim::new(map);

    // Every team runs the starter tree, deployed in the opening set.
    let tree = sim::script::read_tree(&data_dir().join("starter")).expect("starter tree");
    let mut opening = Vec::new();
    for t in 0..teams {
        for (deployment, bundle) in &tree {
            let c = Command::new(
                0,
                TeamId(t as u32),
                opening.len() as u32,
                CommandKind::Deploy {
                    deployment: deployment.clone(),
                    bundle: bundle.clone(),
                },
            );
            sim.validate(&c)
                .unwrap_or_else(|e| panic!("opening deploy: {e}"));
            opening.push(c);
        }
    }
    let report = sim.step(&opening);
    assert!(report.dropped.is_empty(), "an opening deploy was dropped");

    // `n` bots by fiat on every other free tile, dealt round-robin to the teams.
    let (lo, hi) = sim.world.map.bounds();
    let mut placed = 0usize;
    'place: for y in (lo.y..=hi.y).step_by(2) {
        for x in (lo.x..=hi.x).step_by(2) {
            if placed == n {
                break 'place;
            }
            let p = TilePos::new(x, y);
            if !sim.world.passable(p) || sim.world.occupied(p) {
                continue;
            }
            let team = TeamId((placed % teams) as u32);
            sim.world.place(Model::Bot, team, p, Some("bot".into()));
            placed += 1;
        }
    }
    assert_eq!(placed, n, "the map is too small for {n} bots");

    for _ in 0..WARMUP {
        sim.step(&[]);
    }
    let before: Vec<(sim::EntityId, TilePos)> = sim
        .world
        .machines
        .values()
        .filter(|m| m.is_bot())
        .map(|m| (m.id, m.pos))
        .collect();

    let mut step = Duration::ZERO;
    let mut hash = Duration::ZERO;
    let mut ticks = 0;
    let mut hashes = 0;
    let start = Instant::now();
    while ticks < WINDOW && start.elapsed() < BUDGET {
        let t = Instant::now();
        let report = sim.step(&[]);
        step += t.elapsed();
        ticks += 1;
        if hash_every > 0 && ticks % hash_every == 0 {
            let t = Instant::now();
            std::hint::black_box(sim.state_hash());
            hash += t.elapsed();
            hashes += 1;
        }
        assert!(report.ended.is_none(), "the match ended during the window");
    }

    // A bot that never moved ran no program worth timing; count the ones that did.
    let moved = before
        .iter()
        .filter(|(id, pos)| sim.world.machines.get(id).is_some_and(|m| m.pos != *pos))
        .count();
    let bots = sim.world.machines.values().filter(|m| m.is_bot()).count();
    let vm_bytes = sim
        .world
        .machines
        .values()
        .filter_map(|m| m.program.as_ref())
        .map(|p| p.state_bytes().len())
        .sum();
    Row {
        teams,
        bots,
        side,
        ticks,
        step,
        hash,
        hashes,
        vm_bytes,
        moved,
    }
}

fn main() {
    let mut teams = 2usize;
    let mut hash_every = 1u64;
    let mut counts: Vec<usize> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--teams" => {
                teams = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--teams <n>")
            }
            "--hash-every" => {
                hash_every = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--hash-every <ticks>")
            }
            "--bench" => {}
            s => counts.push(s.parse().unwrap_or_else(|_| panic!("not a bot count: {s}"))),
        }
    }
    if counts.is_empty() {
        counts = vec![20, 100, 500, 2000];
    }
    println!(
        "{:>5} {:>6} {:>7} {:>5} {:>10} {:>10} {:>9} {:>12} {:>8} {:>7}",
        "teams",
        "bots",
        "map",
        "ticks",
        "step ms",
        "hash ms",
        "ticks/s",
        "bot-ticks/s",
        "vm KB",
        "moved"
    );
    for n in counts {
        let r = bench(teams, n, hash_every);
        let wall = (r.step + r.hash).as_secs_f64();
        let hash_ms = if r.hashes == 0 {
            f64::NAN
        } else {
            r.hash.as_secs_f64() * 1e3 / r.hashes as f64
        };
        println!(
            "{:>5} {:>6} {:>7} {:>5} {:>10.3} {:>10.3} {:>9.1} {:>12.0} {:>8} {:>7}",
            r.teams,
            r.bots,
            format!("{}x{}", r.side, r.side),
            r.ticks,
            r.step.as_secs_f64() * 1e3 / r.ticks as f64,
            hash_ms,
            r.ticks as f64 / wall,
            r.bots as f64 * r.ticks as f64 / wall,
            r.vm_bytes / 1024,
            r.moved,
        );
    }
}
