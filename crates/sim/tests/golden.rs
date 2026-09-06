//! Stored golden-replay fixtures (CLAUDE.md).
//!
//! Unlike the paired-run tests in `determinism.rs`, these compare against
//! CHECKED-IN hashes, so any behavior change — deterministic or not — fails
//! CI until the fixture is regenerated on purpose:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test -p sim --test golden
//! ```
//!
//! A PR that regenerates the fixture must explain why (CLAUDE.md: "a PR that
//! changes a replay hash must explain why"). That sentence is the whole value
//! of the gate — a fixture that gets silently regenerated whenever it goes red
//! is worse than no fixture, because it looks like coverage.
//!
//! The scenario below is a PLACEHOLDER matching the placeholder sim. When the
//! design lands, replace it with one that exercises every `Command` variant,
//! every fault path, and at least one RNG-driven decision — the fixture is
//! only worth its bytes if a real behavior change moves a hash.

use std::path::PathBuf;

use sim::map::{MapSpec, TilePos};
use sim::sim::Command;
use sim::world::EntityId;
use sim::{Replay, TimedCommand};

/// The canonical golden scenario.
fn golden_replay() -> Replay {
    let mut spec = MapSpec::empty(14, 8);
    spec.seed = 0x5EED_601D; // fixed match seed for the fixture
    spec.spawns.push(TilePos::new(1, 1));
    spec.spawns.push(TilePos::new(2, 6));

    // Spawn order fixes the ids: (1,1) is 1, (2,6) is 2, and the mid-run
    // spawn below is 3.
    let commands = vec![
        TimedCommand {
            tick: 0,
            command: Command::SetGoal {
                entity: EntityId(1),
                goal: Some(TilePos::new(12, 6)),
            },
        },
        // Entity 2 gets no goal at all, so it draws from the wander stream
        // every tick — the fixture covers the RNG path, not just pathing.
        TimedCommand {
            tick: 10,
            command: Command::Spawn {
                pos: TilePos::new(7, 4),
            },
        },
        TimedCommand {
            tick: 12,
            command: Command::SetGoal {
                entity: EntityId(3),
                goal: Some(TilePos::new(0, 0)),
            },
        },
        // Retarget mid-flight, then arrive and fall through to wandering.
        TimedCommand {
            tick: 30,
            command: Command::SetGoal {
                entity: EntityId(1),
                goal: Some(TilePos::new(3, 7)),
            },
        },
        TimedCommand {
            tick: 60,
            command: Command::Despawn {
                entity: EntityId(3),
            },
        },
    ];

    Replay {
        spec,
        commands,
        ticks: 120,
    }
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn hashes_to_text(hashes: &[u64]) -> String {
    let mut out = String::new();
    for h in hashes {
        out.push_str(&format!("{h:016x}\n"));
    }
    out
}

/// The fixture is only worth its bytes if the scenario genuinely exercises the
/// sim. Assert on the outcomes, so a scenario that quietly stops doing
/// anything (every entity despawned by a refactor, say) fails here rather than
/// passing as a stable hash of an empty world.
#[test]
fn golden_scenario_is_alive() {
    // Through Replay::execute, not a hand-rolled copy of its loop: a second
    // copy of the command boundary drifts silently, and this test would keep
    // asserting against the old semantics while the fixture regenerated under
    // the new ones.
    let (sim, _hashes) = golden_replay().execute();
    assert_eq!(
        sim.world.entities.len(),
        2,
        "one of the three spawns was despawned"
    );
    assert!(
        sim.world.entity(EntityId(1)).distance_travelled > 10,
        "the goal-seeking entity must actually travel; got {}",
        sim.world.entity(EntityId(1)).distance_travelled
    );
    assert!(
        sim.world.entity(EntityId(2)).distance_travelled > 0,
        "the wandering entity must have drawn a move from the RNG stream"
    );
    assert!(
        !sim.world.entities.contains_key(&EntityId(3)),
        "Despawn(3) must take effect"
    );
}

#[test]
fn golden_replay_round_trips_through_ron() {
    let replay = golden_replay();
    let text = replay.to_ron();
    let parsed = Replay::from_ron(&text).expect("fixture RON parses");
    assert_eq!(
        replay, parsed,
        "replay must survive serialization byte-exactly"
    );
}

#[test]
fn golden_replay_matches_stored_fixture() {
    let dir = fixture_dir();
    let replay_path = dir.join("showcase.replay.ron");
    let hashes_path = dir.join("showcase.hashes.txt");
    let replay = golden_replay();
    let hashes = replay.run();

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(&replay_path, replay.to_ron()).expect("write replay fixture");
        std::fs::write(&hashes_path, hashes_to_text(&hashes)).expect("write hash fixture");
        eprintln!("golden fixtures regenerated — explain the hash change in the PR (CLAUDE.md)");
        return;
    }

    // Two separate things can drift, and conflating them wastes an afternoon:
    // the in-code scenario vs. the stored artifact (someone edited the
    // scenario), and the stored hashes vs. a fresh run (sim behavior changed).
    let stored_replay = Replay::from_ron(
        &std::fs::read_to_string(&replay_path).expect("stored replay fixture exists"),
    )
    .expect("stored replay parses");
    assert_eq!(
        stored_replay, replay,
        "the in-code scenario and the stored artifact diverged — \
         regenerate with UPDATE_GOLDEN=1 and explain why in the PR"
    );
    let stored_hashes = std::fs::read_to_string(&hashes_path).expect("stored hashes exist");
    assert_eq!(
        stored_hashes,
        hashes_to_text(&hashes),
        "replay hash drift: sim behavior changed. If intentional, regenerate \
         fixtures with UPDATE_GOLDEN=1 and explain the change in the PR (CLAUDE.md)"
    );
}

/// Child half of the cross-process check: prints the final hash. Ignored in
/// normal runs; the parent test invokes it in a fresh process.
#[test]
#[ignore = "spawned by cross_process_replay_matches"]
fn emit_golden_final_hash() {
    let hashes = golden_replay().run();
    println!(
        "GOLDEN_FINAL_HASH={:016x}",
        hashes.last().expect("nonempty run")
    );
}

/// The actual lockstep guarantee: a SEPARATE PROCESS reaches bit-identical
/// state. Same-process pairs cannot catch the whole class — address-dependent
/// iteration order, a hasher seeded per process, anything that is stable
/// within a run and different between runs.
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
    let child_hash = stdout
        .lines()
        .find_map(|l| l.strip_prefix("GOLDEN_FINAL_HASH="))
        .unwrap_or_else(|| panic!("no hash line in child output: {stdout}"))
        .to_string();
    let local = golden_replay().run();
    let local_hash = format!("{:016x}", local.last().expect("nonempty run"));
    assert_eq!(
        child_hash, local_hash,
        "cross-process desync — determinism violation"
    );
}
