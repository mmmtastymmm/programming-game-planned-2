//! Height (`docs/03`, Height; Q65): a height on every tile from the map,
//! and the step a bot may take across it. T43's first part.

use sim::map::{Height, Slope};
use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos, bundle_of};

/// 7 by 5, origin at the centre: x in -3..=3, y in -2..=2. A two-step rise
/// in the north — (-1, 1) at 5 and (0, 1) at 6, on ground at 4 — and a pit
/// at (0, -1), at 2. Team 0 starts at (0, 0), under the rise; team 1 at
/// (3, -2).
fn map_text(heights: &str) -> String {
    format!(
        r#"
name = "height"
speed = 10
width = 7
height = 5
terrain = """
.......
.......
.o...o.
.......
.......
"""
{heights}
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [0, 0]
[[team]]
start = [3, -2]
"#
    )
}

const HEIGHTS: &str = r#"heights = """
4444444
4456444
4444444
4442444
4444444
""""#;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

fn sim() -> Sim {
    let data = Data::load().unwrap();
    Sim::new(Map::parse(&map_text(HEIGHTS), &data).unwrap())
}

fn deploy(main: &str) -> Command {
    Command::new(
        0,
        TeamId(0),
        0,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle_of(main),
        },
    )
}

/// A bot of team 0 placed by fiat at `from`, run for `ticks` on `main`:
/// where it ends, and what it logged.
fn walk(from: TilePos, main: &str, ticks: u64) -> (TilePos, Vec<String>) {
    let mut s = sim();
    let bot = s
        .world
        .place(Model::Bot, TeamId(0), from, Some("bot".into()));
    let mut logs = Vec::new();
    for t in 0..ticks {
        let cmds = if t == 0 { vec![deploy(main)] } else { vec![] };
        s.step(&cmds, &[]);
        logs.extend(s.world.machines[&bot].log.iter().map(|l| l.text.clone()));
    }
    (s.world.machines[&bot].pos, logs)
}

#[test]
fn a_map_gives_every_tile_its_height() {
    let s = sim();
    let h = |x, y| s.world.tile(at(x, y)).unwrap().height.level();
    assert_eq!((h(-1, 1), h(0, 1), h(0, -1), h(0, 0)), (5, 6, 2, 4));
    // The grid's first row is the north edge, as the terrain's is.
    assert_eq!((h(-3, 2), h(3, -2)), (4, 4));
}

#[test]
fn a_map_with_no_heights_stands_at_the_default_and_its_lakes_below_it() {
    let data = Data::load().unwrap();
    let text = map_text("").replace(".o...o.", "~o...o.");
    let map = Map::parse(&text, &data).unwrap();
    let hd = data.world.height;
    assert_eq!(map.tile(at(0, 0)).unwrap().height.level(), hd.default);
    assert_eq!(
        map.tile(at(-3, 0)).unwrap().height.level(),
        hd.default - hd.lake_depth
    );
}

#[test]
fn a_map_whose_heights_do_not_fit_is_refused() {
    let data = Data::load().unwrap();
    let refused = |heights: &str, data: &Data| {
        Map::parse(&map_text(heights), data)
            .expect_err("refused")
            .message
    };
    let rows = HEIGHTS.replace("4442444\n", "");
    assert!(refused(&rows, &data).contains("`heights` has 4 rows"));
    let cols = HEIGHTS.replace("4442444", "444244");
    assert!(refused(&cols, &data).contains("`heights` row 4 has 6 columns"));
    let letter = HEIGHTS.replace("4442444", "444x444");
    assert!(refused(&letter, &data).contains("`heights` holds `x`"));
    // A height above what the tables allow.
    let mut low = data.clone();
    low.world.height.max = 5;
    assert!(refused(HEIGHTS, &low).contains("the height 6 at (0, 1) is above `height.max`, 5"));
    // A start no step leaves cannot reach a deposit.
    let island = HEIGHTS.replace("4444444\n4442444", "4449444\n4442444");
    assert!(refused(&island, &data).contains("cannot reach a deposit"));
}

#[test]
fn the_slope_between_two_heights_is_one_of_five() {
    let h = |l| Height::new(l, 9).unwrap();
    assert_eq!(Height::new(10, 9), None, "no height above the most");
    assert_eq!(h(4).slope_to(h(4), 1), Slope::Level);
    assert_eq!(h(4).slope_to(h(5), 1), Slope::Up);
    assert_eq!(h(4).slope_to(h(3), 1), Slope::Down);
    assert_eq!(h(4).slope_to(h(6), 1), Slope::Cliff);
    assert_eq!(h(4).slope_to(h(2), 1), Slope::Drop(2));
    assert_eq!(h(9).slope_to(h(0), 1), Slope::Drop(9));
    // The step is the tables': with a step of two, two levels are a step.
    assert_eq!(h(4).slope_to(h(6), 2), Slope::Up);
    assert_eq!(h(4).slope_to(h(1), 2), Slope::Drop(3));
    // Only the first three are a step.
    let steps = [Slope::Level, Slope::Up, Slope::Down];
    assert!(steps.into_iter().all(Slope::is_step));
    assert!(!Slope::Cliff.is_step() && !Slope::Drop(2).is_step());
}

#[test]
fn a_bot_steps_one_level_and_no_more() {
    let s = sim();
    let w = &s.world;
    // Up the rise a level at a time, and down it the same way.
    assert!(w.steps(at(-2, 1), at(-1, 1)) && w.steps(at(-1, 1), at(0, 1)));
    assert!(w.steps(at(0, 1), at(-1, 1)) && w.steps(at(-1, 1), at(-2, 1)));
    // Its far side is two levels: a cliff from below, a drop from above.
    assert_eq!(w.slope(at(1, 1), at(0, 1)), Some(Slope::Cliff));
    assert_eq!(w.slope(at(0, 1), at(1, 1)), Some(Slope::Drop(2)));
    assert!(!w.steps(at(1, 1), at(0, 1)) && !w.steps(at(0, 1), at(1, 1)));
    // The pit is the same from the other side.
    assert!(!w.steps(at(0, 0), at(0, -1)) && !w.steps(at(0, -1), at(0, 0)));
    assert_eq!(w.slope(at(0, 0), at(9, 9)), None, "off the map");

    let try_west = "try:\n    move('w')\n    log('moved')\nexcept ValueError:\n    log('refused')\nwhile True:\n    wait(1)\n";
    // From the east, the rise is a cliff: the move is refused.
    let (pos, logs) = walk(at(1, 1), try_west, 6);
    assert_eq!((pos, logs), (at(1, 1), vec!["refused".to_string()]));
    // From the top, the same edge is a drop, which is no step either.
    let try_east = try_west.replace("'w'", "'e'");
    let (pos, logs) = walk(at(0, 1), &try_east, 6);
    assert_eq!((pos, logs), (at(0, 1), vec!["refused".to_string()]));
    // From the west it is two steps up.
    let climb = "move('e')\nmove('e')\nlog('up')\nwhile True:\n    wait(1)\n";
    let (pos, logs) = walk(at(-2, 1), climb, 8);
    assert_eq!((pos, logs), (at(0, 1), vec!["up".to_string()]));
}

#[test]
fn move_to_steps_round_a_cliff_it_cannot_climb() {
    // The nearest tile to the target is the cliff's top; of the others,
    // north and south tie and north comes first.
    let go = "move_to(-1, 1)\nwhile True:\n    wait(1)\n";
    let (pos, _) = walk(at(1, 1), go, 6);
    assert_eq!(pos, at(1, 2));
    // In the pit every tile beside it is a cliff, so there is no step.
    let stuck = "try:\n    move_to(3, 2)\nexcept ValueError:\n    log('nowhere')\nwhile True:\n    wait(1)\n";
    let (pos, logs) = walk(at(0, -1), stuck, 4);
    assert_eq!((pos, logs), (at(0, -1), vec!["nowhere".to_string()]));
}

#[test]
fn a_printed_bot_appears_on_a_tile_it_could_step_onto() {
    // Team 0's printer stands under the rise: north of it is a cliff, so
    // the bot appears east, the next of n, e, s, w.
    let mut s = sim();
    for _ in 0..31 {
        s.step(&[], &[]);
    }
    let bots: Vec<TilePos> = s
        .world
        .machines
        .values()
        .filter(|m| m.team == TeamId(0) && m.model == Model::Bot)
        .map(|m| m.pos)
        .collect();
    assert_eq!(bots, vec![at(1, 0)]);
}

#[test]
fn a_program_reads_a_tiles_height_and_the_team_remembers_it() {
    let read = "log(tile(0, 1).height, tile(1, 1).height)\nwhile True:\n    wait(1)\n";
    let (_, logs) = walk(at(1, 1), read, 3);
    assert_eq!(logs, vec!["6 4".to_string()]);
    let mut s = sim();
    s.step(&[], &[]);
    let idx = s.world.tile_index(at(0, 1)).unwrap();
    assert_eq!(s.world.teams[&TeamId(0)].memory[idx].height.level(), 6);
}

#[test]
fn a_tiles_height_is_in_the_hash() {
    let data = Data::load().unwrap();
    let hash_of = |heights: &str| {
        let mut s = Sim::new(Map::parse(&map_text(heights), &data).unwrap());
        s.step(&[], &[]);
        s.state_hash()
    };
    // One tile a level higher, in a corner no bot goes to.
    let other = HEIGHTS.replacen("4444444", "5444444", 1);
    assert_ne!(hash_of(HEIGHTS), hash_of(&other));
    assert_eq!(hash_of(HEIGHTS), hash_of(HEIGHTS));
}
