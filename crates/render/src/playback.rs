//! A replay as its viewer plays it (`docs/06`, The driver; `docs/07`, The
//! match and Saved states): the replay's playback, the states it saves for
//! seeking and for forks, and what opening it and finishing it report.
//! Plain Rust, so it is tested headless; [`crate::driver`] steps it at the
//! viewer's pace.
//!
//! A playback is a fork (`docs/06`, Forks), never the match: nothing here
//! sends a command or reaches a match's world, log or hash.

use crate::palette::Debugging;
use replay::{Playback, Replay, ReplayLog};
use sim::snapshot::Snapshot;
use sim::{Fork, StepReport, TeamId, Watch};
use std::collections::BTreeMap;

pub struct Viewer {
    replay: Replay,
    playback: Playback,
    /// Ticks between saved states: `debug_checkpoint_ticks` at first,
    /// doubled each time the count is reached.
    interval: u64,
    /// `debug_checkpoints_kept`.
    count: usize,
    /// Saved states an open fork or a listed hit needs, kept beyond the
    /// count, each with how many need it.
    pinned: BTreeMap<u64, u32>,
    /// What opening the replay warned of (Q60): another version wrote it.
    pub warning: Option<String>,
    /// What reaching the last tick reported (Q60): this build played it
    /// back differently.
    pub report: Option<String>,
}

impl Viewer {
    /// The replay's playback, standing before tick 1 with that state
    /// saved. A file that cannot play is an error.
    pub fn open(replay: Replay, tuning: Debugging) -> Result<Viewer, String> {
        let team = replay.team.unwrap_or(TeamId(0));
        let mut playback = replay.playback(team)?;
        if !playback.fork.snapshot().teams.iter().any(|t| t.id == team) {
            return Err(format!(
                "the replay was saved by team {}, and its map has no such team",
                team.0
            ));
        }
        playback.fork.keep()?;
        let mut viewer = Viewer {
            warning: replay.version_warning(),
            replay,
            playback,
            interval: tuning.debug_checkpoint_ticks.max(1),
            count: tuning.debug_checkpoints_kept.max(2),
            pinned: BTreeMap::new(),
            report: None,
        };
        viewer.arrived();
        Ok(viewer)
    }

    /// The team whose view the playback is drawn in: the saver's, or the
    /// first team's when the replay records none.
    pub fn team(&self) -> TeamId {
        self.playback.fork.team()
    }

    /// Whether the replay records its team, and so can be debugged as
    /// that team rather than only watched (Q55).
    pub fn can_debug(&self) -> bool {
        self.replay.team.is_some()
    }

    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    /// The playback's fork, to read and to open a debugger's fork from.
    pub fn fork(&self) -> &Fork {
        &self.playback.fork
    }

    /// The replay's log, which a debugger's fork of the playback runs on.
    pub fn log(&self) -> &ReplayLog {
        &self.playback.log
    }

    /// The tick the playback stands at the end of.
    pub fn tick(&self) -> u64 {
        self.playback.tick()
    }

    /// The replay's last tick.
    pub fn ticks(&self) -> u64 {
        self.playback.ticks()
    }

    pub fn done(&self) -> bool {
        self.playback.done()
    }

    /// The winner, or `None` for a draw, once the playback's match is over.
    pub fn ended(&self) -> Option<Option<TeamId>> {
        self.playback.fork.ended()
    }

    pub fn state_hash(&self) -> u64 {
        self.playback
            .state_hash()
            .expect("a playback stands at a tick's end")
    }

    pub fn snapshot(&self) -> Snapshot {
        self.playback.fork.snapshot()
    }

    /// The ticks between saved states now.
    pub fn interval(&self) -> u64 {
        self.interval
    }

    /// The ticks a state is saved of.
    pub fn kept(&self) -> Vec<u64> {
        self.playback.fork.kept()
    }

    /// Play the next tick, reporting `watches`' hits as `sim.step` does.
    /// `None` once the playback is done.
    pub fn advance(&mut self, watches: &[Watch]) -> Option<StepReport> {
        let report = self.playback.advance(watches)?;
        self.arrived();
        Some(report)
    }

    /// Move toward the end of `tick`, running at most `budget` ticks, and
    /// say whether the playback stands there. It restores the latest saved
    /// state at or before the tick unless it already stands between that
    /// state and the tick, runs on without watches, and saves on the way
    /// as [`Viewer::advance`] does, so a long seek leaves states behind it.
    pub fn seek_some(&mut self, tick: u64, budget: u64) -> bool {
        let target = tick.min(self.ticks());
        let from = self
            .kept()
            .into_iter()
            .rev()
            .find(|s| *s <= target)
            .unwrap_or(0);
        if self.tick() > target || self.tick() < from {
            // The saved state is there, so this restores and runs nothing.
            self.playback.seek(from);
        }
        let mut left = budget;
        while self.tick() < target && left > 0 && self.ended().is_none() {
            let here = self.tick();
            let checkpoint = (here / self.interval + 1).saturating_mul(self.interval);
            let next = target.min(checkpoint).min(here.saturating_add(left));
            self.playback.seek(next);
            let ran = self.tick().saturating_sub(here);
            if ran == 0 {
                break;
            }
            left = left.saturating_sub(ran);
            self.arrived();
        }
        self.tick() >= target || self.ended().is_some()
    }

    /// Stand at the end of `tick`, or of the last tick if that is earlier.
    pub fn seek(&mut self, tick: u64) {
        while !self.seek_some(tick, u64::MAX) {}
    }

    /// Save the state the playback stands at now, for a fork opened by
    /// hand, and keep it for as long as that fork is open.
    pub fn keep_now(&mut self) -> Result<u64, String> {
        let tick = self.playback.fork.keep()?;
        self.pin(tick);
        Ok(tick)
    }

    /// The latest saved state at or before `tick`, kept from now until
    /// [`Viewer::unpin`]: the one an open fork or a listed hit needs.
    pub fn pin_before(&mut self, tick: u64) -> u64 {
        let from = self
            .kept()
            .into_iter()
            .rev()
            .find(|s| *s <= tick)
            .unwrap_or(0);
        self.pin(from);
        from
    }

    fn pin(&mut self, tick: u64) {
        let n = self.pinned.entry(tick).or_insert(0);
        *n = n.saturating_add(1);
    }

    /// The fork or the hit that needed the state saved at `tick` is gone:
    /// the state falls back under the count, and one no interval asks for
    /// is dropped at once.
    pub fn unpin(&mut self, tick: u64) {
        let Some(n) = self.pinned.get_mut(&tick) else {
            return;
        };
        *n = n.saturating_sub(1);
        if *n == 0 {
            self.pinned.remove(&tick);
            if !self.regular(tick) && tick != 0 {
                self.playback.fork.forget(tick);
            }
            self.thin();
        }
    }

    /// Whether the interval asks for a state at `tick`. Tick 0's is kept
    /// beside the count.
    fn regular(&self, tick: u64) -> bool {
        tick != 0 && tick.is_multiple_of(self.interval)
    }

    /// The playback stands at a new tick: save it if the interval asks,
    /// thin the saved states, and compare the final hash at the end.
    fn arrived(&mut self) {
        let tick = self.tick();
        if self.regular(tick) && !self.kept().contains(&tick) {
            let _ = self.playback.fork.keep();
            self.thin();
        }
        if self.done() && self.report.is_none() {
            self.report = self.replay.final_hash_report(self.state_hash());
        }
    }

    /// Once the states the interval asked for number the count, drop
    /// every other one and double the interval (`docs/07`, Saved states),
    /// so they stay spread over the replay. A pinned state stays.
    fn thin(&mut self) {
        loop {
            let regular: Vec<u64> = self
                .kept()
                .into_iter()
                .filter(|t| self.regular(*t))
                .collect();
            if regular.len() < self.count {
                return;
            }
            self.interval = self.interval.saturating_mul(2);
            for tick in self.kept() {
                if tick != 0 && !self.regular(tick) && !self.pinned.contains_key(&tick) {
                    self.playback.fork.forget(tick);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use sim::{CommandKind, bundle_of};

    fn tuning() -> Debugging {
        Debugging {
            debug_checkpoint_ticks: 5,
            debug_checkpoints_kept: 4,
            debug_step_max_ticks: 100,
            debug_hits_kept: 5,
        }
    }

    /// A match against the Fool run `ticks` ticks, as the replay it saves.
    fn recorded(ticks: u64) -> Replay {
        let map = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/maps/first.toml"
        ))
        .unwrap();
        let fool = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/opposition/fool"
        ));
        let mut d = Driver::new(
            &map,
            &[&fool],
            vec![CommandKind::Deploy {
                deployment: "printer".into(),
                bundle: bundle_of("while True:\n    wait(5)\n"),
            }],
        )
        .unwrap();
        while d.tick < ticks {
            d.advance(0.1);
        }
        assert_eq!(d.tick, ticks);
        d.replay()
    }

    fn play_to(v: &mut Viewer, tick: u64) {
        while v.tick() < tick {
            v.advance(&[]).expect("the replay runs this far");
        }
    }

    #[test]
    fn the_playback_reproduces_the_replay_and_stops_at_its_last_tick() {
        let replay = recorded(30);
        let stream = replay.run();
        assert_eq!(stream.len(), 30);
        let mut v = Viewer::open(replay, tuning()).unwrap();
        assert_eq!((v.tick(), v.ticks()), (0, 30));
        for hash in &stream {
            assert!(!v.done());
            v.advance(&[]).expect("a tick to play");
            assert_eq!(v.state_hash(), *hash, "tick {}", v.tick());
        }
        assert!(v.done());
        assert_eq!(v.advance(&[]), None, "it never runs past the replay");
        assert_eq!(v.tick(), 30);
        assert_eq!(v.snapshot().tick, 30);
        // It ended on the hash it recorded, in the version that wrote it.
        assert_eq!((v.warning.clone(), v.report.clone()), (None, None));
    }

    #[test]
    fn saved_states_stay_spread_over_the_replay_by_halving_and_doubling() {
        let mut v = Viewer::open(recorded(90), tuning()).unwrap();
        assert_eq!(v.kept(), [0], "before tick 1, beside the count");
        play_to(&mut v, 4);
        assert_eq!(v.kept(), [0]);
        play_to(&mut v, 15);
        assert_eq!(v.kept(), [0, 5, 10, 15]);
        assert_eq!(v.interval(), 5);
        // The fourth is the count: every other one goes and the interval
        // doubles.
        play_to(&mut v, 20);
        assert_eq!(v.kept(), [0, 10, 20]);
        assert_eq!(v.interval(), 10);
        play_to(&mut v, 39);
        assert_eq!(v.kept(), [0, 10, 20, 30]);
        play_to(&mut v, 40);
        assert_eq!(v.kept(), [0, 20, 40]);
        assert_eq!(v.interval(), 20);
        play_to(&mut v, 90);
        assert_eq!(v.kept(), [0, 40, 80]);
        assert_eq!(v.interval(), 40);
    }

    #[test]
    fn a_seek_restores_the_latest_state_before_the_tick_and_runs_on() {
        let replay = recorded(90);
        let stream = replay.run();
        assert_ne!(stream[11], stream[78], "the ticks tell apart");
        let mut v = Viewer::open(replay, tuning()).unwrap();
        play_to(&mut v, 90);
        assert_eq!((v.kept(), v.interval()), (vec![0, 40, 80], 40));

        // Back: from the state at 40, so one interval's budget is enough
        // and a smaller one is not.
        assert!(!v.seek_some(79, 10));
        assert_eq!(v.tick(), 50, "restored 40 and ran the budget");
        assert!(v.seek_some(79, 39 - 10));
        assert_eq!(v.tick(), 79);
        assert_eq!(v.state_hash(), stream[78]);

        v.seek(12);
        assert_eq!(v.tick(), 12);
        assert_eq!(v.state_hash(), stream[11]);
        assert_eq!(v.snapshot().tick, 12);

        // Forward past a saved state: it starts from that one, not from
        // where the playback stands.
        assert!(v.seek_some(85, 5), "from 80, five ticks");
        assert_eq!(v.state_hash(), stream[84]);
        // Forward with no state between: it simply runs on.
        v.seek(12);
        assert!(v.seek_some(30, 18));
        assert_eq!(v.state_hash(), stream[29]);
        assert!(!v.seek_some(39, 5));
        assert_eq!(v.tick(), 35);

        // Past the end is the end.
        v.seek(1_000);
        assert_eq!(v.tick(), 90);
        assert_eq!(v.state_hash(), stream[89]);
        // Seeking back and forth dropped and added nothing.
        assert_eq!((v.kept(), v.interval()), (vec![0, 40, 80], 40));
    }

    #[test]
    fn a_seek_into_ticks_never_played_saves_states_as_playing_does() {
        let replay = recorded(90);
        let mut played = Viewer::open(replay.clone(), tuning()).unwrap();
        play_to(&mut played, 90);
        let mut sought = Viewer::open(replay, tuning()).unwrap();
        sought.seek(90);
        assert_eq!(sought.kept(), played.kept());
        assert_eq!(sought.interval(), played.interval());
        assert_eq!(sought.state_hash(), played.state_hash());
        assert!(sought.done());
    }

    #[test]
    fn a_state_a_fork_or_a_hit_needs_is_kept_beyond_the_count() {
        let mut v = Viewer::open(recorded(60), tuning()).unwrap();
        play_to(&mut v, 7);
        // A fork opened by hand saves at once, wherever the playback is.
        assert_eq!(v.keep_now(), Ok(7));
        assert_eq!(v.kept(), [0, 5, 7]);
        play_to(&mut v, 20);
        assert_eq!(v.kept(), [0, 7, 10, 20], "thinned around the pin");
        // A hit at tick 18 needs the latest state before it, twice over.
        assert_eq!(v.pin_before(18), 10);
        assert_eq!(v.pin_before(18), 10);
        v.unpin(7);
        assert_eq!(v.kept(), [0, 10, 20], "no interval asks for 7");
        play_to(&mut v, 40);
        assert_eq!(v.interval(), 20);
        assert_eq!(v.kept(), [0, 10, 20, 40], "10 is still needed");
        v.unpin(10);
        assert_eq!(v.kept(), [0, 10, 20, 40], "one of the two still needs it");
        v.unpin(10);
        assert_eq!(v.kept(), [0, 20, 40]);
        // Tick 0's is beside the count and never dropped.
        assert_eq!(v.pin_before(3), 0);
        v.unpin(0);
        assert_eq!(v.kept(), [0, 20, 40]);
    }

    #[test]
    fn a_replay_is_drawn_for_the_team_that_saved_it_or_the_first() {
        let replay = recorded(5);
        assert_eq!(replay.team, Some(TeamId(0)), "the driver records the saver");
        let theirs = Replay {
            team: Some(TeamId(1)),
            ..replay.clone()
        };
        let v = Viewer::open(theirs, tuning()).unwrap();
        assert_eq!(v.team(), TeamId(1));
        assert!(v.can_debug());
        let nobodys = Replay {
            team: None,
            ..replay.clone()
        };
        let v = Viewer::open(nobodys, tuning()).unwrap();
        assert_eq!(v.team(), TeamId(0));
        assert!(!v.can_debug(), "watched, never debugged");
        let stranger = Replay {
            team: Some(TeamId(9)),
            ..replay
        };
        let error = Viewer::open(stranger, tuning()).err().expect("no team 9");
        assert!(
            error.contains("team 9") && error.contains("no such team"),
            "{error}"
        );
    }

    #[test]
    fn another_version_warns_on_opening_and_another_final_hash_reports_at_the_end() {
        let replay = recorded(12);
        let other = Replay {
            version: Some("0.0.0-other".into()),
            final_hash: replay.final_hash.map(|h| h ^ 1),
            ..replay
        };
        let mut v = Viewer::open(other, tuning()).unwrap();
        let warning = v.warning.clone().expect("warned as it opens");
        assert!(warning.contains("version 0.0.0-other"), "{warning}");
        play_to(&mut v, 11);
        assert_eq!(v.report, None, "nothing to compare before the last tick");
        play_to(&mut v, 12);
        let report = v.report.clone().expect("the playback ended elsewhere");
        assert!(
            report.contains("played the replay back differently"),
            "{report}"
        );
        // And by a seek as by playing.
        v.seek(3);
        v.report = None;
        v.seek(12);
        assert!(v.report.is_some());
    }
}
