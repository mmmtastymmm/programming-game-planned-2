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
    let mut kinds: Vec<CommandKind> = tree
        .into_iter()
        .map(|(deployment, bundle)| CommandKind::Deploy { deployment, bundle })
        .collect();
    for (at, plan, value) in [
        (DEPOT, PlanKind::Building, "depot"),
        (RED, PlanKind::Paint, "red"),
        (LABEL, PlanKind::Overlay, "a"),
    ] {
        kinds.push(CommandKind::Mark {
            at,
            plan,
            value: Some(value.into()),
        });
    }
    kinds
        .into_iter()
        .enumerate()
        .map(|(i, k)| Command::new(0, TeamId(0), i as u32, k))
        .collect()
}

#[test]
fn the_starter_bot_builds_then_paints_then_patrols_and_never_faults() {
    let data = Data::load().unwrap();
    let mut sim = Sim::new(Map::parse(MAP, &data).unwrap());
    let mut done_at = None;
    let mut xs_after = Vec::new();
    for tick in 0..TICKS {
        sim.step(&if tick == 0 { opening() } else { vec![] });
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
