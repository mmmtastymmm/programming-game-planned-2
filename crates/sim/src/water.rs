//! Water (`docs/09`; Q65): parts that stand on the ground and flow to
//! lower ground a tile a tick. The water step is the only thing in the sim
//! that moves water.

use crate::map::Terrain;
use crate::world::World;
use std::collections::BTreeSet;

impl World {
    /// An open tile's surface in parts (`docs/09`): its height in parts,
    /// plus its water.
    pub fn surface(&self, idx: usize) -> u32 {
        let t = &self.tiles[idx];
        u32::from(t.height.level())
            .saturating_mul(self.data.world.water.parts)
            .saturating_add(t.water)
    }

    /// Whether the tile is wet (`docs/09`): holding water that counts.
    pub fn is_wet(&self, idx: usize) -> bool {
        self.tiles[idx].water >= self.data.world.water.wet
    }

    /// The tiles beside `idx`, in the order `n`, `e`, `s`, `w`, each
    /// `None` off the map. The tile vector is row-major from the
    /// south-west, so north is a row on.
    fn beside(&self, idx: usize) -> [Option<usize>; 4] {
        let width = self.map.width as usize;
        let col = idx.checked_rem(width).unwrap_or(0);
        [
            idx.checked_add(width).filter(|n| *n < self.tiles.len()),
            (col.saturating_add(1) < width).then(|| idx.saturating_add(1)),
            idx.checked_sub(width),
            (col > 0).then(|| idx.saturating_sub(1)),
        ]
    }

    /// Say that the water on `idx` may no longer be still: its water, its
    /// height or its terrain changed. It and the tiles beside it are looked
    /// at in the next water step. Everything that changes one of those
    /// three must call this, or the step would skip a tile that now gives.
    pub fn stir(&mut self, idx: usize) {
        self.stirred.insert(idx);
        for n in self.beside(idx).into_iter().flatten() {
            self.stirred.insert(n);
        }
    }

    /// The water step (`docs/09`): springs, the flow, sinks, drying. Only
    /// tiles that may have been disturbed are looked at for the flow —
    /// one whose own surface and whose neighbours' surfaces are what they
    /// were when it last gave nothing gives nothing again — so still
    /// water costs nothing, and the result is the one a walk of every
    /// tile gives.
    pub(crate) fn water_step(&mut self) {
        let wd = self.data.world.water;
        // The tiles whose water this step changes.
        let mut changed: BTreeSet<usize> = BTreeSet::new();

        // 1. Springs, in the order of their tiles.
        for i in 0..self.springs.len() {
            let idx = self.springs[i];
            let rate = u64::from(self.tiles[idx].spring.unwrap_or(0));
            let up = rate.min(self.ground_water);
            if up > 0 {
                self.ground_water = self.ground_water.saturating_sub(up);
                let t = &mut self.tiles[idx];
                t.water = t.water.saturating_add(up as u32);
                changed.insert(idx);
                self.stir(idx);
            }
        }

        // 2. The flow: every gift is worked out from the water as the
        // springs left it, and all are applied together.
        let mut gifts: Vec<(usize, usize, u32)> = Vec::new();
        for &idx in &self.stirred {
            let tile = &self.tiles[idx];
            if tile.terrain != Terrain::Ground || tile.water == 0 {
                continue;
            }
            let own = self.surface(idx);
            // Its lower neighbours: open, and at least two parts below.
            let mut lower: Vec<(u32, usize, usize)> = Vec::new();
            for (dir, n) in self.beside(idx).into_iter().enumerate() {
                let Some(n) = n else { continue };
                if self.tiles[n].terrain != Terrain::Ground {
                    continue;
                }
                let theirs = self.surface(n);
                if theirs.saturating_add(2) <= own {
                    lower.push((theirs, dir, n));
                }
            }
            if lower.is_empty() {
                continue;
            }
            // Its share of each difference, itself among those sharing.
            let sharing = (lower.len() as u32).saturating_add(1);
            // Met lowest surface first, then in the order n, e, s, w,
            // while its water lasts.
            lower.sort();
            let mut left = tile.water;
            for (theirs, _, n) in lower {
                let offer = own.saturating_sub(theirs).checked_div(sharing).unwrap_or(0);
                let given = offer.min(left);
                if given > 0 {
                    gifts.push((idx, n, given));
                    left = left.saturating_sub(given);
                }
            }
        }
        self.stirred.clear();
        for (from, to, parts) in gifts {
            self.tiles[from].water = self.tiles[from].water.saturating_sub(parts);
            self.tiles[to].water = self.tiles[to].water.saturating_add(parts);
            changed.insert(from);
            changed.insert(to);
        }

        // 3. Sinks, in the order of their tiles.
        for i in 0..self.sinks.len() {
            let idx = self.sinks[i];
            let rate = self.tiles[idx].sink.unwrap_or(0);
            let down = rate.min(self.tiles[idx].water);
            if down > 0 {
                self.tiles[idx].water = self.tiles[idx].water.saturating_sub(down);
                self.ground_water = self.ground_water.saturating_add(u64::from(down));
                changed.insert(idx);
            }
        }

        // 4. Thin water dries, a spring's own tile aside. A tile this
        // step did not touch held more than a film before it, or nothing.
        for &idx in &changed {
            let t = &mut self.tiles[idx];
            if t.water > 0 && t.water <= wd.film && t.spring.is_none() {
                self.ground_water = self.ground_water.saturating_add(u64::from(t.water));
                t.water = 0;
            }
        }

        // What changed, and what is beside it, is looked at next step.
        for idx in changed {
            self.stir(idx);
        }
    }

    /// All the water of the match, in parts: what stands on the tiles and
    /// the ground water. The water step never changes it (`docs/09`).
    pub fn water_total(&self) -> u64 {
        self.tiles.iter().fold(self.ground_water, |sum, t| {
            sum.saturating_add(u64::from(t.water))
        })
    }
}
