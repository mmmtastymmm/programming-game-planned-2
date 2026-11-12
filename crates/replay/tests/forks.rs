//! The forks-and-watches gate (`docs/06`, Testing and Forks): watching
//! lines, stopping a fork and restoring one change nothing the hash
//! covers. Each test runs the golden replay another way and holds it to
//! the stream the golden test stored:
//!
//! - watched on every line a statement can begin on, through `sim.step`;
//! - on a fork stopped and resumed at every boundary of every machine;
//! - on a fork saved and restored at every tick, and on forks opened from
//!   states saved along the way;
//! - on the replay's playback.
//!
//! A run that hit no watch or stopped nowhere would hash as the stream
//! does and prove nothing, so each test also counts what it did.
//! `scripts/lib/rust-gates.mjs` seeds the two defects this gate exists
//! for: a watch that charges budget, and a resumed slice that charges its
//! deficit again.

use replay::Replay;
use sim::{CommandKind, Fork, Hit, Position, Sim, TeamId, Timeline, Watch};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// The stored golden replay and its stored hash stream.
fn stored() -> (Replay, Vec<u64>) {
    let dir = fixture_dir();
    let text = std::fs::read_to_string(dir.join("showcase.replay.ron")).expect("stored replay");
    let replay = Replay::from_ron(&text).expect("stored replay parses");
    let hashes: Vec<u64> = std::fs::read_to_string(dir.join("showcase.hashes.txt"))
        .expect("stored hashes")
        .lines()
        .map(|l| u64::from_str_radix(l, 16).expect("a hash line"))
        .collect();
    assert_eq!(
        u64::try_from(hashes.len()).ok(),
        Some(replay.ticks),
        "the stored stream should hold one hash per tick of the stored replay"
    );
    assert!(
        replay.ticks >= 100,
        "the golden replay is long enough to mean something"
    );
    (replay, hashes)
}

/// The stored hash at the end of `tick`.
fn at(hashes: &[u64], tick: u64) -> u64 {
    let i = usize::try_from(tick).ok().and_then(|t| t.checked_sub(1));
    *i.and_then(|i| hashes.get(i))
        .unwrap_or_else(|| panic!("no stored hash for tick {tick}"))
}

/// A watch on every line of every file of every bundle the replay
/// deploys on which a statement can begin, for the team that deploys it.
fn every_line(replay: &Replay) -> Vec<Watch> {
    let limits = lang::Limits::parse(lang::data::LIMITS_TOML).expect("limits");
    let mut watches = Vec::new();
    for c in &replay.commands {
        let CommandKind::Deploy { bundle, .. } = &c.kind else {
            continue;
        };
        let loaded = sim::load_bundle(bundle, &limits).expect("a replayed bundle loads");
        for (name, source) in &bundle.files {
            for line in loaded.program.begin_lines(name) {
                watches.push(Watch {
                    team: c.sender,
                    file: name.clone(),
                    content: lang::content_hash(source.as_bytes()),
                    line,
                    deployment: None,
                    machine: None,
                });
            }
        }
    }
    assert!(
        watches.len() > 50,
        "the replay's bundles have lines to watch"
    );
    watches
}

fn in_position_order(hits: &[Hit]) -> bool {
    hits.windows(2).all(|w| w[0].position < w[1].position)
}

#[test]
fn a_golden_replay_watched_on_every_line_gives_its_stream_unchanged() {
    let (replay, hashes) = stored();
    let watches = every_line(&replay);
    let playback = replay.playback(TeamId(0)).expect("the replay plays");
    let mut sim = Sim::new(playback.fork.world().map.clone());
    let mut firsts: u64 = 0;
    let mut rest: u64 = 0;
    let mut watched = Vec::new();
    for tick in 1..=replay.ticks {
        let report = sim.step(&playback.log.commands(tick), &watches);
        assert!(
            in_position_order(&report.hits),
            "tick {tick}: hits out of position order"
        );
        for hit in &report.hits {
            assert_eq!(hit.position.tick(), tick);
            assert!(
                matches!(hit.position, Position::Slice { machine, .. } if machine == hit.machine)
            );
            firsts = firsts.saturating_add(1);
            rest = rest.saturating_add(hit.more);
        }
        watched.push(sim.state_hash());
    }
    assert!(
        firsts > replay.ticks && rest > 0,
        "the watches hit too little to prove anything: {firsts} first beginnings, {rest} more"
    );
    assert!(
        watched == hashes,
        "watches changed the hash stream: a run watched on every line must hash as one that is not (docs/06, Watches). First difference at tick {:?}",
        watched
            .iter()
            .zip(&hashes)
            .position(|(a, b)| a != b)
            .map(|i| i.saturating_add(1))
    );
}

#[test]
fn a_fork_stopped_and_resumed_at_every_boundary_gives_the_stream() {
    let (replay, hashes) = stored();
    let mut playback = replay.playback(TeamId(0)).expect("the replay plays");
    let mut slices: u64 = 0;
    let mut last = playback.fork.position();
    while playback.tick() < replay.ticks {
        let position = playback.fork.nudge(&playback.log);
        assert!(
            position > last,
            "a nudge moved from {last:?} to {position:?}"
        );
        last = position;
        match position {
            Position::Slice { .. } => {
                slices = slices.saturating_add(1);
                assert_eq!(playback.fork.state_hash(), None, "no hash mid-tick");
            }
            Position::Damage { .. } => {}
            Position::End { tick } => {
                let hash = playback.fork.state_hash().expect("a hash at a tick's end");
                assert!(
                    hash == at(&hashes, tick),
                    "a fork stopped and resumed at every boundary left the hash stream at tick {tick} (docs/06, Stopping)"
                );
            }
        }
    }
    assert!(
        slices > replay.ticks.saturating_mul(4),
        "the fork stopped at too few boundaries to prove anything: {slices}"
    );
}

#[test]
fn a_fork_restored_from_each_saved_state_gives_the_stream() {
    let (replay, hashes) = stored();
    let playback = replay.playback(TeamId(0)).expect("the replay plays");
    let log = playback.log;
    let mut fork = playback.fork;
    let mut kept = Vec::new();
    // Restored at every tick: each tick runs on a fork opened from the
    // state the last one saved.
    for tick in 1..=replay.ticks {
        fork.advance(&[], &log);
        let saved = fork.save().expect("a state at a tick's end");
        assert_eq!(saved.tick(), tick);
        fork = Fork::open(&saved, TeamId(0), false);
        assert!(
            fork.state_hash() == Some(at(&hashes, tick)),
            "a fork restored from the state saved at tick {tick} left the hash stream (docs/06, Saved states)"
        );
        if tick % 25 == 0 {
            kept.push(saved.copy());
        }
    }
    assert!(kept.len() >= 4, "states were kept along the way");
    // And each kept state runs to the end on the stream, however long
    // ago it was saved.
    for saved in &kept {
        let mut fork = Fork::open(saved, TeamId(0), false);
        for tick in saved.tick().saturating_add(1)..=replay.ticks {
            fork.advance(&[], &log);
            assert!(
                fork.state_hash() == Some(at(&hashes, tick)),
                "a fork opened from the state saved at tick {} left the hash stream at tick {tick}",
                saved.tick()
            );
        }
    }
}

#[test]
fn a_fork_seeking_through_its_own_saved_states_gives_the_stream() {
    let (replay, hashes) = stored();
    let mut playback = replay.playback(TeamId(0)).expect("the replay plays");
    while playback.advance(&[]).is_some() {
        if playback.tick() % 60 == 0 {
            playback.fork.keep().expect("a state at a tick's end");
        }
    }
    let kept = playback.fork.kept();
    assert!(kept.len() >= 4, "states were kept along the way: {kept:?}");
    // Backwards between states, to ticks no state sits on, then forwards.
    for tick in kept.iter().rev().map(|t| t.saturating_add(7)) {
        playback.seek(tick);
        let tick = tick.min(replay.ticks);
        assert_eq!(playback.tick(), tick);
        assert!(
            playback.state_hash() == Some(at(&hashes, tick)),
            "a fork restored from its own saved state left the hash stream at tick {tick}"
        );
    }
    // With every state forgotten the fork re-runs from the map.
    for tick in kept {
        playback.fork.forget(tick);
    }
    playback.seek(3);
    assert_eq!(playback.state_hash(), Some(at(&hashes, 3)));
    playback.seek(replay.ticks);
    assert_eq!(playback.state_hash(), Some(at(&hashes, replay.ticks)));
}

#[test]
fn a_replays_playback_gives_the_stream_and_the_hits_sim_step_reports() {
    let (replay, hashes) = stored();
    let watches = every_line(&replay);
    let mut playback = replay.playback(TeamId(0)).expect("the replay plays");
    let mut sim = Sim::new(playback.fork.world().map.clone());
    let mut played = Vec::new();
    let mut hits: usize = 0;
    while let Some(report) = playback.advance(&watches) {
        let tick = playback.tick();
        // A fork takes its own team's watches; `sim.step` is given the
        // same ones and must report the same beginnings.
        let own: Vec<Watch> = watches
            .iter()
            .filter(|w| w.team == TeamId(0))
            .cloned()
            .collect();
        let stepped = sim.step(&playback.log.commands(tick), &own);
        assert_eq!(
            report, stepped,
            "tick {tick}: the playback's report is not sim.step's"
        );
        hits = hits.saturating_add(report.hits.len());
        played.push(playback.state_hash().expect("a hash at a tick's end"));
    }
    assert!(hits > 0, "the playback reported no hit");
    assert!(
        played == hashes,
        "a replay's playback left the hash stream: a fork runs the tick loop unchanged (docs/06, Forks)"
    );
    assert!(playback.done());
    assert_eq!(
        playback.advance(&[]),
        None,
        "a playback never runs past its replay"
    );
}

#[test]
fn a_replay_that_cannot_play_is_an_error_not_a_panic() {
    let (replay, _) = stored();
    let no_map = Replay {
        map: "not a map".into(),
        ..replay.clone()
    };
    let error = no_map
        .playback(TeamId(0))
        .err()
        .expect("a bad map is refused");
    assert!(error.contains("map does not load"), "{error}");
    let too_long = Replay {
        ticks: replay::MAX_REPLAY_TICKS.saturating_add(1),
        ..replay
    };
    let error = too_long
        .playback(TeamId(0))
        .err()
        .expect("too many ticks is refused");
    assert!(error.contains("over the cap"), "{error}");
}
