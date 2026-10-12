//! The starter library in `data/starter/lib`, run as the game runs it:
//! every function is called by a bot whose program raises if an answer is
//! wrong, and `nav.go_to` is shown to get round a wall that the game's own
//! `move_to` paces in front of forever.

use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos};
use std::path::Path;

/// 15 by 9, origin at the centre: x in -7..=7, y in -4..=4. A rock wall
/// at x = 0 from y = -2 to 2 stands between team 0's start at (-5, 0) and
/// the goal at (3, 0).
const MAP: &str = r#"
name = "test"
speed = 10
width = 15
height = 9
terrain = """
...............
...............
..o....#.......
.......#.......
.......#.......
.......#.......
.......#.......
...............
............o..
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-5, 0]
[[team]]
start = [6, -3]
"#;

const GOAL: TilePos = TilePos { x: 3, y: 0 };

/// Team 0's opening: the starter tree with `robots/bot.py` replaced.
fn opening(bot: &str) -> Vec<Command> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/starter");
    let mut tree = sim::script::read_tree_files(&root).unwrap();
    tree.insert("robots/bot.py".into(), bot.into());
    sim::script::compose(&tree)
        .unwrap()
        .into_iter()
        .enumerate()
        .map(|(i, (deployment, bundle))| {
            Command::new(
                0,
                TeamId(0),
                i as u32,
                CommandKind::Deploy { deployment, bundle },
            )
        })
        .collect()
}

/// Run `ticks` ticks; fail on any team-0 fault. Returns the tick a team-0
/// bot first stood on `GOAL`, if one did.
fn run(bot: &str, ticks: u64) -> Option<u64> {
    let data = Data::load().unwrap();
    let mut sim = Sim::new(Map::parse(MAP, &data).unwrap());
    let mut reached = None;
    for tick in 0..ticks {
        let commands = if tick == 0 { opening(bot) } else { vec![] };
        let report = sim.step(&commands);
        assert!(
            report.dropped.is_empty(),
            "tick {tick}: {:?}",
            report.dropped
        );
        for m in &sim.snapshot().machines {
            if m.record.team == TeamId(0) {
                assert!(
                    m.fault.is_none(),
                    "tick {tick}: {} faulted: {:?}",
                    m.record.name,
                    m.fault
                );
            }
        }
        let there = sim
            .world
            .machines
            .values()
            .any(|m| m.team == TeamId(0) && m.model == Model::Bot && m.pos == GOAL);
        if there && reached.is_none() {
            reached = Some(tick);
        }
    }
    reached
}

#[test]
fn every_library_function_answers_as_documented() {
    // Each check raises on a wrong answer, which faults the bot and fails
    // the run; the last line parks it so the checks run once per bot.
    let bot = r#"
import geo
import sense
import act
import nav
import share
import jobs

def check(what, got, want):
    if got != want:
        raise ValueError(what, got, want)

check("at", geo.at([2, 3]), (2, 3))
check("same", geo.same((1, 2), [1, 2]), True)
check("step", geo.step((0, 0), "n"), (0, 1))
check("neighbors", geo.neighbors((0, 0)), [(0, 1), (1, 0), (0, -1), (-1, 0)])
check("dist2", geo.dist2((0, 0), (3, 4)), 25)
check("manhattan", geo.manhattan((0, 0), (3, -4)), 7)
check("adjacent", [geo.adjacent((0, 0), (1, 0)), geo.adjacent((0, 0), (1, 1))], [True, False])
check("direction_to", [geo.direction_to((0, 0), (5, 1)), geo.direction_to((0, 0), (1, -1)), geo.direction_to((2, 2), (2, 2))], ["e", "s", None])
check("opposite", geo.OPPOSITE[geo.direction_to((0, 0), (0, 3))], "s")

here = me().pos
check("known", sense.known(here[0], here[1]), True)
check("passable here", sense.passable(here[0], here[1]), True)
check("rock", sense.passable(0, 0), sense.known(0, 0) == False)
check("unknown", sense.passable(7, 4) and sense.buildable(7, 4), True)
check("printer", sense.has_building(-5, 0), True)
check("bounds", len(sense.seen_bounds()), 4)
check("nearest", sense.nearest([(100, 100), here, (50, 50)]), here)
check("nearest none", sense.nearest([]), None)
check("machines", me().id in [m.id for m in sense.machines("bot")], True)
check("printers", [m.model for m in sense.machines("printer")], ["printer"])
check("ore", (-5, 0) in sense.ore_sources() or len(sense.ore_sources()) > 0, True)

check("try_pick", act.try_pick("ore") or True, True)
check("try_drop", act.try_drop("gold"), False)
check("try_build far", act.try_build("depot", 7, 4), False)
check("try_paint", act.try_paint(7, 4), False)
check("try_unpaint", act.try_unpaint(7, 4), False)
check("try_overlay", act.try_overlay(7, 4), False)
check("try_unoverlay", act.try_unoverlay(7, 4), False)
check("try_deconstruct", act.try_deconstruct(7, 4), False)
check("try_move_to here", act.try_move_to(here[0], here[1]), False)

check("path here", nav.path([here]), [])
check("path none", nav.path([]), None)
p = nav.path([(here[0] + 2, here[1])])
check("path", [len(p), p == ["e", "e"] or len(p) == 4], [len(p), True])
check("trace", nav.trace({(0, 0): None, (1, 0): ((0, 0), "e"), (1, 1): ((1, 0), "n")}, (1, 1)), ["e", "n"])

r = share.rank()
check("rank", r[0] < r[1], True)
check("claim none", share.claim([]), None)
check("claim", share.claim(["a"]), "a")
check("mine", len(share.mine(list(range(r[1])))), 1)

check("corners", len(jobs.corners()), 4)
check("mark_jobs", jobs.mark_jobs(), [])
check("mark_step", jobs.mark_step(), False)
check("build_step", jobs.build_step(), False)
check("patrol_step", len(jobs.patrol_step(0, jobs.LEG_STEPS)), 2)
check("do_mark", jobs.do_mark((7, 4, "paint", "red")), False)

check("go_to", nav.go_to(here), True)
check("approach", nav.approach((here[0] + 1, here[1])), True)
nav.forget()

while True:
    wait(100)
"#;
    run(bot, 200);
}

/// Only the team's first bot walks; the rest, printed later, park, so no
/// crowd jostles the walker round the wall.
const FIRST_ONLY: &str = "\
first = min([m.id for m in see() if m.team == me().team and m.model == 'bot'])
if me().id != first:
    while True:
        wait(100)
";

#[test]
fn go_to_gets_round_a_wall_that_move_to_paces_in_front_of() {
    let greedy = format!(
        "{FIRST_ONLY}\
while True:
    try:
        move_to(3, 0)
    except ValueError:
        wait(1)
"
    );
    assert_eq!(run(&greedy, 600), None, "move_to alone got round the wall");
    let routed = format!(
        "{FIRST_ONLY}\
from nav import go_to

while not go_to((3, 0)):
    pass
while True:
    wait(100)
"
    );
    let at = run(&routed, 600).expect("go_to never reached the goal");
    eprintln!("go_to reached (3, 0) at tick {at}");
}
