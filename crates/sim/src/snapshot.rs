//! The completed-tick snapshot (`docs/06`, The snapshot): a plain value with
//! no reference into the sim, for the renderer and the replay tool.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::tick::Sim;
use crate::world::{LogEntry, MachineRecord, Sound, TileMemory};
use lang::FaultRecord;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct MachineSnapshot {
    pub record: MachineRecord,
    pub log: Vec<LogEntry>,
    pub fault: Option<FaultRecord>,
}

#[derive(Debug, Clone)]
pub struct TeamSnapshot {
    pub id: TeamId,
    pub out: bool,
    /// Deployment name to the version of its bundle, if any.
    pub deployments: BTreeMap<String, Option<u64>>,
    pub cap: usize,
    pub bots: usize,
    /// Every tile as this team sees it, in row-then-column order.
    pub tiles: Vec<(TilePos, TileMemory)>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub tick: u64,
    pub speed: u64,
    pub machines: Vec<MachineSnapshot>,
    pub teams: Vec<TeamSnapshot>,
    pub sounds: Vec<Sound>,
}

impl Sim {
    pub fn snapshot(&self) -> Snapshot {
        let w = &self.world;
        let machines = w
            .machines
            .values()
            .map(|m| MachineSnapshot {
                record: MachineRecord::of(m),
                log: m.log.clone(),
                fault: m.program.as_ref().and_then(|p| p.fault_record.clone()),
            })
            .collect();
        let teams = w
            .teams
            .values()
            .map(|t| TeamSnapshot {
                id: t.id,
                out: t.out,
                deployments: t
                    .deployments
                    .iter()
                    .map(|(k, d)| (k.clone(), d.bundle.as_ref().map(|b| b.version)))
                    .collect(),
                cap: w.bot_cap(t.id),
                bots: w.bots_of(t.id),
                tiles: w
                    .map
                    .positions()
                    .into_iter()
                    .map(|p| (p, t.memory[w.tile_index(p).expect("in bounds")].clone()))
                    .collect(),
            })
            .collect();
        Snapshot {
            tick: w.tick,
            speed: w.speed,
            machines,
            teams,
            sounds: w.sounds_prev.clone(),
        }
    }
}
