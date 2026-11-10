//! Line of sight, vision and hearing (`docs/03`, Line of sight; `docs/02`,
//! Senses): one integer algorithm on every peer.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::world::{EntityId, Model, World};
use std::collections::{BTreeMap, BTreeSet};

/// Everything a team's sight depends on: every machine's id, position,
/// model (its vision range) and team. Terrain, the other input, never
/// changes after the map loads.
type Basis = Vec<(EntityId, TilePos, Model, TeamId)>;

/// The tiles each team sees now, kept between queries while the basis
/// they were computed from stands. Sightings are a function of the world
/// and no part of it (`docs/06`, The state hash), so this is neither
/// hashed nor snapshotted; it is checked against the world on every use,
/// so any path that moves, places or removes a machine invalidates it
/// without having to know it exists. Without it every `see()` recomputed
/// its team's whole field of view, which at a thousand bots a team was
/// nearly all of a tick (I8).
#[derive(Default)]
pub struct SightCache {
    basis: Basis,
    pub(crate) teams: BTreeMap<TeamId, BTreeSet<TilePos>>,
}

fn basis(world: &World) -> impl Iterator<Item = (EntityId, TilePos, Model, TeamId)> + '_ {
    world
        .machines
        .values()
        .map(|m| (m.id, m.pos, m.model, m.team))
}

/// Make `world.sight.teams[team]` the tiles `team` sees now.
pub fn ensure_sight(world: &mut World, team: TeamId) {
    if !world.sight.basis.iter().copied().eq(basis(world)) {
        world.sight.basis = basis(world).collect();
        world.sight.teams.clear();
    }
    if !world.sight.teams.contains_key(&team) {
        let visible = team_visible_tiles(world, team);
        world.sight.teams.insert(team, visible);
    }
}

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

/// Every machine any of `team`'s machines sees now, each once: the
/// machines standing on a tile the team sees, since a machine is seen by
/// exactly the test that sees its tile (`sees_tile`).
pub fn team_sees(world: &mut World, team: TeamId) -> BTreeSet<EntityId> {
    ensure_sight(world, team);
    let visible = &world.sight.teams[&team];
    world
        .machines
        .values()
        .filter(|m| visible.contains(&m.pos))
        .map(|m| m.id)
        .collect()
}

/// Whether `team` can see the machine `target` now.
pub fn team_can_see(world: &mut World, team: TeamId, target: EntityId) -> bool {
    ensure_sight(world, team);
    let visible = &world.sight.teams[&team];
    world
        .machines
        .get(&target)
        .is_some_and(|t| visible.contains(&t.pos))
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
    use crate::data::Data;
    use crate::map::Map;
    use crate::tick::Sim;

    /// The definition, seer by seer and target by target: what `team_sees`
    /// computed before the sight cache, kept as its oracle.
    fn team_sees_by_definition(world: &World, team: TeamId) -> BTreeSet<EntityId> {
        let mut out = BTreeSet::new();
        for seer in world.machines.values().filter(|m| m.team == team) {
            let vision = world.data.model(seer.model.name()).vision;
            for target in world.machines.values() {
                if seer.pos.dist2(target.pos) <= vision.saturating_mul(vision)
                    && clear(world, seer.pos, target.pos)
                {
                    out.insert(target.id);
                }
            }
        }
        out
    }

    const MAP: &str = r#"
name = "sight"
speed = 10
width = 12
height = 8
terrain = """
............
....##......
.o..##......
............
............
......o.....
..~~........
............
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-5, 2]
[[team]]
start = [4, -2]
"#;

    #[test]
    fn the_sight_cache_agrees_with_the_definition_and_notices_every_change() {
        let data = Data::load().unwrap();
        let mut s = Sim::new(Map::parse(MAP, &data).unwrap());
        let w = &mut s.world;
        let (t0, t1) = (TeamId(0), TeamId(1));
        // Bots of both teams, some behind the rock from the other side.
        let b = w.place(Model::Bot, t0, TilePos::new(-3, 2), Some("bot".into()));
        w.place(Model::Bot, t1, TilePos::new(1, 2), Some("bot".into()));
        w.place(Model::Bot, t1, TilePos::new(-2, -2), Some("bot".into()));
        for team in [t0, t1] {
            assert_eq!(team_sees(w, team), team_sees_by_definition(w, team));
        }
        // A move changes the basis, so the cached set is dropped.
        w.machines.get_mut(&b).unwrap().pos = TilePos::new(-1, 2);
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert_eq!(team_sees(w, t1), team_sees_by_definition(w, t1));
        // So does a new machine, and a removed one.
        let c = w.place(Model::Bot, t0, TilePos::new(3, -3), Some("bot".into()));
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert!(team_can_see(w, t0, c));
        w.machines.remove(&c);
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert!(!team_can_see(w, t0, c));
        for target in w.machines.keys().copied().collect::<Vec<_>>() {
            let by_definition = team_sees_by_definition(w, t1).contains(&target);
            assert_eq!(team_can_see(w, t1, target), by_definition, "{target:?}");
        }
    }

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
