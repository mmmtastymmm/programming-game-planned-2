//! Power (`docs/08`, Q62 and Q63), one rule a test, on a small map: a
//! bot's battery, charge costs, being out of power and waking, `low_power`,
//! grids and how they are served, dark buildings, recharging, pylons and
//! wire. The golden replay pins these as a whole; here a change names the
//! rule it broke.

use lang::Num;
use sim::data::{MACHINES_TOML, MachinesData};
use sim::world::{EntityId, Model, Plan, TileState};
use sim::{Command, CommandKind, Data, Map, PlanKind, Sim, TeamId, TilePos, bundle_of};

/// 13 by 7, origin at the centre: x in -6..=6, y in -3..=3. Team 0 starts
/// at (-5, 0) and team 1 at (5, 0), each with a deposit beside it; rock at
/// (0, 2).
const MAP: &str = r#"
name = "power"
speed = 10
width = 13
height = 7
terrain = """
.............
......#......
.o...........
.............
...........o.
.............
.............
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-5, 0]
[[team]]
start = [5, 0]
"#;

const T0: TeamId = TeamId(0);
const T1: TeamId = TeamId(1);

fn sim() -> Sim {
    let data = Data::load().unwrap();
    Sim::new(Map::parse(MAP, &data).unwrap())
}

/// A sim on machine data edited from the shipped table: each `(from, to)`
/// replaces text that must be there.
fn sim_with(edits: &[(&str, &str)]) -> Sim {
    let mut text = MACHINES_TOML.to_string();
    for (from, to) in edits {
        assert!(text.contains(from), "the shipped table lacks {from:?}");
        text = text.replace(from, to);
    }
    let mut data = Data::load().unwrap();
    data.machines = MachinesData::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    let map = Map::parse(MAP, &data).unwrap();
    Sim::with_data(map, data)
}

/// A charger that drains `n` a tick, where the shipped one drains nothing.
fn charger_drains(n: &str) -> (&'static str, String) {
    (
        "generation = \"0.5\"\ndrain = 0",
        format!("generation = \"0.5\"\ndrain = {n}"),
    )
}

fn num(text: &str) -> Num {
    Num::parse_literal(text).unwrap()
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
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

/// Step `n` ticks, the first with `opening`; every machine's log lines of
/// those ticks, in order, as `(id, text)`.
fn run(sim: &mut Sim, opening: Vec<Command>, n: u64) -> Vec<(EntityId, String)> {
    let mut cmds = opening;
    for (i, c) in cmds.iter_mut().enumerate() {
        c.seq = i as u32;
    }
    let mut lines = Vec::new();
    for tick in 0..n {
        let report = sim.step(if tick == 0 { &cmds } else { &[] }, &[]);
        assert!(report.dropped.is_empty(), "dropped {:?}", report.dropped);
        for m in sim.world.machines.values() {
            lines.extend(m.log.iter().map(|l| (m.id, l.text.clone())));
        }
    }
    lines
}

fn lines_of(lines: &[(EntityId, String)], id: EntityId) -> Vec<&str> {
    lines
        .iter()
        .filter(|(who, _)| *who == id)
        .map(|(_, text)| text.as_str())
        .collect()
}

fn place(sim: &mut Sim, model: Model, team: TeamId, p: TilePos) -> EntityId {
    sim.world
        .place(model, team, p, model.deployment().map(str::to_string))
}

/// A bot of `team` on `p` holding `charge`, armed only if that is above
/// the low-power level, as a bot that got there by itself would be.
fn bot(sim: &mut Sim, team: TeamId, p: TilePos, charge: &str) -> EntityId {
    let id = place(sim, Model::Bot, team, p);
    let level = sim.world.data.machines.low_power_charge();
    let m = sim.world.machines.get_mut(&id).unwrap();
    m.charge = num(charge);
    m.armed = m.charge > level;
    id
}

fn machine(sim: &Sim, id: EntityId) -> &sim::world::Machine {
    &sim.world.machines[&id]
}

fn printer(sim: &Sim, team: TeamId) -> EntityId {
    sim.world
        .machines
        .values()
        .find(|m| m.team == team && m.model == Model::Printer)
        .unwrap()
        .id
}

// ------------------------------------------------------- a bot's battery ---

#[test]
fn a_call_takes_its_charge_cost_and_solar_gives_a_little_back() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "50");
    run(
        &mut s,
        vec![deploy(0, "bot", "log(len(see()))\nmove('e')\nwait(1000)\n")],
        1,
    );
    // The move took 1 as it began; `see`, `log` and `wait` cost nothing;
    // the tick's power step added the bot's solar, 0.1.
    assert_eq!(machine(&s, b).charge, num("49.1"));
    run(&mut s, vec![], 9);
    assert_eq!(machine(&s, b).pos, at(1, -2));
    assert_eq!(machine(&s, b).charge, num("50"));
    // A full battery takes no more: a bot of the team with no program.
    let full = bot(&mut s, T1, at(0, 2), "100");
    run(&mut s, vec![], 3);
    assert_eq!(machine(&s, full).charge, num("100"));
}

/// Q63: out of power is not "until the next call is covered". The bot is
/// flat from the call it could not pay for until its solar has brought it
/// to the wake level; then `on_low_power` runs, and then the call is made.
#[test]
fn a_bot_out_of_power_stays_flat_to_the_wake_level_then_runs_the_hook_then_its_call() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "0.5");
    let program =
        "log('start')\nmove('e')\nlog('moved')\nwait(1000)\ndef on_low_power():\n    log('hook')\n";
    let mut lines = run(&mut s, vec![deploy(0, "bot", program)], 1);
    assert!(machine(&s, b).flat, "a move costs 1 and it holds 0.5");
    // At tick 6 it holds 1.1, enough for the move, and is still flat.
    lines.extend(run(&mut s, vec![], 5));
    assert!(machine(&s, b).flat && machine(&s, b).charge > num("1"));
    assert_eq!(machine(&s, b).pos, at(0, -2));
    // 0.1 a tick from 0.5: it reaches the wake level, 20, at tick 195.
    lines.extend(run(&mut s, vec![], 188));
    assert!(machine(&s, b).flat, "tick 194: {:?}", machine(&s, b).charge);
    lines.extend(run(&mut s, vec![], 1));
    assert!(!machine(&s, b).flat, "tick 195 wakes it");
    assert_eq!(lines_of(&lines, b), ["start"]);
    lines.extend(run(&mut s, vec![], 4));
    assert_eq!(lines_of(&lines, b), ["start", "hook", "moved"]);
    assert_eq!(machine(&s, b).pos, at(1, -2));
}

/// Q63: `on_low_power` is held for power like main flow, and waking starts
/// it over from its first line, with main flow still set aside beneath it.
#[test]
fn a_hook_that_runs_out_of_power_starts_over_when_the_bot_wakes() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "1.5");
    // Armed, though low: the first call's cost is what trips it.
    s.world.machines.get_mut(&b).unwrap().armed = true;
    let program = "n = 7\nmove('e')\nlog('main goes on', n)\nwait(1000)\ndef on_low_power():\n    log('hook starts')\n    move('w')\n    log('hook done')\n";
    let mut lines = run(&mut s, vec![deploy(0, "bot", program)], 4);
    // The move began on 1.5 and left 0.5, at the level: `low_power`. It
    // waited for the move; then the hook's own move found the bot short.
    assert_eq!(lines_of(&lines, b), ["hook starts"]);
    assert_eq!(machine(&s, b).pos, at(1, -2));
    assert!(machine(&s, b).flat);
    lines.extend(run(&mut s, vec![], 196));
    assert!(!machine(&s, b).flat);
    assert_eq!(
        lines_of(&lines, b),
        ["hook starts", "hook starts", "hook done", "main goes on 7"],
        "the hook ran again from its top, and main flow kept its variables"
    );
    assert_eq!(machine(&s, b).pos, at(0, -2));
}

#[test]
fn on_fault_is_never_held_for_power_and_takes_what_there_is() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "0.5");
    let program = "x = 1 / 0\ndef on_fault(e):\n    move('e')\n    log('moved in on_fault')\n";
    let lines = run(&mut s, vec![deploy(0, "bot", program)], 3);
    assert_eq!(lines_of(&lines, b), ["moved in on_fault"]);
    assert_eq!(machine(&s, b).pos, at(1, -2));
    assert!(!machine(&s, b).flat);
    // The move took the 0.5 there was; three ticks of solar since.
    assert_eq!(machine(&s, b).charge, num("0.3"));
}

/// Q62, Q63: `low_power` comes once as the battery falls to the level,
/// waits for the action in progress, leaves main flow where it was, and
/// comes again only after the battery has been back above the level.
#[test]
fn low_power_comes_once_per_fall_waits_for_the_action_and_main_flow_goes_on() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "21.5");
    let program = "n = 0\nwhile True:\n    move('e' if n % 2 == 0 else 'w')\n    n += 1\n    log('moved', n)\ndef on_low_power():\n    log('hook at', me().pos[0], 'busy', me().busy)\n";
    let mut lines = run(&mut s, vec![deploy(0, "bot", program)], 12);
    // Each move takes 1 and its two ticks give 0.2 back: 20.7 after the
    // first, 19.9 after the second — which trips it as it begins, and the
    // hook runs once that move has landed, back at x = 0.
    assert_eq!(
        lines_of(&lines, b),
        [
            "moved 1",
            "hook at 0 busy None",
            "moved 2",
            "moved 3",
            "moved 4",
            "moved 5"
        ],
        "one hook, after the move that tripped it, and `n` kept counting"
    );
    assert!(!machine(&s, b).armed);
    lines.extend(run(&mut s, vec![], 20));
    assert_eq!(
        lines_of(&lines, b)
            .iter()
            .filter(|l| l.starts_with("hook"))
            .count(),
        1,
        "it stays low and is not interrupted again"
    );
    // Back above the level — as a recharge would leave it — it is armed
    // again, and the next fall trips it again.
    s.world.machines.get_mut(&b).unwrap().charge = num("21.5");
    s.world.charge_changed(b);
    assert!(machine(&s, b).armed);
    lines.extend(run(&mut s, vec![], 12));
    assert_eq!(
        lines_of(&lines, b)
            .iter()
            .filter(|l| l.starts_with("hook"))
            .count(),
        2
    );
}

#[test]
fn a_bot_with_no_hook_is_not_disturbed_by_low_power() {
    let program = "n = 0\nwhile True:\n    move('e' if n % 2 == 0 else 'w')\n    n += 1\n    log('moved', n)\n";
    let lines = |charge: &str| {
        let mut s = sim();
        let b = bot(&mut s, T0, at(0, -2), charge);
        let lines = run(&mut s, vec![deploy(0, "bot", program)], 12);
        let restarts = machine(&s, b).program.as_ref().unwrap().trace().restarts;
        (
            lines_of(&lines, b)
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
            restarts,
        )
    };
    // One falls through the level, the other stays well above it.
    assert_eq!(lines("21.5"), lines("60"));
}

// ------------------------------------------------------------- recharging ---

#[test]
fn a_bot_recharges_from_any_teams_building_at_that_buildings_rate() {
    let mut s = sim();
    let charger = place(&mut s, Model::Charger, T0, at(0, 0));
    s.world.machines.get_mut(&charger).unwrap().charge = num("120");
    // A rival's bot beside it, and one of its own.
    let rival = bot(&mut s, T1, at(1, 0), "30");
    let own = bot(&mut s, T0, at(-1, 0), "90");
    let program = "recharge()\nlog('charge', me().charge)\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![deploy(0, "bot", program), deploy(1, "bot", program)],
        3,
    );
    // A recharge takes two ticks and moves its charge as it completes,
    // in ascending id: the rival, placed first, takes the charger's rate,
    // 50; the other has room for only 9.9, a tick of solar behind it.
    // Each logs a tick later, with that tick's solar on top.
    assert_eq!(lines_of(&lines, rival), ["charge 80.2"]);
    assert_eq!(lines_of(&lines, own), ["charge 100"]);
    assert_eq!(machine(&s, rival).charge, num("80.3"));
    // 120 less 50 and 9.9, plus the charger's own 0.5 a tick for three.
    assert_eq!(machine(&s, charger).charge, num("61.6"));
}

#[test]
fn two_bots_recharging_from_one_building_are_served_lower_id_first() {
    let mut s = sim();
    let charger = place(&mut s, Model::Charger, T0, at(0, 0));
    s.world.machines.get_mut(&charger).unwrap().charge = num("60");
    let first = bot(&mut s, T0, at(1, 0), "10");
    let second = bot(&mut s, T0, at(-1, 0), "10");
    let lines = run(
        &mut s,
        vec![deploy(
            0,
            "bot",
            "recharge()\nlog('charge', me().charge)\nwait(1000)\n",
        )],
        3,
    );
    // The charger holds 60.5 when the transfers happen: the lower id
    // takes its rate, 50, and the other the 10.5 that is left.
    assert_eq!(lines_of(&lines, first), ["charge 60.2"]);
    assert_eq!(lines_of(&lines, second), ["charge 20.7"]);
    assert_eq!(
        machine(&s, charger).charge,
        num("1"),
        "two ticks of its own since"
    );
}

#[test]
fn recharge_refuses_when_there_is_nothing_to_take_or_no_room() {
    let mut s = sim();
    // An empty charger, a full bot beside a full printer, and a pylon,
    // which has no battery at all.
    place(&mut s, Model::Charger, T0, at(0, 0));
    let by_empty = bot(&mut s, T0, at(1, 0), "10");
    let full = bot(&mut s, T0, at(-4, 0), "100");
    place(&mut s, Model::Pylon, T0, at(3, 2));
    let by_pylon = bot(&mut s, T0, at(3, 1), "10");
    let program = "try:\n    recharge()\n    log('recharged')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let lines = run(&mut s, vec![deploy(0, "bot", program)], 3);
    for b in [by_empty, full, by_pylon] {
        assert_eq!(lines_of(&lines, b), ["refused"], "{b:?}");
    }
}

// ------------------------------------------------------------------ grids ---

fn grid_of(sim: &Sim, id: EntityId) -> Option<EntityId> {
    machine(sim, id).grid
}

#[test]
fn a_match_starts_with_its_printer_full_powered_and_a_grid_of_one() {
    let s = sim();
    let p = printer(&s, T0);
    let m = machine(&s, p);
    assert!(m.powered);
    assert_eq!(m.grid, Some(p));
    assert_eq!(m.charge, num("500"));
    let grids = &s.world.teams[&T0].grids;
    assert_eq!(grids.len(), 1);
    let g = &grids[0];
    assert_eq!(
        (g.grid, g.generation, g.drain, g.stored, g.capacity),
        (p, num("6"), num("1"), num("500"), num("500"))
    );
}

#[test]
fn buildings_join_a_grid_by_touching_by_wire_and_by_a_pylons_span() {
    let mut s = sim();
    let p = printer(&s, T0);
    // Touching the printer.
    let beside = place(&mut s, Model::Solar, T0, at(-5, -1));
    // Three tiles off: its own grid until wire reaches it.
    let far = place(&mut s, Model::Charger, T0, at(-1, 0));
    // Further still, with a pylon between it and `far`, within its span
    // of both — rock or no rock.
    let pylon = place(&mut s, Model::Pylon, T0, at(1, 2));
    let beyond = place(&mut s, Model::Solar, T0, at(4, 2));
    run(&mut s, vec![], 1);
    assert_eq!(grid_of(&s, beside), Some(p));
    assert_eq!(grid_of(&s, far), Some(far));
    assert_eq!(
        grid_of(&s, pylon),
        Some(far),
        "(1, 2) to (-1, 0) is within 4"
    );
    assert_eq!(
        grid_of(&s, beyond),
        Some(far),
        "(1, 2) to (4, 2) is within 4"
    );
    assert_eq!(s.world.teams[&T0].grids.len(), 2);
    // Wire from the printer's east side to the charger: one grid, labelled
    // by its oldest building.
    for x in -4..=-2 {
        assert!(s.world.lay_wire(T0, at(x, 0)));
    }
    run(&mut s, vec![], 1);
    for b in [beside, far, pylon, beyond] {
        assert_eq!(grid_of(&s, b), Some(p));
    }
    let snap = s.snapshot();
    let team = snap.teams.iter().find(|t| t.id == T0).unwrap();
    assert_eq!(team.grids.len(), 1);
    assert_eq!(
        team.wires,
        [(at(-4, 0), true), (at(-3, 0), true), (at(-2, 0), true)]
    );
    // A rival's wire on the same tiles joins nothing of this team's, and
    // wire that reaches no building is part of no grid.
    assert!(s.world.lay_wire(T1, at(-3, 0)));
    assert!(s.world.lay_wire(T0, at(0, -3)));
    // Cut the line by unwiring one tile, and the pylon's by removing it.
    assert!(s.world.remove_wire(T0, at(-3, 0)));
    s.world.remove(pylon);
    run(&mut s, vec![], 1);
    assert_eq!(grid_of(&s, far), Some(far));
    assert_eq!(grid_of(&s, beyond), Some(beyond));
    assert_eq!(grid_of(&s, beside), Some(p));
    let snap = s.snapshot();
    let team = snap.teams.iter().find(|t| t.id == T0).unwrap();
    assert_eq!(team.grids.len(), 3);
    assert_eq!(
        team.wires,
        [(at(-4, 0), true), (at(-2, 0), true), (at(0, -3), false)]
    );
}

#[test]
fn a_grids_surplus_fills_its_batteries_oldest_first_and_the_rest_is_lost() {
    let mut s = sim();
    let p = printer(&s, T0);
    let charger = place(&mut s, Model::Charger, T0, at(-5, -1));
    let panel = place(&mut s, Model::Solar, T0, at(-4, -1));
    s.world.machines.get_mut(&p).unwrap().charge = num("499");
    // The grid is served before the tick's prints begin, so the printer
    // needs only its drain: it keeps 1 of its 6. With the charger's 0.5
    // and the panel's 3 the pool is 8.5, which fills the printer's last 1
    // first, as the oldest, and then the charger.
    run(&mut s, vec![], 1);
    assert_eq!(machine(&s, p).charge, num("500"));
    assert_eq!(machine(&s, charger).charge, num("7.5"));
    assert_eq!(machine(&s, panel).charge, Num::ZERO, "a panel holds none");
    let g = s.world.teams[&T0].grids[0].clone();
    assert_eq!(g.grid, p);
    assert_eq!(g.generation, num("9.5"));
    assert_eq!(
        g.drain,
        num("1"),
        "kept as it stood when the grid was served"
    );
    assert_eq!(g.stored, num("507.5"));
    assert_eq!(g.capacity, num("1500"));
    // A charger already full: the surplus has nowhere to go.
    s.world.machines.get_mut(&charger).unwrap().charge = num("1000");
    run(&mut s, vec![], 5);
    assert_eq!(machine(&s, charger).charge, num("1000"));
}

/// `docs/08`, The power step: a building short of its need is paid from
/// the pool and then the grid's batteries, oldest first — or is dark and
/// takes nothing, and the next is tried.
#[test]
fn a_short_grid_serves_the_oldest_first_and_tries_the_next_when_one_does_not_fit() {
    // Chargers that drain 4, and depots that drain 1.
    let mut s = sim_with(&[
        (charger_drains("4").0, &charger_drains("4").1),
        (
            "# depots do not get dearer (Q42)\ncost_step = { ore = 0 }",
            "# depots do not get dearer (Q42)\ncost_step = { ore = 0 }\ndrain = 1",
        ),
    ]);
    // A grid of its own, away from the printer: two chargers, a depot and
    // a panel in a row. The panel makes 3 and each charger 0.5.
    let older = place(&mut s, Model::Charger, T0, at(0, -2));
    let newer = place(&mut s, Model::Charger, T0, at(1, -2));
    let depot = place(&mut s, Model::Depot, T0, at(2, -2));
    place(&mut s, Model::Solar, T0, at(3, -2));
    run(&mut s, vec![], 1);
    // Pool 3; the older charger lacks 3.5 and there is no charge stored:
    // dark, and its own 0.5 joins the pool. The newer lacks 3.5 of 3.5:
    // powered. The depot lacks 1 of nothing left: dark.
    assert!(!machine(&s, older).powered, "the oldest did not fit");
    assert!(machine(&s, newer).powered, "the next was tried, and did");
    assert!(!machine(&s, depot).powered);
    assert_eq!(grid_of(&s, depot), Some(older));
    // With charge in a battery the oldest is served first, from it.
    s.world.machines.get_mut(&newer).unwrap().charge = num("10");
    run(&mut s, vec![], 1);
    assert!(machine(&s, older).powered);
    assert!(machine(&s, newer).powered);
    assert!(machine(&s, depot).powered);
    // Needs 4 + 4 + 1 against 4 made: 5 came out of the battery.
    assert_eq!(machine(&s, newer).charge, num("5"));
    assert_eq!(machine(&s, older).charge, Num::ZERO);
}

/// Cutting a line darkens what the rest of the grid was covering: by a
/// pylon destroyed, which a rival can do, and by the team's own `unwire`.
#[test]
fn a_grid_cut_by_a_lost_pylon_or_by_unwire_darkens_what_it_was_covering() {
    let mut s = sim_with(&[(charger_drains("2").0, &charger_drains("2").1)]);
    // Two chargers, each needing 1.5 more than it makes, each fed by a
    // panel: one through a pylon's span, one along two tiles of wire.
    let by_pylon = place(&mut s, Model::Charger, T0, at(-2, -2));
    let pylon = place(&mut s, Model::Pylon, T0, at(0, -2));
    place(&mut s, Model::Solar, T0, at(2, -2));
    let by_wire = place(&mut s, Model::Charger, T0, at(-2, 2));
    place(&mut s, Model::Solar, T0, at(1, 2));
    for x in [-1, 0] {
        assert!(s.world.lay_wire(T0, at(x, 2)));
    }
    run(&mut s, vec![], 3);
    assert!(machine(&s, by_pylon).powered && machine(&s, by_wire).powered);
    // A rival's bot beside the pylon takes it down.
    let rival = bot(&mut s, T1, at(0, -3), "100");
    run(
        &mut s,
        vec![deploy(1, "bot", "deconstruct(0, -2)\nwait(1000)\n")],
        5,
    );
    assert!(
        !s.world.machines.contains_key(&pylon),
        "{rival:?} cut the line"
    );
    // And the team's own bot takes up a tile of the wire.
    let own = bot(&mut s, T0, at(0, 1), "100");
    run(
        &mut s,
        vec![deploy(0, "bot", "unwire(0, 2)\nwait(1000)\n")],
        4,
    );
    assert!(!s.world.has_wire(T0, at(0, 2)), "{own:?} took it up");
    // Each charger had banked a little of the panels' surplus; give that
    // time to run out, and both are dark.
    for id in [by_pylon, by_wire] {
        s.world.machines.get_mut(&id).unwrap().charge = Num::ZERO;
    }
    run(&mut s, vec![], 1);
    assert!(!machine(&s, by_pylon).powered, "nothing covers it now");
    assert!(!machine(&s, by_wire).powered);
    assert_eq!(grid_of(&s, by_pylon), Some(by_pylon));
    assert_eq!(grid_of(&s, by_wire), Some(by_wire));
}

#[test]
fn a_dark_building_still_fills_its_battery_and_lights_again_by_itself() {
    let mut s = sim_with(&[(charger_drains("2").0, &charger_drains("2").1)]);
    // Alone, it makes 0.5 against a drain of 2: dark three ticks in four,
    // while its own solar fills its battery to the 1.5 it lacks, which
    // then pays for one lit tick and leaves it empty.
    let c = place(&mut s, Model::Charger, T0, at(0, -2));
    let mut lit = Vec::new();
    for _ in 0..8 {
        run(&mut s, vec![], 1);
        lit.push(machine(&s, c).powered);
    }
    assert_eq!(
        lit,
        [false, false, false, true, false, false, false, true],
        "it holds 0, 0.5, 1 as it is served, then 1.5: lit, and empty again"
    );
}

/// `docs/08`, A dark building: its program and its senses stop, and it
/// can no longer be recharged from; its handlers still run.
#[test]
fn a_dark_building_runs_no_main_flow_sees_nothing_and_gives_no_charge() {
    let mut s = sim_with(&[(charger_drains("50").0, &charger_drains("50").1)]);
    // In the far corner from everything of team 0's, so the tiles round
    // it are seen by it alone. Its battery pays for one tick.
    let c = place(&mut s, Model::Charger, T0, at(6, 3));
    s.world.machines.get_mut(&c).unwrap().charge = num("60");
    let beside = bot(&mut s, T1, at(5, 3), "10");
    let program = "while True:\n    log('charger ticks')\n    wait(1)\n";
    let bot_program = "wait(3)\ntry:\n    recharge()\n    log('recharged')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let own = at(6, 2);
    let idx = s.world.tile_index(own).unwrap();
    let mut lines = run(
        &mut s,
        vec![deploy(0, "charger", program), deploy(1, "bot", bot_program)],
        1,
    );
    // Placed powered, and lit for a tick on its battery: 49.5 to pay.
    assert!(machine(&s, c).powered);
    assert_eq!(
        s.world.teams[&T0].memory[idx].state,
        TileState::Visible,
        "it sees its own neighbourhood while lit"
    );
    lines.extend(run(&mut s, vec![], 5));
    // The battery could not pay a second time: dark from tick 2's power
    // step, so its program ran in ticks 1 and 2 and not after.
    assert!(!machine(&s, c).powered);
    assert_eq!(lines_of(&lines, c), ["charger ticks", "charger ticks"]);
    assert_eq!(s.world.teams[&T0].memory[idx].state, TileState::Remembered);
    assert_eq!(lines_of(&lines, beside), ["refused"]);
    // Its own team still lists it, though nothing of the team sees it.
    let seen = "log(len([m for m in see() if m.model == 'charger']))\nwait(1000)\n";
    let lines = run(&mut s, vec![deploy(0, "printer", seen)], 1);
    assert_eq!(lines_of(&lines, printer(&s, T0)), ["1"]);
    // A handler runs whatever the power: `dying` ends in `death`.
    let health = machine(&s, c).health;
    s.world.set_health(c, Num::ZERO);
    s.world
        .machines
        .get_mut(&c)
        .unwrap()
        .program
        .as_mut()
        .unwrap()
        .raise(lang::Interrupt::Dying);
    assert!(health > Num::ZERO);
    run(&mut s, vec![], 2);
    assert!(!s.world.machines.contains_key(&c), "it died dark");
}

#[test]
fn a_printer_is_never_dark_and_powers_its_own_prints() {
    let mut s = sim();
    let p = printer(&s, T0);
    for tick in 0..120 {
        run(&mut s, vec![], 1);
        assert!(machine(&s, p).powered, "tick {tick}");
    }
    let bots = s
        .world
        .machines
        .values()
        .filter(|m| m.team == T0 && m.model == Model::Bot)
        .count();
    assert!(bots >= 3, "it printed {bots} bots in 120 ticks");
    // It makes more than its drain and a print's together, so its battery
    // stays full.
    assert_eq!(machine(&s, p).charge, num("500"));
    assert!(
        s.world
            .machines
            .values()
            .filter(|m| m.model == Model::Bot)
            .all(|m| m.charge == num("100") && m.armed && !m.flat),
        "a printed bot starts full and armed"
    );
}

// ------------------------------------------------------- pylons and wire ---

#[test]
fn a_bot_builds_a_pylon_which_runs_no_program_and_a_charger_which_does() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, 0), "100");
    s.world
        .machines
        .get_mut(&b)
        .unwrap()
        .load
        .insert("ore".into(), num("10"));
    let program = "build('pylon', 1, 0)\ndrop('ore')\nwait(1000)\n";
    run(&mut s, vec![deploy(0, "bot", program)], 12);
    let pylon = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Pylon)
        .expect("the pylon was built");
    assert_eq!(pylon.pos, at(1, 0));
    assert_eq!(pylon.deployment, None);
    assert!(pylon.program.is_none(), "a pylon runs no program");
    assert!(pylon.powered && pylon.grid == Some(pylon.id));
    assert_eq!(machine(&s, b).load["ore"], num("5"), "a pylon costs 5");
    // A deploy to `pylon` is refused; a charger has a deployment.
    assert!(s.validate(&deploy(0, "pylon", "wait(1)\n")).is_err());
    let c = place(&mut s, Model::Charger, T0, at(0, 2));
    assert_eq!(machine(&s, c).deployment.as_deref(), Some("charger"));
    assert!(machine(&s, c).program.is_some());
}

/// Q62: wire is laid for ore by the team's own bots, consumes its plan,
/// and a second bot laying the same tile in the same tick pays nothing.
#[test]
fn two_bots_wiring_one_tile_pay_once_and_the_plan_is_consumed() {
    let mut s = sim();
    let a = bot(&mut s, T0, at(-1, 0), "100");
    let b = bot(&mut s, T0, at(1, 0), "100");
    for id in [a, b] {
        s.world
            .machines
            .get_mut(&id)
            .unwrap()
            .load
            .insert("ore".into(), num("4"));
    }
    // One tick first: a wire plan goes only on ground the team has seen.
    run(&mut s, vec![], 1);
    let mark = Command::new(
        2,
        T0,
        0,
        CommandKind::Mark {
            at: at(0, 0),
            plan: PlanKind::Wire,
            value: Some("wire".into()),
        },
    );
    let program = "log(plans('wire'))\nwire(0, 0)\nlog('wired', tile(0, 0).wires, wires(), plans('wire'))\nwait(1000)\n";
    let lines = run(&mut s, vec![mark, deploy(0, "bot", program)], 4);
    assert!(s.world.has_wire(T0, at(0, 0)));
    assert!(
        s.world.plans(T0, at(0, 0)).is_none(),
        "the plan is consumed"
    );
    assert_eq!(
        lines_of(&lines, a),
        ["[(0, 0, True)]", "wired [0] [(0, 0)] []"]
    );
    assert_eq!(
        (ore(&s, a), ore(&s, b)),
        (num("1"), num("4")),
        "the lower id paid 3"
    );
    // Laying where the team's wire lies, and wiring without the ore, are
    // refused; a rival may lay its own on the same tile.
    let c = bot(&mut s, T1, at(0, 1), "100");
    s.world
        .machines
        .get_mut(&c)
        .unwrap()
        .load
        .insert("ore".into(), num("3"));
    let again = "try:\n    wire(0, 0)\n    log('laid')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![deploy(0, "bot", again), deploy(1, "bot", again)],
        4,
    );
    assert_eq!(lines_of(&lines, a), ["refused"], "its team's wire is there");
    assert_eq!(lines_of(&lines, b), ["refused"]);
    assert_eq!(lines_of(&lines, c), ["laid"]);
    assert_eq!(
        s.world.tile(at(0, 0)).unwrap().wires,
        [T0, T1].into_iter().collect()
    );
    // Only its own team removes a wire, with nothing refunded.
    let take_up = "try:\n    unwire(0, 0)\n    log('gone')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    s.world.remove_wire(T1, at(0, 0));
    let lines = run(
        &mut s,
        vec![deploy(0, "bot", take_up), deploy(1, "bot", take_up)],
        4,
    );
    assert_eq!(lines_of(&lines, c), ["refused"], "a rival cannot cut it");
    assert_eq!(lines_of(&lines, a), ["gone"]);
    assert!(!s.world.has_wire(T0, at(0, 0)));
    assert_eq!(ore(&s, a), num("1"));
}

fn ore(sim: &Sim, id: EntityId) -> Num {
    machine(sim, id).load["ore"]
}

/// `docs/03`: a wire plan goes only on ground the team has seen, and one
/// asking for what the tile already has is dropped on its tick.
#[test]
fn a_wire_plan_that_cannot_be_carried_out_is_dropped() {
    let mut s = sim();
    s.step(&[], &[]);
    let wire = |at: TilePos, lay: bool, tick: u64, seq: u32| {
        Command::new(
            tick,
            T0,
            seq,
            CommandKind::Mark {
                at,
                plan: PlanKind::Wire,
                value: lay.then(|| "wire".to_string()),
            },
        )
    };
    let seen = at(-4, 0);
    let unseen = at(6, 3);
    assert!(s.world.lay_wire(T0, at(-4, 1)));
    let report = s.step(
        &[
            wire(seen, true, 2, 0),
            // Still unknown to the team; and a clear where no wire lies;
            // and a lay where one does.
            wire(unseen, true, 2, 1),
            wire(at(-4, -1), false, 2, 2),
            wire(at(-4, 1), true, 2, 3),
            wire(at(-4, 1), false, 2, 4),
        ],
        &[],
    );
    let dropped: Vec<u32> = report.dropped.iter().map(|c| c.seq).collect();
    assert_eq!(dropped, [1, 2, 3]);
    assert_eq!(
        s.world.plans(T0, seen).unwrap().wire,
        Some(Plan::Set("wire".into()))
    );
    assert_eq!(
        s.world.plans(T0, at(-4, 1)).unwrap().wire,
        Some(Plan::Clear)
    );
    // A wire plan's value is `wire`, checked at submission.
    let bad = Command::new(
        3,
        T0,
        0,
        CommandKind::Mark {
            at: seen,
            plan: PlanKind::Wire,
            value: Some("red".into()),
        },
    );
    assert!(s.validate(&bad).is_err());
}

#[test]
fn a_team_that_is_out_loses_its_wires_and_its_grids() {
    let mut s = sim();
    assert!(s.world.lay_wire(T1, at(4, 0)));
    assert!(s.world.lay_wire(T0, at(4, 0)));
    s.step(&[Command::new(1, T1, 0, CommandKind::Resign)], &[]);
    assert!(!s.world.has_wire(T1, at(4, 0)));
    assert!(s.world.has_wire(T0, at(4, 0)), "another team's stays");
    assert!(s.world.teams[&T1].grids.is_empty());
    assert!(s.world.teams[&T1].wire_tiles.is_empty());
}

// ------------------------------------------------ what a program reads ---

#[test]
fn a_program_reads_its_charge_its_grid_and_the_teams_power() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, -2), "42");
    let bot_program = "m = me()\nlog(m.charge, m.battery, m.flat)\nwait(1000)\n";
    let printer_program = "m = me()\nlog(m.charge, m.battery, m.powered, m.grid == m.id)\nfor g in power():\n    log(g.grid == m.id, g.generation, g.drain, g.stored, g.capacity)\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![
            deploy(0, "bot", bot_program),
            deploy(0, "printer", printer_program),
        ],
        1,
    );
    assert_eq!(lines_of(&lines, b), ["42 100 False"]);
    assert_eq!(
        lines_of(&lines, printer(&s, T0)),
        ["500 500 True True", "True 6 1 500 500"]
    );
}

#[test]
fn the_state_hash_sees_a_wire_a_charge_and_a_flat_bot() {
    let base = || {
        let mut s = sim();
        let b = bot(&mut s, T0, at(0, -2), "50");
        (s, b)
    };
    let (mut plain, _) = base();
    plain.step(&[], &[]);
    let (mut wired, _) = base();
    wired.world.lay_wire(T0, at(0, 0));
    wired.step(&[], &[]);
    let (mut charged, b) = base();
    charged.world.machines.get_mut(&b).unwrap().charge = num("51");
    charged.step(&[], &[]);
    let (mut flat, b) = base();
    flat.world.machines.get_mut(&b).unwrap().flat = true;
    flat.step(&[], &[]);
    let hashes = [
        plain.state_hash(),
        wired.state_hash(),
        charged.state_hash(),
        flat.state_hash(),
    ];
    for (i, a) in hashes.iter().enumerate() {
        for b in &hashes[i.saturating_add(1)..] {
            assert_ne!(a, b, "{hashes:x?}");
        }
    }
}
