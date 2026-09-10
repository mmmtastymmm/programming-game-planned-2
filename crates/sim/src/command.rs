//! The command log's entries (`docs/06`, The command log): everything
//! outside the sim — a player, a peer, a script, a replay — influences the
//! world through these and nothing else (CLAUDE.md rule 5). `net` owns the
//! log and its byte layout; `sim` owns the types.

use crate::map::TilePos;
use serde::{Deserialize, Serialize};

/// A team, from the map's team list, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TeamId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlanKind {
    Paint,
    Overlay,
    Building,
}

impl PlanKind {
    pub fn name(self) -> &'static str {
        match self {
            PlanKind::Paint => "paint",
            PlanKind::Overlay => "overlay",
            PlanKind::Building => "building",
        }
    }

    pub fn from_name(s: &str) -> Option<PlanKind> {
        match s {
            "paint" => Some(PlanKind::Paint),
            "overlay" => Some(PlanKind::Overlay),
            "building" => Some(PlanKind::Building),
            _ => None,
        }
    }
}

/// A bundle as the log carries it: the files, byte-exact (`docs/01`, The
/// bundle). Source is UTF-8 text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub files: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandKind {
    /// Write a bundle into a deployment slot; waits if the slot is locked.
    Deploy { deployment: String, bundle: Bundle },
    /// Place a plan on a tile: a value, or `None` for a plan to clear.
    Mark {
        at: TilePos,
        plan: PlanKind,
        value: Option<String>,
    },
    /// Withdraw a plan.
    Unmark { at: TilePos, plan: PlanKind },
    /// A speed step, recorded for the driver.
    SetSpeed(u64),
    /// The sender's team is out on this tick.
    Resign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// The tick it is agreed for.
    pub tick: u64,
    pub sender: TeamId,
    /// The sender's submission counter, for ordering within a tick.
    pub seq: u32,
    pub kind: CommandKind,
}

impl Command {
    pub fn new(tick: u64, sender: TeamId, seq: u32, kind: CommandKind) -> Command {
        Command {
            tick,
            sender,
            seq,
            kind,
        }
    }
}
