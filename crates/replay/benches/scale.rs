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
//! cargo bench -p replay --bench scale -- --side 98 --program wait 500
//! cargo bench -p replay --bench scale -- --water 500   # a river and a lake
//! ```
//!
//! `--side` fixes the map's side instead of sizing it to the bots, and
//! `--program wait` deploys a bot that only waits, so the sim's own
//! per-machine work can be told from the interpreter's. `--water` cuts a
//! river along the map's north edge, a spring at its west end and a sink
//! at its east, and a lake at rest south of its middle, and adds a column
//! for what the water step costs (`docs/09`).
//!
//! Columns: `step ms` and `hash ms` are per call; `ticks/s` is the window's
//! wall-clock rate with hashing as configured; `bot-ticks/s` is that times
//! the bots; `vm KB` is the size of every bot's serialized VM state, what
//! the hash walks; `moved` is how many of the placed bots changed tile
//! during the window, so a run whose programs did nothing shows as such.
//! `water ms` is one water step, timed alone on a copy of the world that
//! runs nothing else, so the river it times is the one the match has; and
//! `wet` is how many tiles held water at the window's end.
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

/// The river of a `--water` map: the second row from the north, a bed a
/// level below the ground from one tile inside the west edge to one inside
/// the east.
const RIVER_ROW: i32 = 1;
/// The lake south of the river's middle, a row of ground between them:
/// two rows, this many tiles wide, and at rest.
const LAKE_WIDTH: i32 = 8;

/// What a tile of a `--water` map is, by its row and column from the
/// north-west: the river's bed, the lake, or plain ground.
#[derive(PartialEq)]
enum Wet {
    No,
    River,
    Lake,
}

fn wet_at(side: i32, row: i32, col: i32) -> Wet {
    let lake = (side - LAKE_WIDTH) / 2..(side + LAKE_WIDTH) / 2;
    if row == RIVER_ROW && col >= 1 && col < side - 1 {
        Wet::River
    } else if (row == RIVER_ROW + 2 || row == RIVER_ROW + 3) && lake.contains(&col) {
        Wet::Lake
    } else {
        Wet::No
    }
}

/// An open square map with a deposit every sixth tile and `teams` starts
/// spread on a grid, each on plain ground. With `water`, a river and a
/// lake as [`wet_at`] places them, the river a loop: its spring brings up
/// what its sink takes down, to a head below the banks.
fn map_text(side: i32, teams: usize, water: bool, data: &Data) -> String {
    let half = side / 2;
    let is_deposit = |x: i32, y: i32| (x + half) % 6 == 3 && (y + half) % 6 == 3;
    let ground = data.world.height.default;
    let mut terrain = String::new();
    let mut heights = String::new();
    for row in 0..side {
        let y = half - 1 - row;
        for col in 0..side {
            let x = col - half;
            let wet = if water {
                wet_at(side, row, col)
            } else {
                Wet::No
            };
            terrain.push(match wet {
                Wet::Lake => '~',
                Wet::River => '.',
                Wet::No if is_deposit(x, y) => 'o',
                Wet::No => '.',
            });
            let level = match wet {
                Wet::No => ground,
                Wet::River => ground - 1,
                Wet::Lake => ground - 2,
            };
            heights.push(char::from(b'0' + level));
        }
        terrain.push('\n');
        heights.push('\n');
    }
    // The river's own keys: its bed's height grid, a spring whose head is
    // nine tenths of the way up the bank and a sink whose sill is half way,
    // and ground water enough to fill the bed to the head.
    let river = if water {
        let y = half - 1 - RIVER_ROW;
        let (west, east) = (-half + 1, -half + side - 2);
        let bed = ground - 1;
        format!(
            "heights = \"\"\"\n{heights}\"\"\"\nground_water = {}\n\
             [[spring]]\nat = [{west}, {y}]\nrate = 1\nhead = \"{bed}.9\"\n\
             [[sink]]\nat = [{east}, {y}]\nrate = 1\nsill = \"{bed}.5\"\n\n",
            side
        )
    } else {
        String::new()
    };
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
         terrain = \"\"\"\n{terrain}\"\"\"\n{river}\n\
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
    /// The water steps timed alone, with how many, and the wet tiles at
    /// the end; nothing on a dry map.
    water: Option<(Duration, u64, usize)>,
}

fn bench(
    teams: usize,
    n: usize,
    hash_every: u64,
    side: Option<i32>,
    program: &str,
    water: bool,
) -> Row {
    // Room to patrol: about four tiles per bot, never smaller than the first map.
    let side_for = ((4 * n) as f64).sqrt().ceil() as i32 + 8;
    let side = side.map_or(side_for.max(24), |s| s);
    // An even side with water, so that the rows and columns the river is
    // written in are the ones the map reads it in.
    let side = if water { side + side % 2 } else { side };
    let data = Data::load().expect("data");
    let text = map_text(side, teams, water, &data);
    let map = Map::parse(&text, &data).unwrap_or_else(|e| panic!("bench map: {e}"));
    let mut sim = Sim::new(map);

    // Every team runs the starter tree, deployed in the opening set.
    let mut tree = sim::script::read_tree(&data_dir().join("starter")).expect("starter tree");
    if program == "wait" {
        tree.insert("bot".into(), sim::bundle_of("while True:\n    wait(1)\n"));
    }
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
    let report = sim.step(&opening, &[]);
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
            // Off the river and the lake: a bot is placed on dry ground.
            let default = sim.world.data.world.height.default;
            if sim
                .world
                .tile(p)
                .is_some_and(|t| t.height.level() != default)
            {
                continue;
            }
            let team = TeamId((placed % teams) as u32);
            sim.world.place(Model::Bot, team, p, Some("bot".into()));
            placed += 1;
        }
    }
    assert_eq!(placed, n, "the map is too small for {n} bots");

    for _ in 0..WARMUP {
        sim.step(&[], &[]);
    }
    let before: Vec<(sim::EntityId, TilePos)> = sim
        .world
        .machines
        .values()
        .filter(|m| m.is_bot())
        .map(|m| (m.id, m.pos))
        .collect();

    // The water step alone, on a copy that runs nothing else: the same
    // river from the same state, a step for each tick of the window.
    let mut still = water.then(|| sim.world.copy());
    let mut water_time = Duration::ZERO;

    let mut step = Duration::ZERO;
    let mut hash = Duration::ZERO;
    let mut ticks = 0;
    let mut hashes = 0;
    let start = Instant::now();
    while ticks < WINDOW && start.elapsed() < BUDGET {
        let t = Instant::now();
        let report = sim.step(&[], &[]);
        step += t.elapsed();
        ticks += 1;
        if let Some(w) = still.as_mut() {
            let t = Instant::now();
            w.water_step();
            water_time += t.elapsed();
        }
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
    let wet = sim.world.tiles.iter().filter(|t| t.water > 0).count();
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
        water: water.then_some((water_time, ticks, wet)),
    }
}

fn main() {
    let mut teams = 2usize;
    let mut hash_every = 1u64;
    let mut side: Option<i32> = None;
    let mut program = String::from("starter");
    let mut water = false;
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
            "--side" => {
                side = Some(
                    args.next()
                        .and_then(|v| v.parse().ok())
                        .expect("--side <n>"),
                )
            }
            "--program" => program = args.next().expect("--program starter|wait"),
            "--water" => water = true,
            "--bench" => {}
            s => counts.push(s.parse().unwrap_or_else(|_| panic!("not a bot count: {s}"))),
        }
    }
    if counts.is_empty() {
        counts = vec![20, 100, 500, 2000];
    }
    println!(
        "{:>5} {:>6} {:>7} {:>5} {:>10} {:>10} {:>9} {:>12} {:>8} {:>7} {:>9} {:>6}",
        "teams",
        "bots",
        "map",
        "ticks",
        "step ms",
        "hash ms",
        "ticks/s",
        "bot-ticks/s",
        "vm KB",
        "moved",
        "water ms",
        "wet"
    );
    for n in counts {
        let r = bench(teams, n, hash_every, side, &program, water);
        let (water_ms, wet) = match r.water {
            Some((time, steps, wet)) if steps > 0 => (
                format!("{:.4}", time.as_secs_f64() * 1e3 / steps as f64),
                wet.to_string(),
            ),
            _ => ("-".to_string(), "-".to_string()),
        };
        let wall = (r.step + r.hash).as_secs_f64();
        let hash_ms = if r.hashes == 0 {
            f64::NAN
        } else {
            r.hash.as_secs_f64() * 1e3 / r.hashes as f64
        };
        println!(
            "{:>5} {:>6} {:>7} {:>5} {:>10.3} {:>10.3} {:>9.1} {:>12.0} {:>8} {:>7} {:>9} {:>6}",
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
            water_ms,
            wet,
        );
    }
}
