//! Line of sight, vision and hearing (`docs/03`, Line of sight; `docs/02`,
//! Senses): one integer algorithm on every peer.

use crate::command::TeamId;
use crate::map::TilePos;
#[cfg(debug_assertions)]
use crate::map::{Height, Terrain};
use crate::world::{EntityId, Model, World};
use std::collections::{BTreeMap, BTreeSet};

/// What a team's sight depends on among the machines: every machine's
/// id, position, model (its vision range) and team, and whether it is a
/// dark building, which sees nothing (`docs/08`).
#[cfg(debug_assertions)]
type Basis = Vec<(EntityId, TilePos, Model, TeamId, bool)>;

/// And what it depends on in the ground (Q65; `docs/03`, Line of sight):
/// every tile's terrain, which a mined rock changes, and its height,
/// which a dig and a fill change.
#[cfg(debug_assertions)]
type Ground = Vec<(Terrain, Height)>;

#[cfg(debug_assertions)]
fn ground_of(tile: &crate::world::Tile) -> (Terrain, Height) {
    (tile.terrain, tile.height)
}

/// The tiles each team sees now, kept between queries while the basis
/// they were computed from stands. Sightings are a function of the world
/// and no part of it (`docs/06`, The state hash), so this is neither
/// hashed nor snapshotted. Without it every `see()` recomputed its
/// team's whole field of view, which at a thousand bots a team was
/// nearly all of a tick (I8).
///
/// Whether the basis stands is one comparison: the world counts every
/// change to it (`World::generation`), and the cache holds the count it
/// was built at. Comparing the basis itself on every query was every
/// machine for every `see()`, a hundred million comparisons a tick at
/// ten thousand bots (I9). The price is that a machine placed, removed
/// or moved by anything but `World`'s own methods, or ground that
/// changes without the count being told, leaves the cache stale, so a
/// build with debug assertions still keeps the basis and compares it on
/// every use: every test, the golden replay among them, fails on a site
/// that forgot.
///
/// The two halves of the basis are kept differently. The machines' is
/// taken afresh whenever the count has moved. The ground's is taken
/// once and after that changed only a tile at a time, as the world says
/// each changed (`World::ground_changed`): a dig lands in a tick in
/// which other bots move, so a count that a move bumped would hide a
/// dig that forgot to, and a ground taken afresh with it would never
/// see one (T44).
#[derive(Clone, Default)]
pub struct SightCache {
    /// `World::generation` when `teams` was last emptied.
    built_at: u64,
    pub(crate) teams: BTreeMap<TeamId, BTreeSet<TilePos>>,
    #[cfg(debug_assertions)]
    basis: Basis,
    #[cfg(debug_assertions)]
    ground: Ground,
}

#[cfg(debug_assertions)]
impl SightCache {
    /// The world says the tile at `idx` now stands as it does.
    pub(crate) fn told_of_ground(&mut self, idx: usize, tile: &crate::world::Tile) {
        if let Some(g) = self.ground.get_mut(idx) {
            *g = ground_of(tile);
        }
    }
}

#[cfg(debug_assertions)]
fn basis(world: &World) -> impl Iterator<Item = (EntityId, TilePos, Model, TeamId, bool)> + '_ {
    world
        .machines
        .values()
        .map(|m| (m.id, m.pos, m.model, m.team, m.is_stopped()))
}

#[cfg(debug_assertions)]
fn ground(world: &World) -> impl Iterator<Item = (Terrain, Height)> + '_ {
    world.tiles.iter().map(ground_of)
}

/// Make `world.sight.teams[team]` the tiles `team` sees now.
pub fn ensure_sight(world: &mut World, team: TeamId) {
    if world.sight.built_at != world.generation {
        world.sight.built_at = world.generation;
        world.sight.teams.clear();
        #[cfg(debug_assertions)]
        {
            world.sight.basis = basis(world).collect();
        }
    }
    // The ground as it stood at the first query, a map having at least
    // one tile; from then on only what the world tells of.
    #[cfg(debug_assertions)]
    if world.sight.ground.is_empty() {
        world.sight.ground = ground(world).collect();
    }
    #[cfg(debug_assertions)]
    assert!(
        world.sight.basis.iter().copied().eq(basis(world)),
        "the sight cache is stale: a machine was placed, removed, moved or changed, or a building went dark or was lit, by something other than `World::place`, `remove`, `move_machine`, `remove_team` or the power step"
    );
    #[cfg(debug_assertions)]
    assert!(
        world.sight.ground.iter().copied().eq(ground(world)),
        "the sight cache is stale: the ground changed — a tile's height, or a rock mined away — by something other than `World::set_height` or a mine's landing, which tell it by `World::ground_changed`"
    );
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

/// The height of the ground at `p` in levels, and zero off the map.
fn level(world: &World, p: TilePos) -> i64 {
    world.tile(p).map_or(0, |t| i64::from(t.height.level()))
}

/// A clear line of sight from the seer at `a` to `b`: no tile the ray
/// passes through blocks sight by its terrain, and none stands both
/// above the seer's own ground and above the straight line from the
/// ground of `a` to the ground of `b` (`docs/03`, Height on the line;
/// Q65 as Q68 amended it). Always evaluated from the seer.
///
/// The walk takes `n` steps, the last of them onto `b`, so the tile at
/// step `k` is the `k`th the ray passes through, and it stands above
/// the line where `height(tile) × n > height(a) × (n − k) + height(b) ×
/// k`: the line's height there, with the division by `n` multiplied
/// out, so a tile exactly on the line does not block. Ground no higher
/// than the seer's never blocks, so a machine sees down into a hollow
/// past its bank; looking up, the line is the higher of the two tests
/// and a rim still hides what is above it.
pub fn clear(world: &World, a: TilePos, b: TilePos) -> bool {
    let through = ray(a, b);
    let n = i64::try_from(through.len())
        .unwrap_or(i64::MAX)
        .saturating_add(1);
    let (ha, hb) = (level(world, a), level(world, b));
    (1..n).zip(through).all(|(k, p)| {
        let line = ha
            .saturating_mul(n.saturating_sub(k))
            .saturating_add(hb.saturating_mul(k));
        let h = level(world, p);
        !world.blocks_sight(p) && (h <= ha || h.saturating_mul(n) <= line)
    })
}

/// Whether `b` is within the range of a machine on `a` with vision
/// `vision` (`docs/03`, Height and range): the squared distance is at
/// most `r × r`, where `r = vision + height(a) − height(b)` — a tile
/// further for each level it looks down and one less for each level up
/// — and nothing is where `r` is below zero.
fn in_reach(world: &World, vision: i64, a: TilePos, b: TilePos) -> bool {
    let r = vision
        .saturating_add(level(world, a))
        .saturating_sub(level(world, b));
    r >= 0 && a.dist2(b) <= r.saturating_mul(r)
}

/// Whether a machine on `a` with vision `vision` sees the tile `b`: the
/// one test of sight from a machine to a tile (`docs/03`).
fn in_sight(world: &World, vision: i64, a: TilePos, b: TilePos) -> bool {
    in_reach(world, vision, a, b) && clear(world, a, b)
}

/// Whether the machine `seer` sees the tile `p`: within its model's
/// vision by squared distance, as the two heights stretch or shorten
/// it, with a clear line of sight.
pub fn sees_tile(world: &World, seer: EntityId, p: TilePos) -> bool {
    let Some(m) = world.machines.get(&seer) else {
        return false;
    };
    let vision = world.data.model(m.model.name()).vision;
    in_sight(world, vision, m.pos, p)
}

/// Every tile a team's machines see now, in row-then-column order.
pub fn team_visible_tiles(world: &World, team: TeamId) -> BTreeSet<TilePos> {
    let mut out = BTreeSet::new();
    // A dark building sees nothing (`docs/08`), nor a dry one (`docs/09`).
    for m in world
        .machines
        .values()
        .filter(|m| m.team == team && !m.is_stopped())
    {
        let vision = world.data.model(m.model.name()).vision;
        // The furthest it can see is to a tile at height zero: its vision
        // and every level it stands above that (`in_reach`). That is a
        // wider square than its vision alone, so the range is tested
        // first, which is cheap, and the set and the ray only within it.
        let furthest = vision.saturating_add(level(world, m.pos));
        let r = i32::try_from(furthest).unwrap_or(i32::MAX);
        for dy in r.saturating_neg()..=r {
            for dx in r.saturating_neg()..=r {
                let p = TilePos::new(m.pos.x.saturating_add(dx), m.pos.y.saturating_add(dy));
                if !world.map.in_bounds(p) || !in_reach(world, vision, m.pos, p) || out.contains(&p)
                {
                    continue;
                }
                if clear(world, m.pos, p) {
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
        // A dark building hears nothing (`docs/08`), nor a dry one
        // (`docs/09`).
        let heard = world
            .machines
            .values()
            .filter(|m| m.team == team && !m.is_stopped())
            .any(|m| {
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
                if in_sight(world, vision, seer.pos, target.pos) {
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
        let before = w.sight.teams[&t0].clone();
        w.move_machine(b, TilePos::new(-1, 2));
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert_eq!(team_sees(w, t1), team_sees_by_definition(w, t1));
        assert_ne!(w.sight.teams[&t0], before, "the move changed the view");
        // So does a new machine, and a removed one.
        let c = w.place(Model::Bot, t0, TilePos::new(3, -3), Some("bot".into()));
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert!(team_can_see(w, t0, c));
        w.remove(c);
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert!(!team_can_see(w, t0, c));
        for target in w.machines.keys().copied().collect::<Vec<_>>() {
            let by_definition = team_sees_by_definition(w, t1).contains(&target);
            assert_eq!(team_can_see(w, t1, target), by_definition, "{target:?}");
        }
        // A conversion: a site, which sees two tiles, becomes a printer,
        // which sees eight, and the team's view grows with it. The site
        // stands on the level ground beside the lake and not in its bed,
        // two levels down, where the ground around stands above every
        // line out and a printer sees little more than a site (Q65).
        let site = w.place(Model::Site, t0, TilePos::new(-2, -3), None);
        w.machines.get_mut(&site).unwrap().becomes = Some(Model::Printer);
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        let before = w.sight.teams[&t0].clone();
        s.finish_site(site);
        let w = &mut s.world;
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
        assert!(
            w.sight.teams[&t0].len() > before.len(),
            "the building sees further than its site did"
        );
        // A team going out: its machines leave, and it sees nothing.
        assert!(!team_sees(w, t1).is_empty());
        w.teams.get_mut(&t1).unwrap().resigned = true;
        s.out_teams();
        let w = &mut s.world;
        assert!(w.teams[&t1].out);
        assert_eq!(team_sees(w, t1), BTreeSet::new());
        assert_eq!(team_sees(w, t1), team_sees_by_definition(w, t1));
        assert_eq!(team_sees(w, t0), team_sees_by_definition(w, t0));
    }

    /// The price of the generation counter is that a change made behind
    /// `World`'s back leaves the cache stale. A build with debug
    /// assertions, which every test is, catches the next query after it.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "the sight cache is stale")]
    fn a_machine_moved_behind_the_worlds_back_is_caught_at_the_next_query() {
        let data = Data::load().unwrap();
        let mut s = Sim::new(Map::parse(MAP, &data).unwrap());
        let w = &mut s.world;
        let b = w.place(
            Model::Bot,
            TeamId(0),
            TilePos::new(-3, 2),
            Some("bot".into()),
        );
        team_sees(w, TeamId(0));
        w.machines.get_mut(&b).unwrap().pos = TilePos::new(-1, 2);
        team_sees(w, TeamId(0));
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
