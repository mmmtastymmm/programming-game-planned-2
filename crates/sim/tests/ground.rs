//! Changing the ground (`docs/03`; Q65): `dig`, `fill` and `mine`, earth,
//! and the ground plan. A part of T43.

use sim::map::{Height, Terrain};
use sim::world::{Model, Plan};
use sim::{Command, CommandKind, Data, Map, PlanKind, Sim, TeamId, TilePos, bundle_of};

/// 7 by 5, origin at the centre: x in -3..=3, y in -2..=2, every tile at
/// the default height, 4. A rock at (0, 2), a deposit at (1, 1), and a
/// lake two tiles long in the west, at (-3, 1) and (-3, 0). Team 0 starts
/// at (0, 0) and team 1 at (3, -2).
const MAP: &str = r#"
name = "ground"
speed = 10
width = 7
height = 5
terrain = """
...#...
~...o..
~......
.......
.o...o.
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [0, 0]
[[team]]
start = [3, -2]
"#;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

fn sim() -> Sim {
    let data = Data::load().unwrap();
    Sim::new(Map::parse(MAP, &data).unwrap())
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

/// Run `sim` for `ticks` with `main` deployed to team 0's bots on the
/// first, and `more` applied on the second, once the team has seen the
/// map and a mark can stand: what the bot `id` logged.
fn run(
    sim: &mut Sim,
    id: sim::world::EntityId,
    main: &str,
    more: Vec<Command>,
    ticks: u64,
) -> Vec<String> {
    let mut logs = Vec::new();
    for t in 0..ticks {
        let mut cmds = match t {
            0 => vec![deploy(main)],
            1 => more.clone(),
            _ => vec![],
        };
        for (i, c) in cmds.iter_mut().enumerate() {
            c.seq = i as u32;
            c.tick = sim.world.tick.saturating_add(1);
        }
        sim.step(&cmds, &[]);
        logs.extend(sim.world.machines[&id].log.iter().map(|l| l.text.clone()));
    }
    logs
}

/// A bot of team 0 placed by fiat.
fn bot(sim: &mut Sim, x: i32, y: i32) -> sim::world::EntityId {
    sim.world
        .place(Model::Bot, TeamId(0), at(x, y), Some("bot".into()))
}

/// `call` in a `try`, logging `name ok` or `name refused`.
fn attempt(name: &str, call: &str) -> String {
    format!(
        "try:\n    {call}\n    log('{name} ok')\nexcept ValueError:\n    log('{name} refused')\n"
    )
}

const IDLE: &str = "while True:\n    wait(1)\n";

fn level(sim: &Sim, x: i32, y: i32) -> u8 {
    sim.world.tile(at(x, y)).unwrap().height.level()
}

#[test]
fn a_bot_digs_a_tile_down_a_level_at_a_time_and_carries_the_earth() {
    let mut s = sim();
    let id = bot(&mut s, -1, -1);
    let main = format!(
        "dig(-2, -1)\nlog(tile(-2, -1).height, me().load['earth'])\ndig(-2, -1)\ndig(-2, -1)\ndig(-2, -1)\nlog(tile(-2, -1).height, me().load['earth'])\n{}{}log(tile(-2, -1).height, me().load['earth'])\n{IDLE}",
        // The fifth: the tile is at the lowest level, and the load full.
        attempt("fifth", "dig(-2, -1)"),
        attempt("fill", "fill(-2, -1)"),
    );
    let logs = run(&mut s, id, &main, vec![], 40);
    assert_eq!(logs, ["3 1", "0 4", "fifth refused", "fill ok", "1 3"]);
    assert_eq!(level(&s, -2, -1), 1);
    // A full load refuses a dig even where the ground could go lower.
    let cap = s.world.data.model("bot").capacity["earth"];
    assert_eq!(cap, lang::Num::from_int(4).unwrap());
    let mut s = sim();
    let id = bot(&mut s, -1, -1);
    s.world
        .machines
        .get_mut(&id)
        .unwrap()
        .load
        .insert("earth".into(), cap);
    let logs = run(
        &mut s,
        id,
        &format!("{}{IDLE}", attempt("full", "dig(-2, -1)")),
        vec![],
        8,
    );
    assert_eq!(logs, ["full refused"]);
    assert_eq!(level(&s, -2, -1), 4);
}

#[test]
fn what_the_ground_actions_refuse() {
    let mut s = sim();
    // North of the bot is the rock, east the deposit, south team 0's
    // printer, west open ground.
    let id = bot(&mut s, 0, 1);
    let main = [
        attempt("far", "dig(-2, 1)"),
        attempt("off the map", "dig(0, 9)"),
        attempt("rock", "dig(0, 2)"),
        attempt("deposit", "dig(1, 1)"),
        attempt("building", "dig(0, 0)"),
        attempt("no earth", "fill(-1, 1)"),
        attempt("mine ground", "mine(-1, 1)"),
        attempt("mine", "mine(0, 2)"),
        "log(tile(0, 2).terrain, tile(0, 2).height)\n".to_string(),
        attempt("dig what was rock", "dig(0, 2)"),
        IDLE.to_string(),
    ]
    .concat();
    let logs = run(&mut s, id, &main, vec![], 30);
    assert_eq!(
        logs,
        [
            "far refused",
            "off the map refused",
            "rock refused",
            "deposit refused",
            "building refused",
            "no earth refused",
            "mine ground refused",
            "mine ok",
            "ground 4",
            "dig what was rock ok",
        ]
    );
    assert_eq!(s.world.tile(at(0, 2)).unwrap().terrain, Terrain::Ground);
    assert_eq!(level(&s, 0, 2), 3);

    // A tile with a machine on it, and the top level, and the bottom.
    let mut s = sim();
    let id = bot(&mut s, -1, -1);
    bot(&mut s, -2, -1);
    let idx = s.world.tile_index(at(-1, -2)).unwrap();
    s.world.tiles[idx].height = Height::new(9, 9).unwrap();
    let idx = s.world.tile_index(at(-1, 0)).unwrap();
    s.world.tiles[idx].height = Height::new(0, 9).unwrap();
    let earth = lang::Num::from_int(2).unwrap();
    s.world
        .machines
        .get_mut(&id)
        .unwrap()
        .load
        .insert("earth".into(), earth);
    let main = [
        attempt("on a bot", "dig(-2, -1)"),
        attempt("fill a bot", "fill(-2, -1)"),
        attempt("the top", "fill(-1, -2)"),
        attempt("the bottom", "dig(-1, 0)"),
        IDLE.to_string(),
    ]
    .concat();
    let logs = run(&mut s, id, &main, vec![], 10);
    assert_eq!(
        logs,
        [
            "on a bot refused",
            "fill a bot refused",
            "the top refused",
            "the bottom refused"
        ]
    );
}

#[test]
fn a_dig_that_a_machine_walks_onto_does_nothing() {
    // The tile was clear when the dig began; by the time it completes a
    // bot stands on it, so nothing is dug and no earth is taken.
    let mut s = sim();
    let digger = bot(&mut s, -1, -1);
    let walker = bot(&mut s, -2, -2);
    let main = "if me().id == DIGGER:\n    dig(-2, -1)\n    log(tile(-2, -1).height, me().load['earth'])\nelse:\n    move('n')\nwhile True:\n    wait(1)\n"
        .replace("DIGGER", &digger.0.to_string());
    let logs = run(&mut s, digger, &main, vec![], 12);
    assert_eq!(s.world.machines[&walker].pos, at(-2, -1));
    assert_eq!(logs, ["4 0"]);
}

#[test]
fn water_runs_into_ground_dug_beside_a_lake() {
    let mut s = sim();
    let parts = s.world.data.world.water.parts;
    let total = s.world.water_total();
    assert_eq!(s.world.tile(at(-3, 1)).unwrap().water, 2 * parts);
    // Two digs take the tile beside the lake down to the lake's bed.
    let id = bot(&mut s, -1, 1);
    let main = format!("dig(-2, 1)\ndig(-2, 1)\nlog('dug', tile(-2, 1).height)\n{IDLE}");
    let logs = run(&mut s, id, &main, vec![], 80);
    assert_eq!(logs, ["dug 2"]);
    let dug = s.world.tile(at(-2, 1)).unwrap().water;
    let lake = s.world.tile(at(-3, 1)).unwrap().water;
    assert!(dug > parts, "the dug tile holds {dug}");
    assert!(lake < 2 * parts, "and the lake gave it: {lake}");
    assert_eq!(s.world.water_total(), total);
    // Filling it back in sheds the water again.
    let main = format!("fill(-2, 1)\nfill(-2, 1)\nlog('filled', tile(-2, 1).height)\n{IDLE}");
    let logs = run(&mut s, id, &main, vec![], 200);
    assert_eq!(logs, ["filled 4"]);
    assert_eq!(s.world.tile(at(-2, 1)).unwrap().water, 0);
    assert_eq!(s.world.water_total(), total);
}

fn mark(tile: TilePos, value: Option<&str>) -> Command {
    Command::new(
        0,
        TeamId(0),
        0,
        CommandKind::Mark {
            at: tile,
            plan: PlanKind::Ground,
            value: value.map(str::to_string),
        },
    )
}

#[test]
fn a_ground_plan_is_a_height_and_only_a_height() {
    let s = sim();
    assert!(s.validate(&mark(at(-1, 1), Some("2"))).is_ok());
    for bad in ["x", "10", "02", "-1", ""] {
        assert!(
            s.validate(&mark(at(-1, 1), Some(bad))).is_err(),
            "`{bad}` is no height"
        );
    }
    // It has no plan to clear: an unmark withdraws it.
    assert!(
        s.validate(&mark(at(-1, 1), None))
            .unwrap_err()
            .contains("no plan to clear")
    );
    // The strip lists every height the tables allow.
    let data = Data::load().unwrap();
    let heights = Sim::markable(&data, PlanKind::Ground);
    assert_eq!(heights.first().map(String::as_str), Some("0"));
    assert_eq!(heights.len(), usize::from(data.world.height.max) + 1);
}

#[test]
fn a_ground_plan_goes_where_the_team_has_seen_and_the_ground_may_change() {
    let mut s = sim();
    // One tick, so that the team's printer has seen the map.
    s.step(&[], &[]);
    let ground = |s: &Sim, x: i32, y: i32| {
        s.world
            .plans(TeamId(0), at(x, y))
            .and_then(|p| p.ground.clone())
    };
    let mut cmds = vec![
        mark(at(-1, 1), Some("2")),
        // The height the tile already has: dropped.
        mark(at(-1, -1), Some("4")),
        // A deposit's ground does not change: dropped.
        mark(at(1, 1), Some("2")),
        // Rock at its own height is a plan to mine it: kept.
        mark(at(0, 2), Some("4")),
    ];
    for (i, c) in cmds.iter_mut().enumerate() {
        c.tick = 2;
        c.seq = i as u32;
    }
    s.step(&cmds, &[]);
    assert_eq!(ground(&s, -1, 1), Some(Plan::Set("2".into())));
    assert_eq!(ground(&s, -1, -1), None);
    assert_eq!(ground(&s, 1, 1), None);
    assert_eq!(ground(&s, 0, 2), Some(Plan::Set("4".into())));
    // An unmark withdraws one.
    let mut unmark = Command::new(
        3,
        TeamId(0),
        0,
        CommandKind::Unmark {
            at: at(-1, 1),
            plan: PlanKind::Ground,
        },
    );
    unmark.tick = 3;
    s.step(&[unmark], &[]);
    assert_eq!(ground(&s, -1, 1), None);
}

#[test]
fn a_bot_carries_out_a_ground_plan_and_uses_it_up() {
    let mut s = sim();
    let id = bot(&mut s, -1, 0);
    // Dig the tile west of the bot to 2, and mine the rock — which the
    // bot cannot reach from here, so that plan stands.
    let marks = vec![mark(at(-2, 0), Some("2")), mark(at(0, 2), Some("4"))];
    let main = "wait(3)\nlog(tile(-2, 0).plans['ground'], len(plans('ground')))\nwhile True:\n    mine = None\n    for p in plans('ground'):\n        if p[0] == -2:\n            mine = p\n    if mine is None:\n        break\n    if tile(mine[0], mine[1]).height > mine[2]:\n        dig(mine[0], mine[1])\n    else:\n        fill(mine[0], mine[1])\nlog('done', tile(-2, 0).height, tile(-2, 0).plans['ground'], plans('ground'))\nwhile True:\n    wait(1)\n";
    let logs = run(&mut s, id, main, marks, 30);
    assert_eq!(logs, ["2 2", "done 2 None [(0, 2, 4)]"]);
    assert_eq!(level(&s, -2, 0), 2);
    assert_eq!(
        s.world
            .plans(TeamId(0), at(-2, 0))
            .and_then(|p| p.ground.clone()),
        None
    );
}

#[test]
fn earth_is_carried_and_stored_and_no_deposit_holds_it() {
    let mut s = sim();
    // A bot beside a depot and beside a deposit, with a tile to dig.
    let id = bot(&mut s, 2, 1);
    let depot = s
        .world
        .place(Model::Depot, TeamId(0), at(2, 2), Some("depot".into()));
    let main = [
        attempt("earth from a deposit", "pick('earth')"),
        "dig(2, 0)\ndig(2, 0)\nlog(me().load['earth'])\ndrop('earth')\nlog(me().load['earth'])\n"
            .to_string(),
        "pick('earth')\nlog(me().load['earth'])\n".to_string(),
        IDLE.to_string(),
    ]
    .concat();
    let logs = run(&mut s, id, &main, vec![], 40);
    assert_eq!(logs, ["earth from a deposit refused", "2", "0", "2"]);
    assert_eq!(
        s.world.machines[&depot].store["earth"],
        lang::Num::ZERO,
        "it was picked back"
    );
    // The deposit holds ore and nothing else.
    let deposit = s.world.tile(at(1, 1)).unwrap().deposit.as_ref().unwrap();
    assert_eq!(deposit.amount.keys().collect::<Vec<_>>(), ["ore"]);
}

#[test]
fn a_ground_plan_and_a_dug_tile_are_in_the_hash() {
    let run_one = |plan: Option<&str>, dig: bool| {
        let mut s = sim();
        let id = bot(&mut s, -1, 0);
        let main = if dig {
            format!("dig(-2, 0)\n{IDLE}")
        } else {
            IDLE.to_string()
        };
        let marks = plan
            .map(|h| vec![mark(at(-2, 1), Some(h))])
            .unwrap_or_default();
        run(&mut s, id, &main, marks, 8);
        s.state_hash()
    };
    assert_eq!(run_one(None, false), run_one(None, false));
    assert_ne!(run_one(None, false), run_one(Some("2"), false), "a plan");
    assert_ne!(
        run_one(Some("2"), false),
        run_one(Some("3"), false),
        "its height"
    );
    assert_ne!(run_one(None, false), run_one(None, true), "a dug tile");
}
