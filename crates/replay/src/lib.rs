//! The headless driver (`docs/06`, The driver): runs `(map, command log)`
//! to a hash stream, with no clock and no peers — every set is in hand, so
//! the delay is felt and the stall never fires. What CI runs.

use net::CommandLog;
use serde::{Deserialize, Serialize};
use sim::hash::Fnv1a;
use sim::{Command, Data, Fork, Map, Sim, StepReport, TeamId, Timeline, Watch};

/// Ceiling on a replay's declared length, so a corrupt artifact cannot ask
/// the allocator for the moon.
pub const MAX_REPLAY_TICKS: u64 = 10_000_000;

/// The version of the game a build writes into the replays it saves (Q60):
/// the workspace's, which every crate shares.
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A complete description of a match: the map's bytes and every command,
/// in agreed order. Serializable, since it is what a desync report attaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub map: String,
    pub commands: Vec<Command>,
    /// Ticks to run, or until the match ends.
    pub ticks: u64,
    /// The version of the game that wrote the file (Q60), outside the
    /// replay's identity. None in a file older than Q60, and in a replay
    /// built in code rather than saved from a match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The state hash at the last tick as the writer computed it (Q60),
    /// outside the replay's identity: what a playback compares its own
    /// with, since a build's rules can change while its version does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_hash: Option<u64>,
    /// The team of the peer that saved the file (Q55), outside the
    /// replay's identity: whose view a playback draws and whose debugger
    /// it may open. None in a file older than Q55, and in a replay a tool
    /// wrote rather than a match's end, which can only be watched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamId>,
}

/// A replay's log as a fork reads it (`docs/06`, Forks): complete from the
/// start, so every set is final through the replay's last tick.
pub struct ReplayLog {
    log: CommandLog,
    ticks: u64,
}

impl Timeline for ReplayLog {
    fn commands(&self, tick: u64) -> Vec<Command> {
        // The opening set is tick 0's, applied with the first tick.
        let mut commands = if tick == 1 {
            self.log.sets_for(0)
        } else {
            Vec::new()
        };
        commands.extend(self.log.sets_for(tick));
        commands
    }

    fn final_tick(&self) -> u64 {
        self.ticks
    }
}

/// A replay played on a fork (`docs/06`, The driver): it sends no
/// command, moves a tick at a time at the caller's pace, and stands for
/// the match when a debugger forks from it.
pub struct Playback {
    pub fork: Fork,
    pub log: ReplayLog,
}

impl Playback {
    /// The tick the playback stands at the end of.
    pub fn tick(&self) -> u64 {
        self.fork.world().tick
    }

    /// The replay's last tick.
    pub fn ticks(&self) -> u64 {
        self.log.ticks
    }

    /// Whether the playback has reached the replay's last tick, or its
    /// match's end.
    pub fn done(&self) -> bool {
        self.tick() >= self.log.ticks || self.fork.ended().is_some()
    }

    /// Play the next tick, reporting as `sim.step` does, `watches`' hits
    /// included. `None` once the playback is done: it never runs past the
    /// replay.
    pub fn advance(&mut self, watches: &[Watch]) -> Option<StepReport> {
        (!self.done()).then(|| self.fork.advance(watches, &self.log))
    }

    /// Stand at the end of `tick`, or of the replay's last if that is
    /// earlier, from a state the playback kept or by running on.
    pub fn seek(&mut self, tick: u64) {
        self.fork.seek(tick.min(self.log.ticks), &self.log);
    }

    /// The state hash at the tick the playback stands at.
    pub fn state_hash(&self) -> Option<u64> {
        self.fork.state_hash()
    }
}

/// What a run produced: the per-tick hash stream and the finished sim.
pub struct Run {
    pub sim: Sim,
    pub hashes: Vec<u64>,
    /// Commands the sim dropped on their tick, with the tick.
    pub dropped: Vec<(u64, Command)>,
}

impl Replay {
    /// The replay's identity: the map's bytes and the log's canonical
    /// bytes, each length-prefixed (`docs/03`, `docs/06`).
    pub fn identity(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_str(&self.map);
        let log = self.log();
        let bytes = log.canonical_bytes();
        h.write_u64(bytes.len() as u64);
        h.write_bytes(&bytes);
        h.finish()
    }

    /// What opening this replay warns of (Q60, `docs/07`, The match): its
    /// recorded version is not this build's, or it records none. It plays
    /// anyway, on this build's rules; nothing of the old ones is kept.
    pub fn version_warning(&self) -> Option<String> {
        let wrote = match self.version.as_deref() {
            Some(v) if v == GAME_VERSION => return None,
            Some(v) => format!("version {v}"),
            None => "a version older than any that records itself".to_string(),
        };
        Some(format!(
            "this replay was written by {wrote}, and this is version {GAME_VERSION}: the rules may have changed since, so it may not play back as it was played"
        ))
    }

    /// What a playback that reached the last tick on `played` reports
    /// (Q60): nothing if that is the hash the replay recorded, or if it
    /// recorded none, and otherwise that this build played it back
    /// differently, whatever the version.
    pub fn final_hash_report(&self, played: u64) -> Option<String> {
        match self.final_hash {
            Some(recorded) if recorded != played => Some(format!(
                "this build played the replay back differently: it recorded the final hash {recorded:016x}, and the playback ended on {played:016x}"
            )),
            _ => None,
        }
    }

    /// The commands as a log every team is a peer of.
    pub fn log(&self) -> CommandLog {
        let peers: Vec<TeamId> = {
            let mut p: Vec<TeamId> = self.commands.iter().map(|c| c.sender).collect();
            p.sort();
            p.dedup();
            p
        };
        let mut log = CommandLog::new(peers);
        for c in &self.commands {
            log.push(c.clone());
        }
        log
    }

    /// Run every tick, validating each command at submission as a peer
    /// would (`docs/06`): a command that can never apply is a corrupt
    /// artifact and fails loudly.
    pub fn execute(&self) -> Run {
        assert!(
            self.ticks <= MAX_REPLAY_TICKS,
            "replay declares {} ticks, over the cap",
            self.ticks
        );
        let data = Data::load().expect("the shipped tables load");
        let map = Map::parse(&self.map, &data)
            .unwrap_or_else(|e| panic!("the replay's map does not load: {e}"));
        let mut sim = Sim::new(map);
        for c in &self.commands {
            if let Err(e) = sim.validate(c) {
                panic!("a replayed command can never apply: {e}");
            }
        }
        let log = self.log();
        let mut hashes = Vec::with_capacity(self.ticks as usize);
        let mut dropped = Vec::new();
        for tick in 1..=self.ticks {
            // Commands agreed for tick t apply at the start of tick t; the
            // opening set is tick 0's, applied with the first tick.
            let mut commands = if tick == 1 {
                log.sets_for(0)
            } else {
                Vec::new()
            };
            commands.extend(log.sets_for(tick));
            let report = sim.step(&commands, &[]);
            for c in report.dropped {
                dropped.push((tick, c));
            }
            hashes.push(sim.state_hash());
            if report.ended.is_some() {
                break;
            }
        }
        Run {
            sim,
            hashes,
            dropped,
        }
    }

    pub fn run(&self) -> Vec<u64> {
        self.execute().hashes
    }

    /// The replay on a fork, standing before tick 1, for `team`'s viewer
    /// (`docs/06`, The driver). Unlike [`Replay::execute`], which CI runs
    /// on artifacts of its own, this is handed files a player opens: one
    /// that cannot play is an error, never a panic.
    pub fn playback(&self, team: TeamId) -> Result<Playback, String> {
        if self.ticks > MAX_REPLAY_TICKS {
            return Err(format!(
                "the replay declares {} ticks, over the cap of {MAX_REPLAY_TICKS}",
                self.ticks
            ));
        }
        let data = Data::load().map_err(|e| format!("the shipped tables do not load: {e}"))?;
        let map = Map::parse(&self.map, &data)
            .map_err(|e| format!("the replay's map does not load: {e}"))?;
        let fork = Fork::from_map(map, team);
        for c in &self.commands {
            fork.validate(c)
                .map_err(|e| format!("a replayed command can never apply: {e}"))?;
        }
        Ok(Playback {
            fork,
            log: ReplayLog {
                log: self.log(),
                ticks: self.ticks,
            },
        })
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("replay serializes")
    }

    pub fn from_ron(text: &str) -> Result<Replay, ron::error::SpannedError> {
        ron::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replay() -> Replay {
        let map = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/maps/first.toml"
        ))
        .expect("the first map");
        Replay {
            map,
            commands: Vec::new(),
            ticks: 20,
            version: None,
            final_hash: None,
            team: None,
        }
    }

    #[test]
    fn a_replay_round_trips_its_team_outside_its_identity() {
        let bare = replay();
        let text = bare.to_ron();
        assert!(!text.contains("team:"), "{text}");
        let old = Replay::from_ron(&text).expect("a file older than Q55 parses");
        assert_eq!(old.team, None);
        let saved = Replay {
            team: Some(TeamId(1)),
            ..bare.clone()
        };
        let text = saved.to_ron();
        assert!(text.contains("team:"), "{text}");
        let back = Replay::from_ron(&text).expect("parses");
        assert_eq!(back.team, Some(TeamId(1)));
        // Two peers' files of one match are one replay (`docs/06`): they
        // differ in the team alone.
        assert_eq!(back.identity(), bare.identity());
        assert_eq!(back.run(), bare.run(), "the team is no input to the run");
    }

    #[test]
    fn a_replay_round_trips_its_version_and_final_hash_outside_its_identity() {
        let bare = replay();
        let played = *bare.run().last().expect("ticks ran");
        let saved = Replay {
            version: Some(GAME_VERSION.to_string()),
            final_hash: Some(played),
            ..bare.clone()
        };
        let text = saved.to_ron();
        assert!(
            text.contains("version:") && text.contains("final_hash:"),
            "{text}"
        );
        let back = Replay::from_ron(&text).expect("parses");
        assert_eq!(back, saved);
        assert_eq!(back.version.as_deref(), Some(GAME_VERSION));
        assert_eq!(back.final_hash, Some(played));
        // Two peers' files of one match are one replay (`docs/06`).
        assert_eq!(back.identity(), bare.identity());
        assert_eq!(back.version_warning(), None);
        assert_eq!(back.final_hash_report(played), None);
    }

    #[test]
    fn a_replay_older_than_q60_parses_and_warns_that_it_records_no_version() {
        // A file written before the fields existed holds neither.
        let text = replay().to_ron();
        assert!(
            !text.contains("version:") && !text.contains("final_hash:"),
            "{text}"
        );
        let back = Replay::from_ron(&text).expect("an old file still parses");
        assert_eq!((back.version.clone(), back.final_hash), (None, None));
        let warning = back.version_warning().expect("no version is a warning");
        assert!(
            warning.contains("older than any that records itself"),
            "{warning}"
        );
        // Nothing recorded, so nothing to differ from.
        assert_eq!(back.final_hash_report(1), None);
    }

    #[test]
    fn a_replay_from_another_version_warns_naming_it_and_still_plays() {
        let other = Replay {
            version: Some("0.0.0-other".to_string()),
            ..replay()
        };
        let warning = other
            .version_warning()
            .expect("another version is a warning");
        assert!(warning.contains("version 0.0.0-other"), "{warning}");
        assert!(warning.contains(GAME_VERSION), "{warning}");
        assert_eq!(other.run().len(), 20, "it plays anyway");
    }

    #[test]
    fn a_replay_whose_recorded_final_hash_is_altered_reports_the_difference() {
        let bare = replay();
        let played = *bare.run().last().expect("ticks ran");
        let altered = Replay {
            version: Some(GAME_VERSION.to_string()),
            final_hash: Some(played ^ 1),
            ..bare
        };
        // The version is this build's, so only the hash can tell.
        assert_eq!(altered.version_warning(), None);
        let played_again = *altered.run().last().expect("ticks ran");
        assert_eq!(played_again, played, "the fields are no input to the run");
        let report = altered
            .final_hash_report(played_again)
            .expect("a differing final hash is reported");
        assert!(report.contains(&format!("{:016x}", played ^ 1)), "{report}");
        assert!(report.contains(&format!("{played:016x}")), "{report}");
    }
}
