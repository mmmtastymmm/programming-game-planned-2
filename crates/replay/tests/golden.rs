//! The golden replay (CLAUDE.md; `docs/06`, Testing): `(map, command log)`
//! to a checked-in hash stream. Any behaviour change in `sim` or `lang` —
//! deterministic or not — fails here until the fixture is regenerated on
//! purpose:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test -p replay --test golden
//! ```
//!
//! and the PR explains the hash change. The scenario: the first map, a
//! player team running the bundles under `tests/golden/player/`, and the
//! Fool as the opposition, with a **mid-match redeploy** of the player's
//! `red` color (T7: the hash-affecting path most likely to differ between
//! peers), a speed change, and marks. The alive test asserts on what the
//! match must have done, so a run that did nothing cannot score green.

use replay::Replay;
use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, PlanKind, TeamId, TilePos};
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn bundle(dir: &Path) -> sim::Bundle {
    sim::script::read_bundle(dir).unwrap_or_else(|e| panic!("{e}"))
}

/// The canonical golden scenario.
fn golden_replay() -> Replay {
    let map_text = std::fs::read_to_string(data_dir().join("maps/first.toml")).expect("first map");
    let data = Data::load().expect("data");
    let map = Map::parse(&map_text, &data).expect("map");
    let player = TeamId(0);
    let fool = TeamId(1);
    let mut commands = Vec::new();
    // The player's opening set: one deploy per deployment, agreed for tick 0.
    let p = fixture_dir().join("player");
    commands.push(Command::new(
        0,
        player,
        0,
        CommandKind::Deploy {
            deployment: "printer".into(),
            bundle: bundle(&p.join("printer")),
        },
    ));
    commands.push(Command::new(
        0,
        player,
        1,
        CommandKind::Deploy {
            deployment: "red".into(),
            bundle: bundle(&p.join("red")),
        },
    ));
    // A paint plan and an overlay plan beside the start, for the marks path.
    let start = map.teams[0][0];
    commands.push(Command::new(
        0,
        player,
        2,
        CommandKind::Mark {
            at: TilePos::new(start.x, start.y.saturating_add(1)),
            plan: PlanKind::Paint,
            value: Some("red".into()),
        },
    ));
    commands.push(Command::new(
        0,
        player,
        3,
        CommandKind::Mark {
            at: TilePos::new(start.x.saturating_add(1), start.y),
            plan: PlanKind::Overlay,
            value: Some("a".into()),
        },
    ));
    // The Fool's script, tiles relative to its first start.
    let script =
        sim::script::Script::load(&data_dir().join("opposition/fool")).expect("the Fool's script");
    commands.extend(script.resolve(fool, map.teams[1][0]));
    // Mid-match: a speed change, an unmark, and the redeploy of `red`.
    commands.push(Command::new(40, player, 0, CommandKind::SetSpeed(2)));
    commands.push(Command::new(
        55,
        player,
        0,
        CommandKind::Unmark {
            at: TilePos::new(start.x.saturating_add(1), start.y),
            plan: PlanKind::Overlay,
        },
    ));
    commands.push(Command::new(
        90,
        player,
        0,
        CommandKind::Deploy {
            deployment: "red".into(),
            bundle: bundle(&p.join("red2")),
        },
    ));
    // A deploy to a locked color waits in its slot and never applies here.
    commands.push(Command::new(
        95,
        player,
        0,
        CommandKind::Deploy {
            deployment: "blue".into(),
            bundle: bundle(&p.join("red")),
        },
    ));
    Replay {
        map: map_text,
        commands,
        ticks: 220,
    }
}

fn hashes_to_text(hashes: &[u64]) -> String {
    hashes.iter().map(|h| format!("{h:016x}\n")).collect()
}

#[test]
fn golden_replay_matches_stored_fixture() {
    let dir = fixture_dir();
    let replay_path = dir.join("showcase.replay.ron");
    let hashes_path = dir.join("showcase.hashes.txt");
    let replay = golden_replay();
    let run = replay.execute();
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&replay_path, replay.to_ron()).expect("write replay fixture");
        std::fs::write(&hashes_path, hashes_to_text(&run.hashes)).expect("write hash fixture");
        eprintln!("golden fixtures regenerated — explain the hash change in the PR (CLAUDE.md)");
        return;
    }
    let stored_replay = Replay::from_ron(
        &std::fs::read_to_string(&replay_path).expect("stored replay fixture exists"),
    )
    .expect("stored replay parses");
    assert_eq!(
        stored_replay, replay,
        "the in-code scenario and the stored artifact diverged — regenerate with UPDATE_GOLDEN=1 and explain why in the PR"
    );
    let stored_hashes = std::fs::read_to_string(&hashes_path).expect("stored hashes exist");
    assert_eq!(
        stored_hashes,
        hashes_to_text(&run.hashes),
        "replay hash drift: sim behaviour changed. If intentional, regenerate with UPDATE_GOLDEN=1 and explain the change in the PR (CLAUDE.md)"
    );
}

/// The fixture is only worth its bytes if the match genuinely happened.
#[test]
fn golden_scenario_is_alive() {
    let replay = golden_replay();
    let run = replay.execute();
    assert!(
        run.dropped.is_empty(),
        "commands were dropped: {:?}",
        run.dropped
    );
    assert_eq!(
        run.hashes.len(),
        220,
        "the match ended early: {:?}",
        run.sim.ended()
    );
    assert!(
        run.hashes.windows(2).all(|w| w[0] != w[1]),
        "two consecutive ticks hashed the same"
    );
    let w = &run.sim.world;
    let bots = |t: u32| {
        w.machines
            .values()
            .filter(|m| m.team == TeamId(t) && m.model == Model::Bot)
            .count()
    };
    assert!(bots(0) >= 2, "the player printed {} bots", bots(0));
    assert!(bots(1) >= 2, "the Fool printed {} bots", bots(1));
    assert!(
        w.machines
            .values()
            .any(|m| m.team == TeamId(1) && matches!(m.model, Model::Depot | Model::Site)),
        "the Fool placed no site"
    );
    assert!(
        w.machines.values().any(|m| m.team == TeamId(0)
            && m.deployment.as_deref() == Some("red")
            && m.program
                .as_ref()
                .is_some_and(|p| p.global("guard").is_some())),
        "no player bot took the redeploy"
    );
    let ore_moved = w.tiles.iter().any(|t| {
        t.deposit
            .as_ref()
            .is_some_and(|d| d.amount["ore"] < d.cap["ore"])
    }) || w.machines.values().any(|m| {
        m.load.values().any(|v| *v > lang::Num::ZERO)
            || m.store.values().any(|v| *v > lang::Num::ZERO)
    });
    assert!(ore_moved, "no ore was ever picked");
    assert_eq!(w.speed, 2, "the SetSpeed did not land");
    assert!(
        w.teams[&TeamId(0)].deployments["blue"].pending.is_some(),
        "the deploy to the locked color did not wait"
    );
    assert!(w.teams[&TeamId(0)].deployments["blue"].bundle.is_none());
    let remembered = w.teams[&TeamId(0)]
        .memory
        .iter()
        .filter(|m| m.state != sim::world::TileState::Unknown)
        .count();
    assert!(
        remembered > 20,
        "the player's memory holds {remembered} tiles"
    );
}

#[test]
fn golden_replay_round_trips_through_ron() {
    let replay = golden_replay();
    let parsed = Replay::from_ron(&replay.to_ron()).expect("fixture RON parses");
    assert_eq!(replay, parsed);
    assert_eq!(replay.identity(), parsed.identity());
}

/// Child half of the cross-process check; the parent spawns it.
#[test]
#[ignore = "spawned by cross_process_replay_matches"]
fn emit_golden_final_hash() {
    let hashes = golden_replay().run();
    println!(
        "GOLDEN_FINAL_HASH={:016x}",
        hashes.last().expect("nonempty run")
    );
}

/// The lockstep guarantee: a SEPARATE PROCESS reaches the same state.
#[test]
fn cross_process_replay_matches() {
    let exe = std::env::current_exe().expect("test binary path");
    let output = std::process::Command::new(exe)
        .args([
            "--ignored",
            "--exact",
            "emit_golden_final_hash",
            "--nocapture",
        ])
        .env_remove("UPDATE_GOLDEN")
        .output()
        .expect("spawn child test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child process failed: {stdout}");
    let child = stdout
        .lines()
        .find_map(|l| l.strip_prefix("GOLDEN_FINAL_HASH="))
        .unwrap_or_else(|| panic!("no hash line in child output: {stdout}"))
        .to_string();
    let local = format!(
        "{:016x}",
        golden_replay().run().last().expect("nonempty run")
    );
    assert_eq!(child, local, "cross-process desync — determinism violation");
}
