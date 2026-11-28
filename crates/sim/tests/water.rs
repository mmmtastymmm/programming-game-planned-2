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

/// The tables, with a water table of the lab's own: these tests hold the
/// rules to account, part by part, and must not move when the shipped
/// numbers are tuned. A hundred parts to a level, and four passes.
fn lab_data() -> Data {
    let mut data = Data::load().unwrap();
    data.world.water = WaterData {
        parts: 100,
        wet: 10,
        film: 4,
        wade: 100,
        max_rate: 100,
        passes: 4,
        flood_damage: data.world.water.flood_damage,
        cushion: 50,
    };
    data
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

struct Lab {
    sim: Sim,
}

impl Lab {
    fn new(terrain: [&str; 8], heights: Option<[&str; 8]>, extra: &str) -> Lab {
        let data = lab_data();
        let map = Map::parse(&map_text(terrain, heights, extra), &data).unwrap();
        Lab {
            sim: Sim::with_data(map, data),
        }
    }

    /// The same with the flow run once a tick, for the tests that follow
    /// one pass of it part by part.
    fn one_pass(terrain: [&str; 8], heights: Option<[&str; 8]>, extra: &str) -> Lab {
        let mut data = lab_data();
        data.world.water.passes = 1;
        let map = Map::parse(&map_text(terrain, heights, extra), &data).unwrap();
        Lab {
            sim: Sim::with_data(map, data),
        }
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
        self.sim
            .world
            .set_height(idx, Height::new(level, 9).unwrap());
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
    Lab::new(OPEN, None, "ground_water = 1000")
}

/// The same, with the flow run once a tick.
fn open_lab_one_pass() -> Lab {
    Lab::one_pass(OPEN, None, "ground_water = 1000")
}

#[test]
fn a_maps_lakes_start_full_to_their_rims() {
    let data = lab_data();
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
    let data = lab_data();
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
    assert!(refused(OPEN, None, &spring("[0, 9]", "\"0.05\"")).contains("is off the map"));
    assert!(refused(OPEN, None, &spring("[0, 0]", "0")).contains("needs a `rate`"));
    assert!(refused(OPEN, None, &spring("[0, 0]", "\"1.01\"")).contains("`water.max_rate`"));
    // A rate that is no whole number of parts.
    assert!(refused(OPEN, None, &spring("[0, 0]", "\"0.005\"")).contains("needs a `rate`"));
    // Not on rock, not on a deposit, and one to a tile.
    let mut rock = OPEN;
    rock[4] = ".......#.......";
    assert!(
        refused(rock, None, &spring("[0, 0]", "\"0.05\"")).contains("a ground tile of its own")
    );
    assert!(
        refused(OPEN, None, &spring("[-6, -4]", "\"0.05\"")).contains("a ground tile of its own")
    );
    let both = format!(
        "{}[[sink]]\nat = [0, 0]\nrate = \"0.05\"\n",
        spring("[0, 0]", "\"0.05\"")
    );
    assert!(refused(OPEN, None, &both).contains("the sink at (0, 0) needs a ground tile"));
    // A start under a spring is not buildable.
    assert!(refused(OPEN, None, &spring("[-7, -4]", "\"0.05\"")).contains("is not buildable"));
    assert!(refused(OPEN, None, "ground_water = -1").contains("`ground_water`"));
    // A head is a spring's, and stands above its own tile.
    let headed =
        |key: &str, head: &str| format!("[[{key}]]\nat = [0, 0]\nrate = \"0.05\"\nhead = {head}\n");
    assert!(refused(OPEN, None, &headed("sink", "6")).contains("which only a spring has"));
    assert!(
        refused(OPEN, None, &headed("spring", "4"))
            .contains("needs a `head` in levels above its tile's height, 4")
    );
    // A decimal is written as a string, and must come to whole parts.
    assert!(refused(OPEN, None, &headed("spring", "4.5")).contains("needs a `head`"));
    assert!(refused(OPEN, None, &headed("spring", "\"4.555\"")).contains("needs a `head`"));
    let fine = map_text(OPEN, None, &headed("spring", "\"4.5\""));
    let map = Map::parse(&fine, &data).unwrap();
    assert_eq!(map.tile(at(0, 0)).unwrap().head, Some(450));
    // A sill is a sink's, and stands no lower than its own tile.
    let silled =
        |key: &str, sill: u8| format!("[[{key}]]\nat = [0, 0]\nrate = \"0.05\"\nsill = {sill}\n");
    let fine = map_text(OPEN, None, &silled("sink", 4));
    assert_eq!(
        Map::parse(&fine, &data)
            .unwrap()
            .tile(at(0, 0))
            .unwrap()
            .sill,
        Some(400)
    );
    assert!(refused(OPEN, None, &silled("spring", 6)).contains("which only a sink has"));
    assert!(
        refused(OPEN, None, &silled("sink", 3))
            .contains("needs a `sill` in levels at or above its tile's height, 4")
    );
}

#[test]
fn a_tile_offers_each_lower_neighbour_its_share() {
    // One lower neighbour, in a trench cut through rock: a fifth of the
    // difference, as for any number of them.
    let mut terrain = OPEN;
    terrain[1] = "######.########";
    terrain[2] = "#####...#######";
    terrain[3] = "######.########";
    let mut heights = ["444444444444444"; 8];
    heights[2] = "444445444444444";
    let mut lab = Lab::one_pass(terrain, Some(heights), "ground_water = 10");
    // (-1, 2) holds water; west of it is higher ground, east is level.
    lab.pour(-1, 2, 100);
    // North and south of (-1, 2) are open, so cut them off for this case.
    lab.set_height(-1, 3, 9);
    lab.set_height(-1, 1, 9);
    lab.step(1);
    assert_eq!((lab.water(-1, 2), lab.water(0, 2)), (80, 20));
    assert_eq!(lab.water(-2, 2), 0, "nothing runs uphill");

    // Four lower neighbours on open ground: a fifth each, a fifth kept.
    let mut lab = open_lab_one_pass();
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
    // is. North is met in full and east gets the four left.
    let mut lab = open_lab_one_pass();
    lab.set_height(0, 2, 5);
    lab.pour(0, 2, 30);
    let ground = lab.ground();
    lab.step(1);
    assert_eq!(lab.water(0, 2), 0);
    assert_eq!((lab.water(0, 3), lab.water(1, 2)), (26, 4));
    assert_eq!((lab.water(0, 1), lab.water(-1, 2)), (0, 0));
    // Four parts are a film. It stood through the step that brought it,
    // and dries in the next, when nothing more arrives.
    assert_eq!(lab.ground(), ground);
    lab.step(1);
    assert_eq!(lab.water(1, 2), 0);
    assert!(lab.ground() >= ground + 4, "the film east of it dried");

    // With the south a level lower still, the south is met first.
    let mut lab = open_lab_one_pass();
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
    Lab::one_pass(terrain, None, "ground_water = 1000")
}

#[test]
fn a_small_rise_on_a_flat_pool_does_not_slosh() {
    let mut lab = basin();
    for x in -1..=1 {
        for y in 0..=2 {
            lab.pour(x, y, 50);
        }
    }
    // Two parts more in the middle: a fifth of each difference rounds to
    // nothing, so it gives nothing, now or ever.
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
    let mut lab = Lab::one_pass(terrain, Some(heights), "");
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
    assert!(low > 0 && high - low <= 44, "{levels:?}");
    assert_eq!(lab.sim.world.water_total(), total);
}

#[test]
fn a_spill_on_open_ground_dries_and_one_in_a_hollow_stays() {
    let mut lab = open_lab();
    let total = lab.sim.world.water_total();
    lab.pour(0, 1, 500);
    lab.step(1500);
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
    let extra = "ground_water = 20\n[[spring]]\nat = [-6, 2]\nrate = \"0.2\"\n[[sink]]\nat = [6, 2]\nrate = \"0.2\"\n";
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
    let extra = "[[spring]]\nat = [-1, 2]\nrate = \"0.3\"\n";
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
    assert!(high - low <= 12, "{levels:?}");
    assert_eq!(lab.sim.world.water_total(), total);
}

#[test]
fn a_spring_with_a_head_stops_at_it() {
    // A trench closed at both ends, a spring in it with a head one level
    // above its bed, no sink, and far more ground water than it holds.
    let mut terrain = OPEN;
    terrain[1] = "###############";
    terrain[2] = "####.......####";
    terrain[3] = "###############";
    let extra = "ground_water = 500\n[[spring]]\nat = [0, 2]\nrate = \"0.3\"\nhead = 5\n";
    let mut lab = Lab::new(terrain, None, extra);
    let parts = lab.sim.world.data.world.water.parts;
    for tick in 1..=400 {
        lab.step(1);
        for x in -3..=3 {
            assert!(
                lab.water(x, 2) <= parts,
                "tick {tick}: ({x}, 2) stands past the head"
            );
        }
    }
    // The trench is full to the head, to within the slope the rule
    // leaves, and the spring holds: the ground water is no longer falling.
    assert_eq!(lab.water(0, 2), parts);
    assert!((-3..=3).all(|x| lab.water(x, 2) >= parts - 12));
    let ground = lab.ground();
    lab.step(50);
    assert_eq!(lab.ground(), ground);
    assert!(ground > 49000);
}

#[test]
fn a_sink_with_a_sill_leaves_the_water_at_it() {
    // A pool in a walled basin, two levels deep, with a sink whose sill
    // is a level above the bed: it takes the upper level and no more.
    let mut terrain = OPEN;
    terrain[1] = ".....#####.....";
    terrain[2] = ".....#...#.....";
    terrain[3] = ".....#...#.....";
    terrain[4] = ".....#...#.....";
    terrain[5] = ".....#####.....";
    let extra = "ground_water = 50\n[[sink]]\nat = [0, 1]\nrate = \"0.25\"\nsill = 5\n";
    let mut lab = Lab::new(terrain, None, extra);
    let parts = lab.sim.world.data.world.water.parts;
    for x in -1..=1 {
        for y in 0..=2 {
            lab.pour(x, y, 2 * parts);
        }
    }
    let total = lab.sim.world.water_total();
    lab.step(600);
    // The sink's own tile stands at its sill, and the rest within the
    // slope the rule leaves above it.
    assert_eq!(lab.water(0, 1), parts);
    for x in -1..=1 {
        for y in 0..=2 {
            let w = lab.water(x, y);
            assert!(w >= parts && w <= parts + 8, "({x}, {y}) holds {w}");
        }
    }
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
    head: Vec<Option<u32>>,
    sink: Vec<Option<u32>>,
    sill: Vec<Option<u32>>,
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
            head: w.tiles.iter().map(|t| t.head).collect(),
            sill: w.tiles.iter().map(|t| t.sill).collect(),
            sink: w.tiles.iter().map(|t| t.sink).collect(),
            ground: w.ground_water,
            order: w.map.walk().map(|(_, i)| i).collect(),
        }
    }

    // The index names one tile in every array at once.
    #[allow(clippy::needless_range_loop)]
    fn step(&mut self, wd: WaterData) {
        let n = self.water.len();
        let mut fed = vec![false; n];
        for &i in &self.order {
            if let Some(rate) = self.spring[i] {
                let surface = self.height[i] * wd.parts + self.water[i];
                let room = match self.head[i] {
                    Some(head) => u64::from(head.saturating_sub(surface)),
                    None => u64::MAX,
                };
                let up = u64::from(rate).min(self.ground).min(room);
                self.ground -= up;
                self.water[i] += up as u32;
                fed[i] = up > 0;
            }
        }
        let surface = |s: &Walk, i: usize| s.height[i] * wd.parts + s.water[i];
        for _ in 0..wd.passes {
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
                    .filter(|(_, b)| self.open[*b] && surface(self, *b) < own)
                    .map(|(dir, b)| (surface(self, b), dir, b))
                    .collect();
                lower.sort();
                let mut left = self.water[i];
                for (theirs, _, b) in lower {
                    let given = ((own - theirs) / 5).min(left);
                    gave[i] += given;
                    got[b] += given;
                    left -= given;
                }
            }
            for i in 0..n {
                self.water[i] = self.water[i] - gave[i] + got[i];
                fed[i] |= got[i] > 0;
            }
        }
        for &i in &self.order {
            if let Some(rate) = self.sink[i] {
                let surface = self.height[i] * wd.parts + self.water[i];
                let above = match self.sill[i] {
                    Some(sill) => surface.saturating_sub(sill),
                    None => u32::MAX,
                };
                let down = rate.min(self.water[i]).min(above);
                self.water[i] -= down;
                self.ground += u64::from(down);
            }
        }
        for i in 0..n {
            let thin = self.water[i] > 0 && self.water[i] <= wd.film;
            if thin && self.spring[i].is_none() && !fed[i] {
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
        let mut extra = String::from("ground_water = 30\n");
        for key in ["spring", "spring", "spring", "sink", "sink"] {
            let (x, y) = (next(15) as i32 - 7, next(8) as i32 - 3);
            extra.push_str(&format!(
                "[[{key}]]\nat = [{x}, {y}]\nrate = \"0.{:02}\"\n",
                1 + next(40)
            ));
            // Half the springs have a head, above any height this ground
            // has, and half the sinks a sill, at or above any.
            if key == "spring" && next(2) == 0 {
                extra.push_str(&format!("head = {}\n", 6 + next(4)));
            }
            if key == "sink" && next(2) == 0 {
                extra.push_str(&format!("sill = {}\n", 5 + next(3)));
            }
        }
        let data = lab_data();
        let text = map_text(terrain, Some(heights), &extra);
        // A spring that fell on rock, or on another, is a map to skip.
        let Ok(map) = Map::parse(&text, &data) else {
            continue;
        };
        rounds += 1;
        let mut sim = Sim::with_data(map, data.clone());
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
                sim.world.set_height(i, Height::new(level, 9).unwrap());
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
    let extra = "ground_water = 0\n[[spring]]\nat = [-6, -3]\nrate = \"0.07\"\n[[sink]]\nat = [-5, -3]\nrate = \"0.09\"\n";
    let mut lab = Lab::new(OPEN, None, extra);
    // The bot stands beside all three tiles: the third is a level down,
    // and from a tile further off the ground between would stand above
    // the line to it (`docs/03`, Height on the line).
    let bot = lab
        .sim
        .world
        .place(Model::Bot, TeamId(0), at(-5, -2), Some("bot".into()));
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
    assert_eq!(logs, vec!["0.07 None None 0.09 None 0.8".to_string()]);
    // The team remembers the water it saw.
    let idx = lab.idx(-4, -3);
    assert_eq!(lab.sim.world.teams[&TeamId(0)].memory[idx].water, 80);
}

#[test]
fn water_and_the_ground_water_are_in_the_hash() {
    let hash = |pour: u32, ground: u64| {
        let mut lab = Lab::new(OPEN, None, "ground_water = 10");
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

#[test]
fn every_shipped_map_loads_and_the_falls_run() {
    let data = Data::load().unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            let text = std::fs::read_to_string(&path).unwrap();
            let map = Map::parse(&text, &data)
                .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.message));
            names.push(map.name);
        }
    }
    names.sort();
    assert_eq!(names, ["falls", "first"]);

    // `falls` is a spring at the head of a bed that steps down into a
    // deep basin, with a sink there: a loop. The spring's head is the
    // height of the banks, 7, and the sink's sill a tenth of a level
    // under it, so the cut fills until its surface stands between the
    // two — level, to within that tenth, from the spring to the sink —
    // and then holds, the same tick after tick. Not a part ever stands on the plain, and not a part is
    // made or lost.
    let text = std::fs::read_to_string(dir.join("falls.toml")).unwrap();
    let mut sim = Sim::new(Map::parse(&text, &data).unwrap());
    let parts = data.world.water.parts;
    let total = sim.world.water_total();
    let surface =
        |sim: &Sim, x: i32, y: i32| sim.world.surface(sim.world.tile_index(at(x, y)).unwrap());
    let on_the_plain = |sim: &Sim| -> u64 {
        sim.world
            .tiles
            .iter()
            .filter(|t| t.height.level() == 7)
            .map(|t| u64::from(t.water))
            .sum()
    };
    for tick in 1..=2000 {
        sim.step(&[], &[]);
        assert_eq!(on_the_plain(&sim), 0, "tick {tick}: a spill");
    }
    assert_eq!(sim.world.water_total(), total);
    // Full: the whole cut stands between the sill and the head, highest
    // at the spring and lowest at the sink.
    for x in -9..=8 {
        let here = surface(&sim, x, 0);
        assert!(
            here >= 7 * parts - parts / 10 && here <= 7 * parts,
            "({x}, 0) stands at {here}"
        );
    }
    assert!(
        surface(&sim, -9, 0) > surface(&sim, 8, -1),
        "it slopes to the sink"
    );
    // Settled: a hundred ticks on, every tile holds what it held.
    let settled: Vec<u32> = sim.world.tiles.iter().map(|t| t.water).collect();
    let ground = sim.world.ground_water;
    for _ in 0..100 {
        sim.step(&[], &[]);
    }
    let later: Vec<u32> = sim.world.tiles.iter().map(|t| t.water).collect();
    assert_eq!(later, settled);
    assert_eq!(sim.world.ground_water, ground);
    assert!(
        ground > 0,
        "the spring has more to give than the loop needs"
    );
    // The lake in the south-west stands as it began.
    let lake = sim.world.tile_index(at(-8, -5)).unwrap();
    assert_eq!(sim.world.tiles[lake].water, 2 * data.world.water.parts);
}
