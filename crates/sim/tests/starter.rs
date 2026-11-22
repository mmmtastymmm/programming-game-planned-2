//! The starter programs in `data/starter`, run as the game runs them: a
//! team on the default tree builds what is planned, then paints what is
//! planned, then patrols, and never faults doing it. Nothing else runs
//! that tree in CI, so without this a broken starter ships silently.

use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, PlanKind, Sim, TeamId, TilePos};
use std::path::Path;

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

const TICKS: u64 = 3000;
const DEPOT: TilePos = TilePos { x: -1, y: -2 };
const RED: TilePos = TilePos { x: -2, y: -2 };
const LABEL: TilePos = TilePos { x: 0, y: -1 };

fn opening() -> Vec<Command> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/starter");
    let tree = sim::script::read_tree(&root).unwrap();
    tree.into_iter()
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

/// The plans, agreed for `tick`: after the team's first sight, since paint
/// and overlay go only on ground it has seen (Q49).
fn plans(tick: u64) -> Vec<Command> {
    [
        (DEPOT, PlanKind::Building, "depot"),
        (RED, PlanKind::Paint, "red"),
        (LABEL, PlanKind::Overlay, "a"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (at, plan, value))| {
        let kind = CommandKind::Mark {
            at,
            plan,
            value: Some(value.into()),
        };
        Command::new(tick, TeamId(0), i as u32, kind)
    })
    .collect()
}

#[test]
fn the_starter_bot_builds_then_paints_then_patrols_and_never_faults() {
    let data = Data::load().unwrap();
    let mut sim = Sim::new(Map::parse(MAP, &data).unwrap());
    let mut done_at = None;
    let mut xs_after = Vec::new();
    for tick in 0..TICKS {
        let commands = match tick {
            0 => opening(),
            2 => plans(sim.world.tick + 1),
            _ => vec![],
        };
        let report = sim.step(&commands, &[]);
        assert!(
            report.dropped.is_empty(),
            "tick {tick}: dropped {:?}",
            report.dropped
        );
        let snap = sim.snapshot();
        for m in &snap.machines {
            if m.record.team == TeamId(0) {
                assert!(
                    m.fault.is_none(),
                    "tick {tick}: {} faulted: {:?}",
                    m.record.name,
                    m.fault
                );
            }
        }
        let w = &sim.world;
        let built = w
            .machines
            .values()
            .any(|m| m.team == TeamId(0) && m.model == Model::Depot && m.pos == DEPOT);
        let tile = |p| w.tile(p).unwrap();
        let painted = tile(RED).paint.as_deref() == Some("red")
            && tile(LABEL).overlay.as_deref() == Some("a");
        if done_at.is_none() && built && painted {
            done_at = Some(tick);
        }
        if done_at.is_some() {
            xs_after.extend(
                w.machines
                    .values()
                    .filter(|m| m.team == TeamId(0) && m.model == Model::Bot)
                    .map(|m| m.pos.x),
            );
        }
    }
    let done_at = done_at.expect("the depot is built and the marks are made");
    for p in [DEPOT, RED, LABEL] {
        assert!(
            sim.world.plans(TeamId(0), p).is_none(),
            "{p:?}: every plan is consumed"
        );
    }
    // With nothing left to do the bots walk the loop: across the map, not
    // parked where the last job left them.
    let (lo, hi) = (xs_after.iter().min(), xs_after.iter().max());
    eprintln!("done at tick {done_at}; bots ranged over x {lo:?}..{hi:?}");
    assert!(
        lo.zip(hi).is_some_and(|(lo, hi)| hi - lo >= 4),
        "done at tick {done_at}; bots ranged over x {lo:?}..{hi:?}"
    );
}

/// Q62, Q63: on the default tree a planned panel and charger get built,
/// planned wire gets laid between them so the panel feeds the charger, and
/// the bots keep themselves charged: each goes to recharge at half a
/// battery, so none is ever out of power. A bot that does run low — one is
/// put down nearly empty — is taken to recharge by the starter's
/// `on_low_power` hook. Nobody faults, the hook's budget included.
#[test]
fn the_starter_bots_wire_a_panel_to_a_charger_and_recharge_when_low() {
    const CHARGER: TilePos = TilePos { x: -1, y: -2 };
    const WIRE: TilePos = TilePos { x: 0, y: -2 };
    const PANEL: TilePos = TilePos { x: 1, y: -2 };
    let data = Data::load().unwrap();
    let mut sim = Sim::new(Map::parse(MAP, &data).unwrap());
    let marks = |tick: u64| -> Vec<Command> {
        [
            (CHARGER, PlanKind::Building, "charger"),
            (PANEL, PlanKind::Building, "solar"),
            (WIRE, PlanKind::Wire, "wire"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (at, plan, value))| {
            let kind = CommandKind::Mark {
                at,
                plan,
                value: Some(value.into()),
            };
            Command::new(tick, TeamId(0), i as u32, kind)
        })
        .collect()
    };
    let mut recharges = 0u32;
    let mut hooks = 0u32;
    let mut before: std::collections::BTreeMap<sim::EntityId, lang::Num> = Default::default();
    let five = lang::Num::from_int(5).unwrap();
    for tick in 0..TICKS {
        let commands = match tick {
            0 => opening(),
            2 => marks(sim.world.tick + 1),
            _ => vec![],
        };
        if tick == 10 {
            // A bot a move or two above the low-power level, away from the
            // printer: its next moves trip the hook.
            let low = sim.world.place(
                Model::Bot,
                TeamId(0),
                TilePos { x: 2, y: 2 },
                Some("bot".into()),
            );
            let m = sim.world.machines.get_mut(&low).unwrap();
            m.charge = lang::Num::parse_literal("21.5").unwrap();
        }
        let report = sim.step(&commands, &[]);
        assert!(
            report.dropped.is_empty(),
            "tick {tick}: dropped {:?}",
            report.dropped
        );
        for m in sim.world.machines.values() {
            if m.team != TeamId(0) {
                continue;
            }
            assert!(!m.flat, "tick {tick}: {} is out of power", m.name);
            let program = m.program.as_ref();
            assert!(
                program.is_none_or(|p| p.fault_record.is_none()),
                "tick {tick}: {} faulted: {:?}",
                m.name,
                program.and_then(|p| p.fault_record.clone())
            );
            if m.model != Model::Bot {
                continue;
            }
            // A bot's own solar adds a tenth a tick; a jump is a recharge.
            if let Some(was) = before.insert(m.id, m.charge)
                && m.charge.checked_sub(was).is_ok_and(|gain| gain > five)
            {
                recharges += 1;
            }
            if program.is_some_and(|p| p.mode_name() == "low_power") {
                hooks += 1;
            }
        }
    }
    let w = &sim.world;
    let building = |model, at| {
        w.machines
            .values()
            .find(|m| m.team == TeamId(0) && m.model == model && m.pos == at)
    };
    let charger = building(Model::Charger, CHARGER).expect("the charger is built");
    let panel = building(Model::Solar, PANEL).expect("the panel is built");
    assert!(w.has_wire(TeamId(0), WIRE), "the planned wire is laid");
    assert!(w.plans(TeamId(0), WIRE).is_none(), "and its plan consumed");
    assert_eq!(charger.grid, panel.grid, "the wire joins them");
    assert!(charger.grid.is_some() && charger.powered && panel.powered);
    assert!(
        charger.charge > lang::Num::ZERO,
        "the panel's power reached the charger"
    );
    eprintln!("{recharges} recharges, {hooks} bot-ticks in on_low_power");
    assert!(hooks > 0, "no bot ever ran its low-power hook");
    assert!(
        recharges >= 3,
        "only {recharges} recharges in {TICKS} ticks"
    );
}
