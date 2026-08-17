//! The serialized replay artifact: `(map spec, command log)` → hash stream.
//!
//! This is the format golden fixtures are stored in and the artifact a
//! lockstep desync report attaches. Because a replay is a *complete*
//! description of a match, a bug report is one file.

use serde::{Deserialize, Serialize};

use crate::map::MapSpec;
use crate::sim::{Command, Sim};

/// A command agreed for a specific tick. Commands fire *before* the tick's
/// step: `tick: 0` applies before the first `step()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedCommand {
    pub tick: u64,
    pub command: Command,
}

/// A complete deterministic run description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub spec: MapSpec,
    /// Must be sorted by tick (validated in [`Replay::run`]); same-tick
    /// commands apply in list order — the agreed lockstep order.
    pub commands: Vec<TimedCommand>,
    /// Number of ticks to run.
    pub ticks: u64,
}

impl Replay {
    /// Execute the replay, returning the per-tick state-hash stream
    /// (`result[t]` = hash after `t + 1` steps).
    ///
    /// A rejected command panics: a replay is a record of commands that *were*
    /// accepted, so rejection means the artifact is corrupt or the sim has
    /// drifted, and both deserve a loud failure rather than a skipped command
    /// and a mystery hash.
    pub fn run(&self) -> Vec<u64> {
        assert!(
            self.commands.windows(2).all(|w| w[0].tick <= w[1].tick),
            "replay commands must be sorted by tick"
        );
        let mut sim = Sim::new(&self.spec);
        let mut next = 0;
        let mut hashes = Vec::with_capacity(self.ticks as usize);
        for tick in 0..self.ticks {
            while next < self.commands.len() && self.commands[next].tick == tick {
                sim.apply(&self.commands[next].command)
                    .expect("replayed command accepted");
                next += 1;
            }
            sim.step();
            hashes.push(sim.state_hash());
        }
        assert!(
            next == self.commands.len(),
            "replay has commands past its tick count"
        );
        hashes
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("replay serializes")
    }

    pub fn from_ron(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }
}
