//! World state.
//!
//! PLACEHOLDER CONTENT. The entity model here is the smallest thing that makes
//! the determinism gate real — it has ordered state, a seeded RNG draw, and
//! integer movement, so the golden fixture would actually catch a violation.
//! It is not a design commitment; the design docs replace it.
//!
//! What is *not* placeholder, and should survive whatever replaces it:
//!
//! * `BTreeMap`, never `HashMap`, for anything the sim iterates (rule 3).
//! * `state_hash` walks state in one fixed order and includes the RNG streams.
//! * Ids are allocated by a counter owned by the world, never by insertion
//!   order into a hashed container.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::hash::Fnv1a;
use crate::map::{MapSpec, TilePos};
use crate::rng::Stream;

/// Serializable because [`crate::Command`] carries ids and commands land in
/// replay fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub pos: TilePos,
    /// Where it is headed, if anywhere. `None` means it wanders.
    pub goal: Option<TilePos>,
    /// Tiles moved since spawn — a cheap way for the fixture to notice that
    /// behavior changed even when final positions happen to coincide.
    pub distance_travelled: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub tick: u64,
    pub width: i32,
    pub height: i32,
    /// Ordered by construction. Every iteration in sim logic goes through this
    /// map's natural order or an explicitly sorted `Vec`.
    pub entities: BTreeMap<EntityId, Entity>,
    next_id: u32,
    /// Movement jitter for entities with no goal. One stream, one domain.
    pub wander: Stream,
}

impl World {
    pub fn from_spec(spec: &MapSpec) -> Self {
        let mut world = Self {
            tick: 0,
            width: spec.width,
            height: spec.height,
            entities: BTreeMap::new(),
            next_id: 1,
            wander: Stream::new(spec.seed, "wander"),
        };
        for &pos in &spec.spawns {
            world.spawn(pos);
        }
        world
    }

    pub fn alloc_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn spawn(&mut self, pos: TilePos) -> EntityId {
        let id = self.alloc_id();
        self.entities.insert(
            id,
            Entity {
                pos,
                goal: None,
                distance_travelled: 0,
            },
        );
        id
    }

    /// The entity with `id`. Panics if absent — callers hold ids they just got
    /// from the world, so a miss is a sim bug rather than player input.
    pub fn entity(&self, id: EntityId) -> &Entity {
        self.entities.get(&id).expect("entity exists")
    }

    pub fn in_bounds(&self, pos: TilePos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width && pos.y < self.height
    }

    /// The full state fingerprint. Everything that can differ between two
    /// peers goes in, in a fixed order — including the RNG stream states, so a
    /// peer that has drawn a different *number* of randoms fails immediately
    /// instead of on the tick where the divergence finally moves something.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_str("world");
        h.write_u64(self.tick);
        h.write_i32(self.width);
        h.write_i32(self.height);
        h.write_u32(self.next_id);
        h.write_u64(self.entities.len() as u64);
        for (id, e) in &self.entities {
            h.write_u32(id.0);
            h.write_i32(e.pos.x);
            h.write_i32(e.pos.y);
            match e.goal {
                Some(g) => {
                    h.write_u8(1);
                    h.write_i32(g.x);
                    h.write_i32(g.y);
                }
                None => h.write_u8(0),
            }
            h.write_u64(e.distance_travelled);
        }
        self.wander.hash_into(&mut h);
        h.finish()
    }
}
