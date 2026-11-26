//! Water (`docs/09`; Q65): parts that flow a tile a tick, springs, sinks
//! and the ground water, drying, and wading. T45's first part; flooding
//! is its second.
//!
//! The arithmetic lint is relaxed here: this file counts parts of water in
//! plain sums, and a sum that overflowed would be a failed test, not a
//! desync, since nothing here reaches the sim's state but through its API.
#![allow(clippy::arithmetic_side_effects)]

use sim::data::WaterData;
use sim::map::{Height, Terrain};
use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos, bundle_of};

/// 15 by 9, origin at the centre: x in -7..=7, y in -4..=4, the first row
/// the north edge. The teams start in the south corners with a deposit
/// beside each, so the rows above are free for water.
fn map_text(terrain: [&str; 8], heights: Option<[&str; 8]>, extra: &str) -> String {
    let heights = match heights {
        Some(h) => format!(
            "heights = \"\"\"\n{}\n444444444444444\n\"\"\"",
            h.join("\n")
        ),
        None => String::new(),
    };
    format!(
        "name = \"water\"\nspeed = 10\nwidth = 15\nheight = 9\nterrain = \"\"\"\n{}\n.o...........o.\n\"\"\"\n{heights}\n{extra}\n[deposit_default]\ncap = 50\nregrowth = 1\namount = 50\n[[team]]\nstart = [-7, -4]\n[[team]]\nstart = [7, -4]\n",
        terrain.join("\n")
    )
}

const OPEN: [&str; 8] = ["..............."; 8];

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

struct Lab {
    sim: Sim,
}

impl Lab {
    fn new(terrain: [&str; 8], heights: Option<[&str; 8]>, extra: &str) -> Lab {
        let data = Data::load().unwrap();
        let map = Map::parse(&map_text(terrain, heights, extra), &data).unwrap();
        Lab { sim: Sim::new(map) }
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        self.sim.world.tile_index(at(x, y)).unwrap()
    }

    fn water(&self, x: i32, y: i32) -> u32 {
        self.sim.world.tiles[self.idx(x, y)].water
    }

    /// Put `parts` on a tile by fiat, from the ground water's account so
    /// that the total is what it was, and say the tile is disturbed.
    fn pour(&mut self, x: i32, y: i32, parts: u32) {
        let idx = self.idx(x, y);
        let w = &mut self.sim.world;
        w.ground_water += u64::from(w.tiles[idx].water);
        w.tiles[idx].water = parts;
        w.ground_water -= u64::from(parts);
        w.stir(idx);
    }

    fn set_height(&mut self, x: i32, y: i32, level: u8) {
        let idx = self.idx(x, y);
        self.sim.world.tiles[idx].height = Height::new(level, 9).unwrap();
        self.sim.world.stir(idx);
    }

    fn step(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.sim.step(&[], &[]);
        }
    }

    fn ground(&self) -> u64 {
        self.sim.world.ground_water
    }
}

/// A lab with plenty of ground water to pour from.
fn open_lab() -> Lab {
    Lab::new(OPEN, None, "ground_water = 100000")
}

#[test]
fn a_maps_lakes_start_full_to_their_rims() {
    let data = Data::load().unwrap();
    let parts = data.world.water.parts;
    // No heights: ground at the default, a lake two levels below it.
    let mut terrain = OPEN;
    terrain[2] = "....~~.........";
    terrain[3] = "....~~.........";
    let lab = Lab::new(terrain, None, "");
    assert_eq!(lab.water(-3, 2), 2 * parts);
    assert_eq!(lab.water(-2, 1), 2 * parts);
    assert_eq!(lab.water(-1, 2), 0, "beside the lake is dry");
    assert_eq!(lab.sim.world.tiles[lab.idx(-3, 2)].terrain, Terrain::Ground);
    // With heights, the surface is the lowest open tile beside the lake,
    // and each tile holds what stands between its bed and that.
    let mut heights = ["444444444444444"; 8];
    heights[2] = "444412444444444";
    heights[3] = "444422344444444";
    let lab = Lab::new(terrain, Some(heights), "");
    // The lowest tile beside the two-by-two lake is (-1, 1), at 3.
    assert_eq!(lab.water(-3, 2), 2 * parts, "bed 1, surface 3");
    assert_eq!(lab.water(-3, 1), parts, "bed 2");
    assert_eq!(lab.water(-2, 1), parts, "bed 2");
    // A lake at rest gives nothing, and the step no longer looks at it.
    let before = lab.sim.state_hash();
    let mut lab = lab;
    lab.step(3);
    assert_eq!(lab.water(-3, 2), 2 * parts);
    assert_ne!(lab.sim.state_hash(), before, "the tick is in the hash");
    assert_eq!(lab.ground(), 0);
}

#[test]
fn a_map_whose_water_does_not_fit_is_refused() {
    let data = Data::load().unwrap();
    let refused = |terrain: [&str; 8], heights: Option<[&str; 8]>, extra: &str| {
        Map::parse(&map_text(terrain, heights, extra), &data)
            .expect_err("refused")
            .message
    };
    // A `~` tile as high as the ground beside its lake.
    let mut terrain = OPEN;
    terrain[2] = "....~~.........";
    let mut heights = ["444444444444444"; 8];
    heights[2] = "444424444444444";
    assert!(refused(terrain, Some(heights), "").contains("the `~` tile at (-2, 2) stands at 4"));
    let spring = |at: &str, rate: &str| format!("[[spring]]\nat = {at}\nrate = {rate}\n");
    assert!(refused(OPEN, None, &spring("[0, 9]", "5")).contains("is off the map"));
    assert!(refused(OPEN, None, &spring("[0, 0]", "0")).contains("needs a whole `rate`"));
    assert!(refused(OPEN, None, &spring("[0, 0]", "101")).contains("`water.max_rate`, 100"));
    // Not on rock, not on a deposit, and one to a tile.
    let mut rock = OPEN;
    rock[4] = ".......#.......";
    assert!(refused(rock, None, &spring("[0, 0]", "5")).contains("a ground tile of its own"));
    assert!(refused(OPEN, None, &spring("[-6, -4]", "5")).contains("a ground tile of its own"));
    let both = format!("{}[[sink]]\nat = [0, 0]\nrate = 5\n", spring("[0, 0]", "5"));
    assert!(refused(OPEN, None, &both).contains("the sink at (0, 0) needs a ground tile"));
    // A start under a spring is not buildable.
    assert!(refused(OPEN, None, &spring("[-7, -4]", "5")).contains("is not buildable"));
    assert!(refused(OPEN, None, "ground_water = -1").contains("`ground_water`"));
}

#[test]
fn a_tile_offers_each_lower_neighbour_its_share() {
    // One lower neighbour, in a trench cut through rock: half the
    // difference.
    let mut terrain = OPEN;
    terrain[1] = "######.########";
    terrain[2] = "#####...#######";
    terrain[3] = "######.########";
    let mut heights = ["444444444444444"; 8];
    heights[2] = "444445444444444";
    let mut lab = Lab::new(terrain, Some(heights), "ground_water = 1000");
    // (-1, 2) holds water; west of it is higher ground, east is level.
    lab.pour(-1, 2, 100);
    // North and south of (-1, 2) are open, so cut them off for this case.
    lab.set_height(-1, 3, 9);
    lab.set_height(-1, 1, 9);
    lab.step(1);
    assert_eq!((lab.water(-1, 2), lab.water(0, 2)), (50, 50));
    assert_eq!(lab.water(-2, 2), 0, "nothing runs uphill");

    // Four lower neighbours on open ground: a fifth each, a fifth kept.
    let mut lab = open_lab();
    lab.pour(0, 2, 100);
    lab.step(1);
    assert_eq!(lab.water(0, 2), 20);
    for (x, y) in [(0, 3), (1, 2), (0, 1), (-1, 2)] {
        assert_eq!(lab.water(x, y), 20, "({x}, {y})");
    }
}

#[test]
fn offers_that_outrun_a_tiles_water_are_met_lowest_first_then_n_e_s_w() {
    // Thirty parts on a tile a level above the ground round it: each
    // neighbour is offered (100 + 30) / 5 = 26, which is more than there
    // is. North is met in full, east gets the four left, and four parts
    // are a film, which dries into the ground water.
    let mut lab = open_lab();
    lab.set_height(0, 2, 5);
    lab.pour(0, 2, 30);
    let ground = lab.ground();
    lab.step(1);
    assert_eq!(lab.water(0, 2), 0);
    assert_eq!((lab.water(0, 3), lab.water(1, 2)), (26, 0));
    assert_eq!((lab.water(0, 1), lab.water(-1, 2)), (0, 0));
    assert_eq!(lab.ground(), ground + 4, "the film east of it dried");

    // With the south a level lower still, the south is met first.
    let mut lab = open_lab();
    lab.set_height(0, 2, 5);
    lab.set_height(0, 1, 3);
    lab.pour(0, 2, 30);
    lab.step(1);
    // South is offered (200 + 30) / 5 = 46, and takes all thirty.
    assert_eq!(lab.water(0, 1), 30);
    assert_eq!(
        (lab.water(0, 3), lab.water(1, 2), lab.water(-1, 2)),
        (0, 0, 0)
    );
}

/// A three-by-three basin walled by rock, every tile at the default.
fn basin() -> Lab {
    let mut terrain = OPEN;
    terrain[1] = ".....#####.....";
    terrain[2] = ".....#...#.....";
    terrain[3] = ".....#...#.....";
    terrain[4] = ".....#...#.....";
    terrain[5] = ".....#####.....";
    Lab::new(terrain, None, "ground_water = 100000")
}

#[test]
fn a_small_rise_on_a_flat_pool_does_not_slosh() {
    let mut lab = basin();
    for x in -1..=1 {
        for y in 0..=2 {
            lab.pour(x, y, 50);
        }
    }
    // Two parts more in the middle: its share of each difference, among
    // five, rounds to nothing, so it gives nothing, now or ever.
    lab.pour(0, 1, 52);
    lab.step(6);
    assert_eq!(lab.water(0, 1), 52);
    assert_eq!((lab.water(0, 2), lab.water(1, 1)), (50, 50));
    // Five more is a part each, once, and then it is level.
    lab.pour(0, 1, 55);
    lab.step(1);
    assert_eq!(lab.water(0, 1), 51);
    assert_eq!((lab.water(0, 2), lab.water(1, 1)), (51, 51));
    lab.step(5);
    assert_eq!(lab.water(0, 1), 51, "and it stays so");
}

#[test]
fn water_runs_a_tile_a_tick_down_a_channel() {
    // A lake in the west, and a trench in the rock running east from it,
    // at the lake's bed.
    let mut terrain = OPEN;
    terrain[1] = "###############";
    terrain[2] = "#~~~........###";
    terrain[3] = "###############";
    let mut heights = ["444444444444444"; 8];
    heights[2] = "422244444444444";
    let mut lab = Lab::new(terrain, Some(heights), "");
    let total = lab.sim.world.water_total();
    assert_eq!(lab.water(-6, 2), 200);
    // Cut the trench: the eight tiles east of the lake, down to its bed.
    for x in -3..=4 {
        lab.set_height(x, 2, 2);
    }
    for tick in 1..=8 {
        lab.step(1);
        // The front is at most `tick` tiles in: the tile beyond is dry.
        let beyond = -4 + tick + 1;
        if beyond <= 4 {
            assert_eq!(lab.water(beyond, 2), 0, "tick {tick}: ({beyond}, 2)");
        }
        assert!(lab.water(-3, 2) > 0, "tick {tick}: the first tile is wet");
    }
    lab.step(400);
    // It ends level, to within the slope the rule leaves, and nothing was
    // made: what is not in the trench dried on the way and is ground water.
    let levels: Vec<u32> = (-6..=4).map(|x| lab.water(x, 2)).collect();
    let (low, high) = (*levels.iter().min().unwrap(), *levels.iter().max().unwrap());
    assert!(low > 0 && high - low <= 11, "{levels:?}");
    assert_eq!(lab.sim.world.water_total(), total);
}

#[test]
fn a_spill_on_open_ground_dries_and_one_in_a_hollow_stays() {
    let mut lab = open_lab();
    let total = lab.sim.world.water_total();
    lab.pour(0, 1, 500);
    lab.step(200);
    let standing: u64 = lab.sim.world.tiles.iter().map(|t| u64::from(t.water)).sum();
    assert_eq!(standing, 0, "all of it dried");
    assert_eq!(lab.ground(), total, "into the ground water");
    assert_eq!(lab.sim.world.water_total(), total);

    // A pit a level deep holds what is put in it.
    let mut lab = open_lab();
    lab.set_height(0, 1, 3);
    lab.pour(0, 1, 60);
    lab.step(60);
    assert_eq!(lab.water(0, 1), 60);
}

#[test]
fn a_spring_and_a_sink_make_a_river_and_the_total_never_changes() {
    // A trench with a spring at its west end and a sink at its east.
    let mut terrain = OPEN;
    terrain[1] = "###############";
    terrain[2] = "#.............#";
    terrain[3] = "###############";
    let extra = "ground_water = 2000\n[[spring]]\nat = [-6, 2]\nrate = 20\n[[sink]]\nat = [6, 2]\nrate = 20\n";
    let mut lab = Lab::new(terrain, None, extra);
    let total = lab.sim.world.water_total();
    assert_eq!(total, 2000);
    for tick in 1..=600 {
        lab.step(1);
        assert_eq!(lab.sim.world.water_total(), total, "tick {tick}");
    }
    // The water reached the far end, and the sink is still handing the
    // spring something to bring up: a river, not a pond.
    assert!(lab.water(-6, 2) > 0 && lab.water(0, 2) > 0 && lab.water(5, 2) > 0);
    assert!(lab.ground() > 0);
}

#[test]
fn a_spring_brings_back_a_lake_that_was_emptied() {
    // A lake in a pit with a spring on its bed, and no ground water.
    let mut terrain = OPEN;
    terrain[2] = ".....~~~.......";
    terrain[3] = ".....~~~.......";
    let extra = "[[spring]]\nat = [-1, 2]\nrate = 30\n";
    let mut lab = Lab::new(terrain, None, extra);
    let total = lab.sim.world.water_total();
    assert_eq!(total, 6 * 200);
    lab.step(5);
    assert_eq!(
        lab.ground(),
        0,
        "a full lake gives the spring nothing to do"
    );
    // Empty it by fiat into the ground water, as a pump one day will.
    for x in -2..=0 {
        for y in 1..=2 {
            lab.pour(x, y, 0);
        }
    }
    assert_eq!(lab.ground(), total);
    lab.step(400);
    assert_eq!(lab.ground(), 0, "the spring brought it all back");
    let levels: Vec<u32> = [(-2, 2), (-1, 2), (0, 2), (-2, 1), (-1, 1), (0, 1)]
        .into_iter()
        .map(|(x, y)| lab.water(x, y))
        .collect();
    let (low, high) = (*levels.iter().min().unwrap(), *levels.iter().max().unwrap());
    assert!(high - low <= 6, "{levels:?}");
    assert_eq!(lab.sim.world.water_total(), total);
}

#[test]
fn a_tile_raised_under_water_sheds_it() {
    let mut lab = basin();
    for x in -1..=1 {
        for y in 0..=2 {
            lab.pour(x, y, 50);
        }
    }
    let total = lab.sim.world.water_total();
    // Fill the middle in a level: its water now stands a level above the
    // rest and runs off.
    lab.set_height(0, 1, 5);
    lab.step(40);
    assert_eq!(lab.water(0, 1), 0);
    assert!(lab.water(0, 2) > 50 && lab.water(1, 1) > 50);
    assert_eq!(lab.sim.world.water_total(), total);
}

/// The water step as `docs/09` states it, by a walk of every tile: what
/// the sim's step, which looks only at tiles that may have changed, must
/// leave.
struct Walk {
    width: usize,
    open: Vec<bool>,
    height: Vec<u32>,
    water: Vec<u32>,
    spring: Vec<Option<u32>>,
    sink: Vec<Option<u32>>,
    ground: u64,
    /// Tile indices in row-then-column order.
    order: Vec<usize>,
}

impl Walk {
    fn of(sim: &Sim) -> Walk {
        let w = &sim.world;
        Walk {
            width: w.map.width as usize,
            open: w
                .tiles
                .iter()
                .map(|t| t.terrain == Terrain::Ground)
                .collect(),
            height: w
                .tiles
                .iter()
                .map(|t| u32::from(t.height.level()))
                .collect(),
            water: w.tiles.iter().map(|t| t.water).collect(),
            spring: w.tiles.iter().map(|t| t.spring).collect(),
            sink: w.tiles.iter().map(|t| t.sink).collect(),
            ground: w.ground_water,
            order: w.map.walk().map(|(_, i)| i).collect(),
        }
    }

    // The index names one tile in every array at once.
    #[allow(clippy::needless_range_loop)]
    fn step(&mut self, wd: WaterData) {
        let n = self.water.len();
        for &i in &self.order {
            if let Some(rate) = self.spring[i] {
                let up = u64::from(rate).min(self.ground);
                self.ground -= up;
                self.water[i] += up as u32;
            }
        }
        let surface = |s: &Walk, i: usize| s.height[i] * wd.parts + s.water[i];
        let mut gave = vec![0u32; n];
        let mut got = vec![0u32; n];
        for i in 0..n {
            if !self.open[i] || self.water[i] == 0 {
                continue;
            }
            let col = i % self.width;
            let beside = [
                (i + self.width < n).then(|| i + self.width),
                (col + 1 < self.width).then(|| i + 1),
                i.checked_sub(self.width),
                (col > 0).then(|| i - 1),
            ];
            let own = surface(self, i);
            let mut lower: Vec<(u32, usize, usize)> = beside
                .into_iter()
                .enumerate()
                .filter_map(|(dir, b)| b.map(|b| (dir, b)))
                .filter(|(_, b)| self.open[*b] && surface(self, *b) + 2 <= own)
                .map(|(dir, b)| (surface(self, b), dir, b))
                .collect();
            let sharing = lower.len() as u32 + 1;
            lower.sort();
            let mut left = self.water[i];
            for (theirs, _, b) in lower {
                let given = ((own - theirs) / sharing).min(left);
                gave[i] += given;
                got[b] += given;
                left -= given;
            }
        }
        for i in 0..n {
            self.water[i] = self.water[i] - gave[i] + got[i];
        }
        for &i in &self.order {
            if let Some(rate) = self.sink[i] {
                let down = rate.min(self.water[i]);
                self.water[i] -= down;
                self.ground += u64::from(down);
            }
        }
        for i in 0..n {
            if self.water[i] > 0 && self.water[i] <= wd.film && self.spring[i].is_none() {
                self.ground += u64::from(self.water[i]);
                self.water[i] = 0;
            }
        }
    }
}

#[test]
fn the_step_leaves_what_a_walk_of_every_tile_leaves() {
    // Random ground, lakes, springs and sinks, and random changes to the
    // ground and the water as it runs.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move |below: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % below
    };
    let mut rounds = 0;
    for round in 0..16 {
        let mut heights_owned: Vec<String> = Vec::new();
        for _ in 0..8 {
            heights_owned.push((0..15).map(|_| char::from(b'2' + next(4) as u8)).collect());
        }
        let mut terrain_owned: Vec<String> = Vec::new();
        for _ in 0..8 {
            terrain_owned.push(
                (0..15)
                    .map(|_| if next(9) == 0 { '#' } else { '.' })
                    .collect(),
            );
        }
        let terrain: [&str; 8] = std::array::from_fn(|i| terrain_owned[i].as_str());
        let heights: [&str; 8] = std::array::from_fn(|i| heights_owned[i].as_str());
        // Springs and sinks at random tiles of the rows above the teams'.
        let mut extra = String::from("ground_water = 3000\n");
        for key in ["spring", "spring", "spring", "sink", "sink"] {
            let (x, y) = (next(15) as i32 - 7, next(8) as i32 - 3);
            extra.push_str(&format!(
                "[[{key}]]\nat = [{x}, {y}]\nrate = {}\n",
                1 + next(40)
            ));
        }
        let data = Data::load().unwrap();
        let text = map_text(terrain, Some(heights), &extra);
        // A spring that fell on rock, or on another, is a map to skip.
        let Ok(map) = Map::parse(&text, &data) else {
            continue;
        };
        rounds += 1;
        let mut sim = Sim::new(map);
        let n = sim.world.tiles.len();
        let total = sim.world.water_total();
        let mut walk = Walk::of(&sim);
        for tick in 0..150u64 {
            // Now and then: pour some water, or raise or lower a tile.
            if next(4) == 0 {
                let i = next(n as u64) as usize;
                if sim.world.tiles[i].terrain == Terrain::Ground {
                    let parts = next(300) as u32;
                    let take = u64::from(parts).min(sim.world.ground_water) as u32;
                    sim.world.tiles[i].water += take;
                    sim.world.ground_water -= u64::from(take);
                    sim.world.stir(i);
                    walk.water[i] += take;
                    walk.ground -= u64::from(take);
                }
            }
            if next(6) == 0 {
                let i = next(n as u64) as usize;
                let level = 1 + next(6) as u8;
                sim.world.tiles[i].height = Height::new(level, 9).unwrap();
                sim.world.stir(i);
                walk.height[i] = u32::from(level);
            }
            sim.step(&[], &[]);
            walk.step(data.world.water);
            let now: Vec<u32> = sim.world.tiles.iter().map(|t| t.water).collect();
            assert_eq!(now, walk.water, "round {round}, tick {tick}");
            assert_eq!(
                sim.world.ground_water, walk.ground,
                "round {round}, tick {tick}"
            );
            assert_eq!(sim.world.water_total(), total, "round {round}, tick {tick}");
        }
    }
    assert!(rounds >= 4, "only {rounds} random maps were valid");
}

#[test]
fn a_bot_wades_a_level_of_water_and_no_more() {
    let mut lab = open_lab();
    let wade = lab.sim.world.data.world.water.wade;
    lab.set_height(2, 2, 3);
    lab.pour(2, 2, wade);
    assert!(lab.sim.world.steps(at(2, 3), at(2, 2)), "a level is waded");
    lab.pour(2, 2, wade + 1);
    assert!(!lab.sim.world.steps(at(2, 3), at(2, 2)), "more is not");
    // And a wet tile is not buildable, where a damp one is.
    let wet = lab.sim.world.data.world.water.wet;
    lab.pour(2, 2, wet);
    assert!(!lab.sim.world.buildable(at(2, 2)));
    lab.pour(2, 2, wet - 1);
    assert!(lab.sim.world.buildable(at(2, 2)));
}

#[test]
fn a_program_reads_a_tiles_water_spring_and_sink() {
    let extra = "ground_water = 0\n[[spring]]\nat = [-6, -3]\nrate = 7\n[[sink]]\nat = [-5, -3]\nrate = 9\n";
    let mut lab = Lab::new(OPEN, None, extra);
    let bot = lab
        .sim
        .world
        .place(Model::Bot, TeamId(0), at(-6, -2), Some("bot".into()));
    lab.set_height(-4, -3, 3);
    lab.sim.world.ground_water = 80;
    lab.pour(-4, -3, 80);
    let read = "a = tile(-6, -3)\nb = tile(-5, -3)\nc = tile(-4, -3)\nlog(a.spring, a.sink, b.spring, b.sink, c.spring, c.water)\nwhile True:\n    wait(1)\n";
    let deploy = Command::new(
        0,
        TeamId(0),
        0,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle_of(read),
        },
    );
    let mut logs = Vec::new();
    for t in 0..3 {
        let cmds = if t == 0 { vec![deploy.clone()] } else { vec![] };
        lab.sim.step(&cmds, &[]);
        logs.extend(
            lab.sim.world.machines[&bot]
                .log
                .iter()
                .map(|l| l.text.clone()),
        );
    }
    assert_eq!(logs, vec!["7 None None 9 None 80".to_string()]);
    // The team remembers the water it saw.
    let idx = lab.idx(-4, -3);
    assert_eq!(lab.sim.world.teams[&TeamId(0)].memory[idx].water, 80);
}

#[test]
fn water_and_the_ground_water_are_in_the_hash() {
    let hash = |pour: u32, ground: u64| {
        let mut lab = Lab::new(OPEN, None, "ground_water = 1000");
        lab.set_height(0, 1, 3);
        lab.sim.world.ground_water = ground + u64::from(pour);
        lab.pour(0, 1, pour);
        lab.step(1);
        lab.sim.state_hash()
    };
    assert_eq!(hash(50, 1000), hash(50, 1000));
    assert_ne!(hash(50, 1000), hash(51, 1000), "a part on a tile");
    assert_ne!(hash(50, 1000), hash(50, 1001), "a part in the ground");
}
