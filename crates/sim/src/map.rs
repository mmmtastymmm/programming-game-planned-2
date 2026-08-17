//! The map specification: everything needed to construct a world, in a form
//! that serializes. A `MapSpec` plus a command log *is* a replay, so anything
//! that affects the starting state has to live here rather than being chosen
//! at construction time.

use serde::{Deserialize, Serialize};

/// Integer tile coordinates. There is no float position type in `sim` and
/// there will not be one (CLAUDE.md determinism rule 2) — sub-tile smoothing
/// is a rendering concern and belongs in the renderer crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan(self, other: Self) -> i64 {
        (self.x as i64 - other.x as i64).abs() + (self.y as i64 - other.y as i64).abs()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSpec {
    pub width: i32,
    pub height: i32,
    /// The match seed. Every named RNG stream derives from it; nothing in the
    /// sim reads entropy from anywhere else.
    pub seed: u64,
    /// Entities present at tick 0, in the order they get their ids.
    pub spawns: Vec<TilePos>,
}

impl MapSpec {
    pub fn empty(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            seed: 0,
            spawns: Vec::new(),
        }
    }

    pub fn contains(&self, pos: TilePos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width && pos.y < self.height
    }
}
