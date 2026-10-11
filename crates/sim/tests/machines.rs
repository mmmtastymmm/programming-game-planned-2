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

fn printer_of(sim: &Sim, team: u32) -> &sim::world::Machine {
    sim.world
        .machines
        .values()
        .find(|m| m.model == Model::Printer && m.team == TeamId(team))
        .expect("a printer")
}

/// Q42: printing is the printers' own rule. Begun in tick 1's print step,
/// a print of 30 ticks completes in step 3 of tick 31; printers keep
/// printing until the team is at its cap.
#[test]
fn a_printer_prints_by_itself_one_bot_per_print_time_up_to_the_cap() {
    let mut s = sim();
    run(&mut s, vec![deploy(0, "bot", "wait(1000)\n")], 30);
    assert!(bots(&s, 0).is_empty(), "a bot before the print time");
    assert_eq!(printer_of(&s, 0).printing, 1);
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0).len(), 1, "no bot after the print time");
    assert_eq!(
        bots(&s, 0)[0].0,
        TilePos::new(-3, 1),
        "the first free adjacent tile is north"
    );
    let bot = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot && m.team == TeamId(0))
        .unwrap();
    assert_eq!(bot.deployment.as_deref(), Some("bot"));
    assert_eq!(bot.name, format!("bot{}", bot.id.0));
    // The next print began in the same tick's print step.
    assert_eq!(printer_of(&s, 0).printing, 30);
    run(&mut s, vec![], 29);
    assert_eq!(bots(&s, 0).len(), 1, "a second bot before its print time");
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0).len(), 2, "the second bot, east");
    assert_eq!(s.world.bot_cap(TeamId(0)), 10);
    // A third print is in flight. Fill the cap by fiat, away from the
    // printer: the print in flight completes, the bots above the cap are
    // kept, and no further print begins.
    assert_eq!(printer_of(&s, 0).printing, 30);
    for x in -4..=3 {
        s.world.place(
            Model::Bot,
            TeamId(0),
            TilePos::new(x, -3),
            Some("bot".into()),
        );
    }
    assert_eq!(bots(&s, 0).len(), 10);
    run(&mut s, vec![], 30);
    assert_eq!(
        bots(&s, 0).len(),
        11,
        "the print begun under the cap completes; bots above it are kept"
    );
    assert_eq!(printer_of(&s, 0).printing, 0, "a print began at the cap");
    run(&mut s, vec![], 40);
    assert_eq!(bots(&s, 0).len(), 11);
    // Team 1 deployed nothing: its printer prints all the same, and its
    // bots run the empty program.
    assert!(!bots(&s, 1).is_empty());
}

/// Q42: printers take turns — the ready printer with the least id past
/// the team's print turn, wrapping to the least.
#[test]
fn printers_take_turns_by_entity_id() {
    let mut s = sim();
    // Team 0: printer 1 from the map, printer 3 by fiat; team 1's is 2.
    let p1 = printer_of(&s, 0).id;
    let p3 = s.world.place(
        Model::Printer,
        TeamId(0),
        TilePos::new(0, 0),
        Some("printer".into()),
    );
    assert!(p3 > p1);
    // Nineteen bots by fiat: a cap of twenty leaves a shortfall of one.
    let mut fiat = Vec::new();
    for y in [-3, -2] {
        for x in -4..=4 {
            fiat.push(s.world.place(
                Model::Bot,
                TeamId(0),
                TilePos::new(x, y),
                Some("bot".into()),
            ));
        }
    }
    fiat.push(s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(4, -1),
        Some("bot".into()),
    ));
    assert_eq!(bots(&s, 0).len(), 19);
    let printing = |s: &Sim, id| s.world.machines[&id].printing;
    let turn = |s: &Sim| s.world.teams[&TeamId(0)].print_turn;
    run(&mut s, vec![deploy(0, "bot", "wait(1000)\n")], 1);
    assert_eq!((printing(&s, p1), printing(&s, p3)), (30, 0));
    assert_eq!(turn(&s), p1, "the first print goes to the least id");
    s.world.machines.remove(&fiat[0]);
    run(&mut s, vec![], 1);
    assert_eq!(printing(&s, p3), 30, "the next turn is the next id");
    assert_eq!(turn(&s), p3);
    s.world.machines.remove(&fiat[1]);
    run(&mut s, vec![], 1);
    assert_eq!(turn(&s), p3, "no printer was ready, so the turn stays");
    // Tick 31: printer 1's print lands, and it is the only one ready for
    // the bot still short — past the greatest id, the turn wraps to it.
    // Tick 32: printer 3's lands, and the team is at its cap.
    run(&mut s, vec![], 28);
    assert_eq!(bots(&s, 0).len(), 18);
    assert_eq!(turn(&s), p1, "past the greatest id the turn wraps");
    assert_eq!(printing(&s, p1), 30);
    run(&mut s, vec![], 1);
    assert_eq!(bots(&s, 0).len(), 19);
    assert_eq!((printing(&s, p1), printing(&s, p3)), (29, 0));
    assert_eq!(turn(&s), p1, "at the cap, no print began");
}

#[test]
fn a_move_takes_its_ticks_and_a_tile_holds_one_machine() {
    let mut s = sim();
    run(
        &mut s,
        vec![deploy(0, "bot", "move('e')\nwait(1000)\n")],
        31,
    );
    assert_eq!(bots(&s, 0)[0].0, TilePos::new(-3, 1));
    // Tick 32: the bot's first slice begins the move (2 ticks): it lands in
    // step 3 of tick 33.
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
    run(&mut s, vec![deploy(0, "bot", program)], 39);
    // Printed tick 31; moves east 32–33; picks 34–36 and 37–39.
    let bot = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot && m.team == TeamId(0))
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
    // Tick 41: the site is placed (a build takes no ticks) and the drop
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
    assert_eq!(
        site.store_capacity["ore"],
        Num::from_int(40).unwrap(),
        "a depot's price does not rise"
    );
    assert!(
        site.store["ore"] >= Num::from_int(10).unwrap(),
        "the first drop did not land"
    );
    // Four loads fill the site; construction then runs its 20 ticks. On
    // the tick the depot appears its store is empty; the bots keep hauling
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
        "the bots stopped hauling into the depot"
    );
}

/// Q42: a printer's price is `cost + n × cost_step`, `n` the team's
/// printers and other printer sites; a depot's does not rise.
#[test]
fn a_printer_site_costs_more_for_every_printer_the_team_has() {
    let data = Data::load().unwrap();
    let pd = data.model("printer");
    let price = |n: i128| {
        pd.cost["ore"]
            .checked_add(
                pd.cost_step["ore"]
                    .checked_mul(Num::from_int(n).unwrap())
                    .unwrap(),
            )
            .unwrap()
    };
    let mut s = sim();
    assert_eq!(s.world.price(TeamId(0), Model::Printer)["ore"], price(1));
    assert_eq!(
        s.world.price(TeamId(0), Model::Depot)["ore"],
        data.model("depot").cost["ore"]
    );
    // A bot builds a printer site beside it: the starting printer counts,
    // the new site does not.
    run(
        &mut s,
        vec![deploy(0, "bot", "build('printer', -1, -1)\nwait(1000)\n")],
        1,
    );
    s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(-1, 0),
        Some("bot".into()),
    );
    run(&mut s, vec![], 2);
    let site = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Site)
        .expect("a printer site");
    assert_eq!(
        (site.pos, site.becomes),
        (TilePos::new(-1, -1), Some(Model::Printer))
    );
    assert_eq!(site.name, format!("printer{}", site.id.0));
    assert_eq!(site.store_capacity["ore"], price(1));
    // With the site standing, the next one costs one step more; team 1's
    // count is its own.
    assert_eq!(s.world.price(TeamId(0), Model::Printer)["ore"], price(2));
    assert_eq!(s.world.price(TeamId(1), Model::Printer)["ore"], price(1));
}

#[test]
fn a_printer_defends_itself_and_a_bot_at_zero_health_dies() {
    let mut s = sim();
    run(&mut s, vec![deploy(0, "bot", "wait(1000)\n")], 1);
    // A team-0 bot beside team 1's printer, by fiat.
    let id = s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(2, 0),
        Some("bot".into()),
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

/// Q42: a team with no printer at the end of a tick is out, whatever
/// printer site it holds; a printer at zero health loses its print.
#[test]
fn losing_the_last_printer_ends_the_match_whatever_sites_remain() {
    let attack = "for m in see():\n    if m.model == 'printer' and m.team != me().team:\n        attack(m.id)\nwait(1000)\n";
    let mut s = sim();
    run(&mut s, vec![deploy(0, "bot", attack)], 1);
    let printer1 = printer_of(&s, 1).id;
    assert_eq!(s.world.machines[&printer1].printing, 30);
    // Team 1's printer at one health, and a printer site of its own.
    s.world.machines.get_mut(&printer1).unwrap().health = Num::ONE;
    let site = s
        .world
        .place(Model::Site, TeamId(1), TilePos::new(3, -2), None);
    s.world.machines.get_mut(&site).unwrap().becomes = Some(Model::Printer);
    s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(2, 0),
        Some("bot".into()),
    );
    // Tick 2's slice begins the attack (3 ticks): it lands in tick 4.
    run(&mut s, vec![], 3);
    assert_eq!(s.world.machines[&printer1].health, Num::ZERO);
    assert_eq!(
        s.world.machines[&printer1].printing, 0,
        "a printer at zero health keeps its print"
    );
    let mut ended = None;
    for _ in 0..5 {
        let report = s.step(&[]);
        if report.ended.is_some() {
            ended = report.ended;
            break;
        }
    }
    assert_eq!(
        ended,
        Some(Some(TeamId(0))),
        "team 1 has no printer: out, site or no site, and team 0 wins"
    );
    assert!(s.world.teams[&TeamId(1)].out);
    assert!(!s.world.machines.contains_key(&site));
}

/// A printer whose own program faults it to zero health loses its print
/// at once, in the fault's prologue, before step 3 could complete it.
#[test]
fn a_printer_faulted_to_zero_loses_its_print() {
    let mut s = sim();
    // A second printer keeps team 0 in the match; both begin a print in
    // tick 1, due in step 3 of tick 31.
    let p2 = s.world.place(
        Model::Printer,
        TeamId(0),
        TilePos::new(0, 0),
        Some("printer".into()),
    );
    run(
        &mut s,
        vec![deploy(0, "printer", "wait(30)\nx = 1 / 0\n")],
        1,
    );
    let p = printer_of(&s, 0).id;
    assert_ne!(p, p2);
    s.world.machines.get_mut(&p).unwrap().health = Num::ONE;
    run(&mut s, vec![], 29);
    assert_eq!(s.world.machines[&p].printing, 1);
    // Tick 31: the wait ended in tick 30, so the program faults in tick
    // 31's slice, before the print would complete in its step 3.
    run(&mut s, vec![], 1);
    assert!(s.world.machines.get(&p).is_none_or(|m| m.printing == 0));
    assert_eq!(
        bots(&s, 0).len(),
        1,
        "only the healthy printer's print completed"
    );
}

#[test]
fn a_fault_costs_health_and_a_faulting_program_dies_in_time() {
    let mut s = sim();
    run(&mut s, vec![deploy(0, "bot", "x = 1 / 0\n")], 31);
    let id = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot && m.team == TeamId(0))
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
    run(&mut s, vec![deploy(0, "bot", "wait(500)\n")], 32);
    let id = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Bot && m.team == TeamId(0))
        .unwrap()
        .id;
    assert_eq!(s.world.machines[&id].busy(), Some("wait"));
    let mut redeploy = deploy(0, "bot", "log('new program')\nmove('e')\nwait(1000)\n");
    redeploy.tick = 33;
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
    assert!(
        s.validate(&printer_plan).is_ok(),
        "a printer plan is a plan like any other (Q42)"
    );
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
            deployment: "bot".into(),
            bundle: bundle_of("def (:\n"),
        },
    );
    assert!(s.validate(&broken).is_err());
    let report = s.step(&[broken]);
    assert_eq!(report.dropped.len(), 1);
    let _ = mark;
}

/// Q42: a team's deployments are `bot`, `printer` and `depot`, and no
/// others.
#[test]
fn a_team_has_one_deployment_per_model() {
    let s = sim();
    for good in ["bot", "printer", "depot"] {
        assert!(s.validate(&deploy(0, good, "wait(1)\n")).is_ok());
    }
    for bad in ["1", "9", "red", "site", ""] {
        assert!(
            s.validate(&deploy(0, bad, "wait(1)\n")).is_err(),
            "`{bad}` accepted as a deployment"
        );
    }
    let snap = s.snapshot();
    let team = snap.teams.iter().find(|t| t.id == TeamId(0)).unwrap();
    assert_eq!(
        team.deployments.keys().cloned().collect::<Vec<_>>(),
        ["bot", "depot", "printer"]
    );
}

/// A tile a bot is moving onto is occupied for `build` as for `move`: a
/// site never lands under a bot (docs/02, Occupancy).
#[test]
fn a_build_refuses_a_tile_a_bot_is_moving_onto() {
    let program = "if me().pos[0] == -1:\n    move('e')\nelse:\n    try:\n        build('depot', 0, 0)\n    except ValueError:\n        log('refused')\nwait(1000)\n";
    let mut s = sim();
    run(&mut s, vec![deploy(0, "bot", program)], 1);
    // The mover has the lower id, so its move begins first in the slice.
    let mover = s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(-1, 0),
        Some("bot".into()),
    );
    let builder = s.world.place(
        Model::Bot,
        TeamId(0),
        TilePos::new(0, -1),
        Some("bot".into()),
    );
    assert!(mover < builder);
    run(&mut s, vec![], 1);
    assert!(
        s.world.machines[&builder]
            .log
            .iter()
            .any(|l| l.text == "refused"),
        "the build was not refused"
    );
    assert!(!s.world.machines.values().any(|m| m.model == Model::Site));
    run(&mut s, vec![], 1);
    assert_eq!(s.world.machines[&mover].pos, TilePos::new(0, 0));
}

/// Tick 0's opening set applies with tick 1, before tick 1's own commands,
/// whatever their sequence numbers (docs/06, The command log).
#[test]
fn the_opening_set_applies_before_tick_ones_commands() {
    let mut s = sim();
    let mut opening = deploy(0, "bot", "wait(1)\n");
    opening.seq = 1;
    let mut newer = deploy(0, "bot", "wait(2)\n");
    newer.tick = 1;
    newer.seq = 0;
    let want = sim::tick::load_bundle(&bundle_of("wait(2)\n"), &s.world.limits)
        .unwrap()
        .version;
    s.step(&[opening, newer]);
    let slot = &s.world.teams[&TeamId(0)].deployments["bot"];
    assert_eq!(slot.bundle.as_ref().map(|b| b.version), Some(want));
}
