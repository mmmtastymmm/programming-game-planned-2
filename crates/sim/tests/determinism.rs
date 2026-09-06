//! Paired-run determinism: properties that must hold for *any* replay, not
//! just the one in the golden fixture.
//!
//! These are cheap and they never need regenerating, so they can be as noisy
//! as they like. `golden.rs` is the one that pins exact behavior.

use sim::map::{MapSpec, TilePos};
use sim::rng::Stream;
use sim::sim::{Command, CommandError};
use sim::world::EntityId;
use sim::{Replay, Sim, TimedCommand};

fn scenario(seed: u64) -> Replay {
    let mut spec = MapSpec::empty(10, 10);
    spec.seed = seed;
    spec.spawns.push(TilePos::new(0, 0));
    spec.spawns.push(TilePos::new(9, 9));
    let commands = vec![
        TimedCommand {
            tick: 1,
            command: Command::SetGoal {
                entity: EntityId(1),
                goal: Some(TilePos::new(8, 2)),
            },
        },
        TimedCommand {
            tick: 5,
            command: Command::Spawn {
                pos: TilePos::new(4, 4),
            },
        },
    ];
    Replay {
        spec,
        commands,
        ticks: 50,
    }
}

#[test]
fn same_replay_twice_gives_the_same_hash_stream() {
    let r = scenario(1234);
    assert_eq!(r.run(), r.run(), "a replay is not a function of its inputs");
}

#[test]
fn a_different_seed_gives_a_different_run() {
    // Not a determinism property as such — it is the guard that says the seed
    // is actually plumbed through. A sim that ignores its seed is perfectly
    // deterministic and completely broken.
    assert_ne!(
        scenario(1).run(),
        scenario(2).run(),
        "the match seed does not reach the sim"
    );
}

#[test]
fn hash_covers_rng_advance_not_just_visible_state() {
    // Two worlds whose entities sit on identical tiles but that have drawn a
    // different number of randoms have already desynced. If this fails, the
    // divergence will still surface — just later, somewhere unrelated.
    let spec = MapSpec::empty(4, 4);
    let mut a = Sim::new(&spec);
    let b = Sim::new(&spec);
    assert_eq!(a.state_hash(), b.state_hash());
    a.world.wander.next_u64();
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "state_hash ignores RNG stream state — desyncs will surface late and misattributed"
    );
}

#[test]
fn streams_with_similar_names_decorrelate() {
    // The scramble step in Stream::new exists for exactly this; without it,
    // adjacent names produce adjacent first draws.
    let mut a = Stream::new(7, "wander");
    let mut b = Stream::new(7, "wander2");
    let first: Vec<u64> = (0..4).map(|_| a.next_u64()).collect();
    let second: Vec<u64> = (0..4).map(|_| b.next_u64()).collect();
    assert_ne!(first, second);
}

#[test]
fn rejection_is_a_function_of_state() {
    // Every peer must refuse the same command on the same tick. The weaker
    // thing checked here — that rejection is decided by world state and leaves
    // the world untouched — is the part a test can reach.
    let mut sim = Sim::new(&MapSpec::empty(4, 4));
    let before = sim.state_hash();
    assert_eq!(
        sim.apply(&Command::Spawn {
            pos: TilePos::new(9, 9)
        }),
        Err(CommandError::OutOfBounds(TilePos::new(9, 9)))
    );
    assert_eq!(
        sim.apply(&Command::Despawn {
            entity: EntityId(42)
        }),
        Err(CommandError::NoSuchEntity(EntityId(42)))
    );
    assert_eq!(
        before,
        sim.state_hash(),
        "a refused command still changed the world"
    );
}

#[test]
#[should_panic(expected = "sorted by tick")]
fn out_of_order_commands_are_rejected_loudly() {
    let mut r = scenario(1);
    r.commands.swap(0, 1);
    r.run();
}

#[test]
// The distinctive half of the message, not a fragment any panic could carry.
#[should_panic(expected = ") is outside the")]
fn a_spawn_outside_the_map_is_rejected_at_construction() {
    // `Command::Spawn` refuses an off-map position, so `MapSpec::spawns` must
    // too — otherwise a replay carries an entity that consumes an id, enters the
    // state hash, and can never move, and the two entity-creation paths disagree
    // about the bounds rule.
    let mut spec = MapSpec::empty(4, 4);
    spec.spawns.push(TilePos::new(-5, 99));
    Sim::new(&spec);
}

#[test]
#[should_panic(expected = "ticks, over the")]
fn an_absurd_tick_count_is_rejected_before_allocating() {
    // `ticks` arrives from a .replay.ron, which is untrusted: the artifact is
    // what gets attached to a desync report. u64::MAX would ask the allocator
    // for ~147 exabytes.
    let mut r = scenario(1);
    r.ticks = u64::MAX;
    r.run();
}
