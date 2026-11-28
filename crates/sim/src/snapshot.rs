//! The completed-tick snapshot (`docs/06`, The snapshot): a plain value with
//! no reference into the sim, for the renderer and the replay tool.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::tick::Sim;
use crate::world::{
    Action, Effect, EntityId, GridFigures, LogEntry, Machine, MachineRecord, Plans, Sound,
    TileMemory, World,
};
use lang::FaultRecord;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct MachineSnapshot {
    pub record: MachineRecord,
    /// The version of the bundle its program runs, which a deploy since
    /// may have left behind (Q11); `None` with no program.
    pub runs: Option<u64>,
    pub log: Vec<LogEntry>,
    pub fault: Option<FaultRecord>,
    /// A bot's move or bonk in progress (T46), for the renderer alone:
    /// read off the machine's action as the snapshot is taken, stored
    /// nowhere in the world, and no part of the record or the state hash.
    pub moving: Option<Motion>,
}

/// A move or a bonk in progress (`docs/03`, Moving), as the renderer needs
/// it to draw one over the whole of its ticks: `record.busy` is `"move"`
/// for both and `record.progress` the ticks left, which say neither where
/// the bot is headed nor how far through it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motion {
    /// The tile a move takes the bot onto, or the tile of the bot a bonk
    /// is on.
    pub to: TilePos,
    /// The ticks the whole of it takes, of which `record.progress` are
    /// left: a bonk's are its model's `bonk_ticks`, and a move's are
    /// worked out again from the bot's pace and the slope, which is what
    /// fixed them as it began unless the ground has changed under it
    /// since. Never fewer than are left.
    pub ticks: u64,
    /// The bot a bonk is on; `None` for a move.
    pub bonk: Option<EntityId>,
}

impl MachineSnapshot {
    pub(crate) fn of(w: &World, m: &Machine) -> MachineSnapshot {
        MachineSnapshot {
            record: MachineRecord::of(m),
            runs: m.program.as_ref().map(|p| p.version()),
            log: m.log.clone(),
            fault: m.program.as_ref().and_then(|p| p.fault_record.clone()),
            moving: Motion::of(w, m),
        }
    }
}

impl Motion {
    fn of(w: &World, m: &Machine) -> Option<Motion> {
        let Action {
            remaining, effect, ..
        } = m.action.as_ref()?;
        let (to, ticks, bonk) = match effect {
            Effect::Move { to } => (*to, w.move_ticks(m.id, *to), None),
            Effect::Bonk { to, target } => {
                (*to, w.data.model(m.model.name()).bonk_ticks, Some(*target))
            }
            _ => return None,
        };
        Some(Motion {
            to,
            ticks: ticks.max(*remaining),
            bonk,
        })
    }
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
    /// This team's own plans, on the tiles that carry any, in the same
    /// order (Q37). Plans are private (`docs/03`), so a renderer reads only
    /// its own team's; nothing here is in the state hash.
    pub plans: Vec<(TilePos, Plans)>,
    /// The team's grids as its last power step kept them, ascending
    /// label (Q62; `docs/08`).
    pub grids: Vec<GridFigures>,
    /// The tiles carrying this team's own wire, in the same order, each
    /// with whether that power step found it part of a grid.
    pub wires: Vec<(TilePos, bool)>,
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
            .map(|m| MachineSnapshot::of(w, m))
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
                plans: w
                    .map
                    .positions()
                    .into_iter()
                    .filter_map(|p| {
                        w.plans(t.id, p)
                            .filter(|plans| !plans.is_empty())
                            .map(|plans| (p, plans.clone()))
                    })
                    .collect(),
                grids: t.grids.clone(),
                wires: {
                    let mut tiles: Vec<TilePos> = t.wire_tiles.iter().copied().collect();
                    tiles.sort_by(crate::map::row_then_column);
                    tiles
                        .into_iter()
                        .map(|p| (p, t.live_wires.contains(&p)))
                        .collect()
                },
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
