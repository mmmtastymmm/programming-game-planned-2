//! The headless driver (`docs/06`, The driver): runs `(map, command log)`
//! to a hash stream, with no clock and no peers — every set is in hand, so
//! the delay is felt and the stall never fires. What CI runs.

use net::CommandLog;
use serde::{Deserialize, Serialize};
use sim::hash::Fnv1a;
use sim::{Command, Data, Map, Sim, TeamId};

/// Ceiling on a replay's declared length, so a corrupt artifact cannot ask
/// the allocator for the moon.
pub const MAX_REPLAY_TICKS: u64 = 10_000_000;

/// A complete description of a match: the map's bytes and every command,
/// in agreed order. Serializable, since it is what a desync report attaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub map: String,
    pub commands: Vec<Command>,
    /// Ticks to run, or until the match ends.
    pub ticks: u64,
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
            let report = sim.step(&commands);
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

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("replay serializes")
    }

    pub fn from_ron(text: &str) -> Result<Replay, ron::error::SpannedError> {
        ron::from_str(text)
    }
}
