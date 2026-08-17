//! The tick loop and the command boundary.
//!
//! Everything outside the sim — a local player, a remote peer, an AI, a
//! replay file — influences the world through [`Command`] and nothing else
//! (CLAUDE.md determinism rule 5). That is what makes single-player lockstep
//! with one peer, and it is what makes a replay a complete description of a
//! match rather than a hint about one.

use serde::{Deserialize, Serialize};

use crate::map::{MapSpec, TilePos};
use crate::world::{EntityId, World};

/// External input. Serializable because it travels the wire and lands in
/// replay fixtures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Spawn {
        pos: TilePos,
    },
    SetGoal {
        entity: EntityId,
        goal: Option<TilePos>,
    },
    Despawn {
        entity: EntityId,
    },
}

/// Why a command was refused. Rejection must be a *deterministic function of
/// world state* — every peer refuses the same command on the same tick, or
/// they have already desynced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    OutOfBounds(TilePos),
    NoSuchEntity(EntityId),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds(p) => write!(f, "position ({}, {}) is off the map", p.x, p.y),
            Self::NoSuchEntity(id) => write!(f, "no entity {}", id.0),
        }
    }
}

impl std::error::Error for CommandError {}

pub struct Sim {
    pub world: World,
}

impl Sim {
    pub fn new(spec: &MapSpec) -> Self {
        Self {
            world: World::from_spec(spec),
        }
    }

    /// Apply one command. Commands land *before* the tick's `step`.
    pub fn apply(&mut self, command: &Command) -> Result<(), CommandError> {
        match command {
            Command::Spawn { pos } => {
                if !self.world.in_bounds(*pos) {
                    return Err(CommandError::OutOfBounds(*pos));
                }
                self.world.spawn(*pos);
            }
            Command::SetGoal { entity, goal } => {
                if let Some(g) = goal
                    && !self.world.in_bounds(*g)
                {
                    return Err(CommandError::OutOfBounds(*g));
                }
                let e = self
                    .world
                    .entities
                    .get_mut(entity)
                    .ok_or(CommandError::NoSuchEntity(*entity))?;
                e.goal = *goal;
            }
            Command::Despawn { entity } => {
                self.world
                    .entities
                    .remove(entity)
                    .ok_or(CommandError::NoSuchEntity(*entity))?;
            }
        }
        Ok(())
    }

    /// Advance one tick.
    ///
    /// PLACEHOLDER behavior — one tile of movement per entity per tick. The
    /// shape is the part that matters: entities are visited in id order, and
    /// the shared RNG stream is drawn from inside that ordered walk, so the
    /// draw sequence is a function of world state alone.
    pub fn step(&mut self) {
        let ids: Vec<EntityId> = self.world.entities.keys().copied().collect();
        for id in ids {
            let (pos, goal) = {
                let e = self.world.entity(id);
                (e.pos, e.goal)
            };
            let next = match goal {
                // Close the larger axis first, x on a tie. Arbitrary, but
                // *stated* — an unstated tiebreak is the classic desync.
                Some(g) if g != pos => {
                    let dx = g.x - pos.x;
                    let dy = g.y - pos.y;
                    if dx.abs() >= dy.abs() {
                        TilePos::new(pos.x + dx.signum(), pos.y)
                    } else {
                        TilePos::new(pos.x, pos.y + dy.signum())
                    }
                }
                // Arrived, or never had a goal: jitter.
                _ => {
                    const STEPS: [(i32, i32); 5] = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)];
                    let (dx, dy) = STEPS[self.world.wander.below(STEPS.len() as u64) as usize];
                    TilePos::new(pos.x + dx, pos.y + dy)
                }
            };
            let moved = self.world.in_bounds(next) && next != pos;
            let e = self.world.entities.get_mut(&id).expect("entity exists");
            if moved {
                e.pos = next;
                e.distance_travelled += 1;
            }
            if e.goal == Some(e.pos) {
                e.goal = None;
            }
        }
        self.world.tick += 1;
    }

    pub fn state_hash(&self) -> u64 {
        self.world.state_hash()
    }
}
