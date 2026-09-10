//! Line of sight, vision and hearing (`docs/03`, Line of sight; `docs/02`,
//! Senses): one integer algorithm on every peer.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::world::{EntityId, World};
use std::collections::BTreeSet;

/// The tiles Bresenham's ray from `a` to `b` passes through, `a` and `b`
/// excluded, with `docs/03`'s fixings so the walk is single-valued.
pub fn ray(a: TilePos, b: TilePos) -> Vec<TilePos> {
    let dx = i64::from(b.x).saturating_sub(i64::from(a.x)).abs();
    let dy = i64::from(b.y).saturating_sub(i64::from(a.y)).abs();
    let sx: i64 = if b.x > a.x { 1 } else { -1 };
    let sy: i64 = if b.y > a.y { 1 } else { -1 };
    let mut err = dx.saturating_sub(dy);
    let (mut x, mut y) = (i64::from(a.x), i64::from(a.y));
    let mut out = Vec::new();
    loop {
        if x == i64::from(b.x) && y == i64::from(b.y) {
            break;
        }
        let e2 = err.saturating_mul(2);
        if e2 > dy.saturating_neg() {
            err = err.saturating_sub(dy);
            x = x.saturating_add(sx);
        }
        if e2 < dx {
            err = err.saturating_add(dx);
            y = y.saturating_add(sy);
        }
        let here = TilePos::new(x as i32, y as i32);
        if here != b {
            out.push(here);
        }
    }
    out
}

/// A clear line of sight from the seer at `a` to `b`: no tile the ray
/// passes through blocks sight. Always evaluated from the seer.
pub fn clear(world: &World, a: TilePos, b: TilePos) -> bool {
    ray(a, b).into_iter().all(|p| !world.blocks_sight(p))
}

/// Whether the machine `seer` sees the tile `p`: within its model's vision
/// range by squared distance, with a clear line of sight.
pub fn sees_tile(world: &World, seer: EntityId, p: TilePos) -> bool {
    let Some(m) = world.machines.get(&seer) else {
        return false;
    };
    let vision = world.data.model(m.model.name()).vision;
    m.pos.dist2(p) <= vision.saturating_mul(vision) && clear(world, m.pos, p)
}

/// Every tile a team's machines see now, in row-then-column order.
pub fn team_visible_tiles(world: &World, team: TeamId) -> BTreeSet<TilePos> {
    let mut out = BTreeSet::new();
    for m in world.machines.values().filter(|m| m.team == team) {
        let vision = world.data.model(m.model.name()).vision;
        let r = i32::try_from(vision).unwrap_or(i32::MAX);
        for dy in r.saturating_neg()..=r {
            for dx in r.saturating_neg()..=r {
                let p = TilePos::new(m.pos.x.saturating_add(dx), m.pos.y.saturating_add(dy));
                if !world.map.in_bounds(p) || out.contains(&p) {
                    continue;
                }
                if m.pos.dist2(p) <= vision.saturating_mul(vision) && clear(world, m.pos, p) {
                    out.insert(p);
                }
            }
        }
    }
    out
}

/// Every machine any of `team`'s machines sees now, each once.
pub fn team_sees(world: &World, team: TeamId) -> BTreeSet<EntityId> {
    let mut out = BTreeSet::new();
    for seer in world.machines.values().filter(|m| m.team == team) {
        let vision = world.data.model(seer.model.name()).vision;
        for target in world.machines.values() {
            if out.contains(&target.id) {
                continue;
            }
            if seer.pos.dist2(target.pos) <= vision.saturating_mul(vision)
                && clear(world, seer.pos, target.pos)
            {
                out.insert(target.id);
            }
        }
    }
    out
}

/// Whether `team` can see the machine `target` now.
pub fn team_can_see(world: &World, team: TeamId, target: EntityId) -> bool {
    let Some(t) = world.machines.get(&target) else {
        return false;
    };
    world
        .machines
        .values()
        .filter(|m| m.team == team)
        .any(|seer| {
            let vision = world.data.model(seer.model.name()).vision;
            seer.pos.dist2(t.pos) <= vision.saturating_mul(vision) && clear(world, seer.pos, t.pos)
        })
}

/// The previous tick's sounds any of `team`'s machines hears, each once,
/// in emission order.
pub fn team_hears(world: &World, team: TeamId) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, s) in world.sounds_prev.iter().enumerate() {
        let heard = world.machines.values().filter(|m| m.team == team).any(|m| {
            let reach = world
                .data
                .model(m.model.name())
                .hearing
                .saturating_add(s.loudness);
            m.pos.dist2(s.pos) <= reach.saturating_mul(reach)
        });
        if heard {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ray_is_the_docs_walk() {
        assert_eq!(ray(TilePos::new(0, 0), TilePos::new(0, 0)), vec![]);
        assert_eq!(ray(TilePos::new(0, 0), TilePos::new(1, 0)), vec![]);
        assert_eq!(
            ray(TilePos::new(0, 0), TilePos::new(3, 0)),
            vec![TilePos::new(1, 0), TilePos::new(2, 0)]
        );
        assert_eq!(
            ray(TilePos::new(0, 0), TilePos::new(2, 2)),
            vec![TilePos::new(1, 1)]
        );
        // Not symmetric in general (docs/03): a knight's move, by the doc's
        // walk — `err = dx - dy = 1`, `e2 = 2` is not below `dx`, so the
        // first step is along x only.
        let there = ray(TilePos::new(0, 0), TilePos::new(2, 1));
        let back = ray(TilePos::new(2, 1), TilePos::new(0, 0));
        assert_eq!(there, vec![TilePos::new(1, 0)]);
        assert_eq!(back, vec![TilePos::new(1, 1)]);
    }
}
