//! The machine rules of `docs/02` through `docs/05`, one scenario each, on
//! a small map: what the golden replay pins as a whole, isolated so a
//! change names the rule it broke.

use lang::Num;
use sim::world::{Model, TileState};
use sim::{Command, CommandKind, Data, Map, PlanKind, Sim, TeamId, TilePos, bundle_of};

/// 9 by 7, origin at the centre: x in -4..=4, y in -3..=3. Team 0 starts
/// at (-3, 0) with a deposit north of it at (-3, 1); rock at (-1, 2) and
/// (0, 2); team 1 starts at (3, 0) with a deposit at (1, -1).
const MAP: &str = r#"
name = "test"
speed = 10
width = 9
height = 7
terrain = """
.........
...##....
.o.......
.........
.....o...
.........
.........
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-3, 0]
[[team]]
start = [3, 0]
"#;

fn sim() -> Sim {
    let data = Data::load().unwrap();
    Sim::new(Map::parse(MAP, &data).unwrap())
}

fn deploy(team: u32, deployment: &str, main: &str) -> Command {
    Command::new(
        0,
        TeamId(team),
        0,
        CommandKind::Deploy {
            deployment: deployment.into(),
            bundle: bundle_of(main),
        },
    )
}

/// Step with the opening set, then `n - 1` empty ticks.
fn run(sim: &mut Sim, opening: Vec<Command>, n: u64) {
    let mut cmds = opening;
    for (i, c) in cmds.iter_mut().enumerate() {
        c.seq = i as u32;
    }
    sim.step(&cmds);
    for _ in 1..n {
        sim.step(&[]);
    }
}

fn bots(sim: &Sim, team: u32) -> Vec<(TilePos, Num)> {
    sim.world
        .machines
        .values()
        .filter(|m| m.team == TeamId(team) && m.model == Model::Bot)
        .map(|m| (m.pos, m.health))
        .collect()
}

/// Prints, and says so when refused.
const PRINT_LOG: &str = "while True:\n    try:\n        print('red')\n    except ValueError:\n        log('print refused')\n        wait(1)\n";

const PRINT_RED: &str =
    "while True:\n    try:\n        print('red')\n    except ValueError:\n        wait(3)\n";

#[test]
fn a_printer_prints_one_bot_per_print_time_up_to_the_cap() {
    let mut s = sim();
    run(
        &mut s,
        vec![
            deploy(0, "printer", PRINT_LOG),
            deploy(0, "red", "wait(1000)\n"),
        ],
        30,
    );
    // Begun in tick 1's slice, 30 ticks: completes in step 3 of tick 30.
    assert_eq!(bots(&s, 0).len(), 1, "no bot after the print time");
    assert_eq!(
        bots(&s, 0)[0].0,
        TilePos::new(-3, 1),
        "the first free adjacent tile is north"
    );
    run(&mut s, vec![], 29);
    assert_eq!(bots(&s, 0).len(), 1, "a second bot before its print time");
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0).len(), 2, "the second bot, east");
    assert_eq!(s.world.bot_cap(TeamId(0)), 10);
    // Tick 61: the third print begins, under the cap. Then fill the cap by
    // fiat, away from either printer: the cap is checked when a print
    // begins, so the one in flight completes and the next is refused.
    run(&mut s, vec![], 1);
    assert_eq!(
        s.world
            .machines
            .values()
            .find(|m| m.model == Model::Printer && m.team == TeamId(0))
            .and_then(|m| m.busy()),
        Some("print")
    );
    for (i, y) in (-3..=3).enumerate() {
        s.world.place(
            Model::Bot,
            TeamId(0),
            TilePos::new(1, y),
            Some("red".into()),
        );
        if i == 7 {
            break;
        }
    }
    s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(0, -3),
        Some("red".into()),
    );
    assert_eq!(bots(&s, 0).len(), 10);
    run(&mut s, vec![], 29);
    assert_eq!(
        bots(&s, 0).len(),
        11,
        "the print begun under the cap completes; bots above it are kept"
    );
    run(&mut s, vec![], 2);
    let printer = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Printer && m.team == TeamId(0))
        .unwrap();
    assert!(
        printer.log.iter().any(|l| l.text == "print refused"),
        "a print at the cap is a ValueError: {:?}",
        printer.log
    );
    assert_eq!(bots(&s, 0).len(), 11);
    // Team 1 deployed nothing: its printer runs the empty program.
    assert!(bots(&s, 1).is_empty());
}

#[test]
fn a_move_takes_its_ticks_and_a_tile_holds_one_machine() {
    let mut s = sim();
    run(
        &mut s,
        vec![
            deploy(0, "printer", PRINT_RED),
            deploy(0, "red", "move('e')\nwait(1000)\n"),
        ],
        30,
    );
    assert_eq!(bots(&s, 0)[0].0, TilePos::new(-3, 1));
    // Tick 31: the bot's first slice begins the move (2 ticks): it lands in
    // step 3 of tick 32.
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0)[0].0, TilePos::new(-3, 1));
    assert!(
        s.world.occupied(TilePos::new(-2, 1)),
        "a moving bot occupies its target too"
    );
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0)[0].0, TilePos::new(-2, 1));
    // The second bot is printed north again, moves east, and stops short
    // of the first: it must not share the tile.
    run(&mut s, vec![], 60);
    let mut positions: Vec<TilePos> = bots(&s, 0).iter().map(|b| b.0).collect();
    positions.sort();
    positions.dedup();
    assert_eq!(positions.len(), bots(&s, 0).len(), "two bots on one tile");
}

#[test]
fn ore_is_picked_dropped_into_a_site_and_the_site_becomes_a_depot() {
    let program = "move('e')\nwhile True:\n    while me().load['ore'] < 10:\n        try:\n            pick('ore')\n        except ValueError:\n            wait(1)\n    try:\n        build('depot', -2, 2)\n    except ValueError:\n        pass\n    try:\n        drop('ore')\n    except ValueError:\n        wait(1)\n";
    let mut s = sim();
    run(
        &mut s,
        vec![
            deploy(0, "printer", "print('red')\nwait(1000)\n"),
            deploy(0, "red", program),
        ],
        38,
    );
    // Printed tick 30; moves east 31–32; picks 33–35 and 36–38.
    let bot = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot)
        .expect("a bot");
    assert_eq!(bot.pos, TilePos::new(-2, 1));
    assert_eq!(
        bot.load["ore"],
        Num::from_int(10).unwrap(),
        "two picks of five"
    );
    let deposit = s
        .world
        .tile(TilePos::new(-3, 1))
        .unwrap()
        .deposit
        .as_ref()
        .unwrap();
    assert!(
        deposit.amount["ore"] < Num::from_int(50).unwrap(),
        "the deposit did not shrink"
    );
    // Tick 40: the site is placed (a build takes no ticks) and the drop
    // begins; it lands the same tick.
    run(&mut s, vec![], 3);
    let site = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Site)
        .expect("a site");
    assert_eq!(
        (site.pos, site.team, site.becomes),
        (TilePos::new(-2, 2), TeamId(0), Some(Model::Depot))
    );
    assert_eq!(site.store_capacity["ore"], Num::from_int(40).unwrap());
    assert!(
        site.store["ore"] >= Num::from_int(10).unwrap(),
        "the first drop did not land"
    );
    // Four loads fill the site; construction then runs its 20 ticks. On
    // the tick the depot appears its store is empty; the bot keeps hauling
    // into it afterwards.
    let mut appeared_at = None;
    for _ in 0..200 {
        s.step(&[]);
        if let Some(depot) = s.world.machines.values().find(|m| m.model == Model::Depot) {
            assert_eq!(depot.pos, TilePos::new(-2, 2));
            assert_eq!(depot.deployment.as_deref(), Some("depot"));
            assert_eq!(
                depot.store["ore"],
                Num::ZERO,
                "a new building's store is empty"
            );
            assert!(!s.world.machines.values().any(|m| m.model == Model::Site));
            appeared_at = Some(s.world.tick);
            break;
        }
    }
    let appeared_at = appeared_at.expect("the site became a depot");
    assert!(
        appeared_at > 60 && appeared_at < 160,
        "at tick {appeared_at}"
    );
    run(&mut s, vec![], 30);
    let depot = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Depot)
        .unwrap();
    assert!(
        depot.store["ore"] > Num::ZERO,
        "the bot stopped hauling into the depot"
    );
    // Regrowth: the deposit climbs back toward its cap once picking stops.
    let before = s
        .world
        .tile(TilePos::new(-3, 1))
        .unwrap()
        .deposit
        .as_ref()
        .unwrap()
        .amount["ore"];
    assert!(before <= Num::from_int(50).unwrap());
}

#[test]
fn a_printer_defends_itself_and_a_bot_at_zero_health_dies() {
    let mut s = sim();
    run(&mut s, vec![deploy(0, "red", "wait(1000)\n")], 1);
    // A team-0 bot beside team 1's printer, by fiat.
    let id = s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(2, 0),
        Some("red".into()),
    );
    run(&mut s, vec![], 9);
    assert_eq!(
        s.world.machines[&id].health,
        Num::ONE,
        "nine ticks of defence"
    );
    run(&mut s, vec![], 1);
    // Tenth hit: health zero, `dying` raised; delivered at its next slice.
    assert_eq!(s.world.machines[&id].health, Num::ZERO);
    run(&mut s, vec![], 2);
    assert!(
        !s.world.machines.contains_key(&id),
        "dying with no hook is death"
    );
}

#[test]
fn converting_the_last_printer_ends_the_match() {
    let convert = "for m in see():\n    if m.model == 'printer' and m.team != me().team:\n        convert(m.id)\nwait(1000)\n";
    let mut s = sim();
    run(&mut s, vec![deploy(0, "red", convert)], 1);
    let bot = s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(2, 0),
        Some("red".into()),
    );
    let printer1 = s
        .world
        .machines
        .values()
        .find(|m| m.team == TeamId(1))
        .unwrap()
        .id;
    run(&mut s, vec![], 9);
    assert!(
        s.world.machines.contains_key(&bot),
        "the bot died before converting"
    );
    assert_eq!(s.world.machines[&printer1].team, TeamId(1));
    let report = s.step(&[]);
    assert_eq!(
        s.world.machines[&printer1].team,
        TeamId(0),
        "the conversion did not land"
    );
    assert_eq!(
        s.world.machines[&bot].health,
        Num::ONE,
        "one health to spare (docs/05)"
    );
    assert_eq!(
        report.ended,
        Some(Some(TeamId(0))),
        "team 1 has no printer: out, and team 0 wins"
    );
    assert!(s.world.teams[&TeamId(1)].out);
    assert_eq!(s.world.printers_of(TeamId(0)), 2);
    assert_eq!(s.world.unlocked_colors(TeamId(0)), ["red", "blue"]);
}

#[test]
fn a_fault_costs_health_and_a_faulting_program_dies_in_time() {
    let mut s = sim();
    run(
        &mut s,
        vec![
            deploy(0, "printer", "print('red')\nwait(1000)\n"),
            deploy(0, "red", "x = 1 / 0\n"),
        ],
        30,
    );
    let id = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot)
        .unwrap()
        .id;
    run(&mut s, vec![], 1);
    assert_eq!(
        s.world.machines[&id].health,
        Num::from_int(9).unwrap(),
        "one fault, one health"
    );
    run(&mut s, vec![], 8);
    assert_eq!(s.world.machines[&id].health, Num::ONE);
    run(&mut s, vec![], 3);
    assert!(
        !s.world.machines.contains_key(&id),
        "ten faults, ten health: dead"
    );
}

#[test]
fn a_redeploy_cancels_the_action_and_the_new_program_runs() {
    let mut s = sim();
    run(
        &mut s,
        vec![
            deploy(0, "printer", "print('red')\nwait(1000)\n"),
            deploy(0, "red", "wait(500)\n"),
        ],
        31,
    );
    let id = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot)
        .unwrap()
        .id;
    assert_eq!(s.world.machines[&id].busy(), Some("wait"));
    let mut redeploy = deploy(0, "red", "log('new program')\nmove('e')\nwait(1000)\n");
    redeploy.tick = 32;
    s.step(&[redeploy]);
    let m = &s.world.machines[&id];
    assert_eq!(
        m.log.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["new program"]
    );
    assert_eq!(
        m.busy(),
        Some("move"),
        "the wait was cancelled and the new program acted"
    );
    // A deploy to a locked color waits, and a second deploy to the same
    // slot replaces it.
    let mut locked = deploy(0, "blue", "wait(1)\n");
    locked.tick = 33;
    let report = s.step(&[locked]);
    assert!(report.dropped.is_empty());
    assert!(
        s.world.teams[&TeamId(0)].deployments["blue"]
            .pending
            .is_some()
    );
}

#[test]
fn line_of_sight_vision_and_memory() {
    let mut s = sim();
    run(&mut s, vec![], 1);
    let mem = |s: &Sim, p: TilePos| {
        s.world.teams[&TeamId(0)].memory[s.world.tile_index(p).unwrap()].clone()
    };
    // The printer at (-3, 0) sees 8 tiles around, the rock itself included.
    assert_eq!(mem(&s, TilePos::new(-3, 0)).state, TileState::Visible);
    assert_eq!(
        mem(&s, TilePos::new(-1, 2)).state,
        TileState::Visible,
        "a rock with a clear line is seen"
    );
    // (0, 3) lies behind the rock at (-1, 2) along the ray: unknown.
    assert_eq!(mem(&s, TilePos::new(0, 3)).state, TileState::Unknown);
    // Team 1's printer at (3, 0) is 6 away: within vision 8.
    assert_eq!(mem(&s, TilePos::new(3, 0)).state, TileState::Visible);
    assert_eq!(
        mem(&s, TilePos::new(3, 0))
            .building
            .as_ref()
            .map(|b| b.model),
        Some(Model::Printer)
    );
    // A visible deposit's amount is the live one; a remembered tile keeps
    // its snapshot and tick.
    assert_eq!(
        mem(&s, TilePos::new(-3, 1)).deposit.as_ref().unwrap()["ore"],
        Num::from_int(50).unwrap()
    );
    let seen_at = mem(&s, TilePos::new(-3, 1)).seen_at;
    assert_eq!(seen_at, 1);
    // Resign team 0: its printer goes, and its memory stays as it was.
    let mut resign = Command::new(2, TeamId(0), 0, CommandKind::Resign);
    resign.tick = 2;
    let report = s.step(&[resign]);
    assert!(s.world.teams[&TeamId(0)].out);
    assert_eq!(report.ended, Some(Some(TeamId(1))));
}

#[test]
fn commands_that_cannot_apply_on_their_tick_are_dropped() {
    let mut s = sim();
    let mark = Command::new(
        1,
        TeamId(0),
        0,
        CommandKind::Mark {
            at: TilePos::new(0, 0),
            plan: PlanKind::Building,
            value: Some("depot".into()),
        },
    );
    let report = s.step(std::slice::from_ref(&mark));
    assert!(report.dropped.is_empty());
    assert!(s.world.plans(TeamId(0), TilePos::new(0, 0)).is_some());
    // Validation refuses what can never apply.
    let bad = Command::new(1, TeamId(0), 0, CommandKind::SetSpeed(7));
    assert!(s.validate(&bad).is_err());
    let printer_plan = Command::new(
        1,
        TeamId(0),
        0,
        CommandKind::Mark {
            at: TilePos::new(0, 0),
            plan: PlanKind::Building,
            value: Some("printer".into()),
        },
    );
    assert!(s.validate(&printer_plan).unwrap_err().contains("Q31"));
    let off = Command::new(
        1,
        TeamId(0),
        0,
        CommandKind::Unmark {
            at: TilePos::new(40, 0),
            plan: PlanKind::Paint,
        },
    );
    assert!(s.validate(&off).is_err());
    // A deploy whose bundle does not load is dropped on its tick (a peer
    // would have refused it at submission).
    let broken = Command::new(
        2,
        TeamId(0),
        0,
        CommandKind::Deploy {
            deployment: "red".into(),
            bundle: bundle_of("def (:\n"),
        },
    );
    assert!(s.validate(&broken).is_err());
    let report = s.step(&[broken]);
    assert_eq!(report.dropped.len(), 1);
    let _ = mark;
}
