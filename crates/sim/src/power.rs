//! Power (`docs/08`): a team's grids, how a power step serves them, and a
//! bot's solar and waking. The order inside the tick's step 6 is that
//! doc's, The power step; [`Sim`](crate::tick::Sim) calls these in it.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::world::{EntityId, GridFigures, Model, World};
use lang::{Interrupt, Num};
use std::collections::{BTreeMap, BTreeSet};

/// One grid as traced: its label, its buildings in ascending entity id,
/// and the tiles of the team's wire that are part of it.
pub(crate) struct Grid {
    pub label: EntityId,
    pub buildings: Vec<EntityId>,
    pub wires: Vec<TilePos>,
}

fn add(a: Num, b: Num) -> Num {
    a.checked_add(b).unwrap_or(Num::MAX)
}

/// `a - b`, never below zero: charge is never negative.
fn sub(a: Num, b: Num) -> Num {
    a.checked_sub(b).unwrap_or(Num::ZERO).max(Num::ZERO)
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Join two sets, the lower root kept, so the result does not depend on
/// the order the joins are made in.
fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra.max(rb)] = ra.min(rb);
    }
}

impl World {
    /// The team's grids (`docs/08`, Grids): its conductors — the tiles of
    /// its buildings, sites excluded, and of its wire — joined where they
    /// are adjacent, and from each pylon to every pylon and building of
    /// the team within its span; every set of them that holds a building.
    /// Ascending label. `own` is the team's machines in id order
    /// ([`World::rosters`]), so that tracing one team's grids reads that
    /// team's machines and no other's.
    pub(crate) fn trace_grids(&self, team: TeamId, own: &[EntityId]) -> Vec<Grid> {
        let buildings: Vec<(EntityId, TilePos, Model)> = own
            .iter()
            .filter_map(|id| self.machines.get(id))
            .filter(|m| m.team == team && m.is_building() && m.model != Model::Site)
            .map(|m| (m.id, m.pos, m.model))
            .collect();
        let mut nodes: BTreeMap<TilePos, usize> = BTreeMap::new();
        for (_, pos, _) in &buildings {
            let n = nodes.len();
            nodes.entry(*pos).or_insert(n);
        }
        if let Some(t) = self.teams.get(&team) {
            for p in &t.wire_tiles {
                let n = nodes.len();
                nodes.entry(*p).or_insert(n);
            }
        }
        let mut parent: Vec<usize> = (0..nodes.len()).collect();
        // Adjacent conductors: east and north of each covers every pair.
        for (p, &i) in &nodes {
            let beside = [
                TilePos::new(p.x.saturating_add(1), p.y),
                TilePos::new(p.x, p.y.saturating_add(1)),
            ];
            for q in beside {
                if let Some(&j) = nodes.get(&q) {
                    union(&mut parent, i, j);
                }
            }
        }
        // A pylon's span, whatever stands between.
        let span = self.data.model("pylon").span;
        let reach = span.saturating_mul(span);
        for (_, pylon, model) in &buildings {
            if *model != Model::Pylon {
                continue;
            }
            for (_, other, _) in &buildings {
                if pylon.dist2(*other) <= reach {
                    union(&mut parent, nodes[pylon], nodes[other]);
                }
            }
        }
        // Buildings come in ascending id, so each grid's first is its label.
        let mut by_root: BTreeMap<usize, Grid> = BTreeMap::new();
        for (id, pos, _) in &buildings {
            let root = find(&mut parent, nodes[pos]);
            by_root
                .entry(root)
                .or_insert_with(|| Grid {
                    label: *id,
                    buildings: Vec::new(),
                    wires: Vec::new(),
                })
                .buildings
                .push(*id);
        }
        if let Some(t) = self.teams.get(&team) {
            for p in &t.wire_tiles {
                let root = find(&mut parent, nodes[p]);
                if let Some(g) = by_root.get_mut(&root) {
                    g.wires.push(*p);
                }
            }
        }
        let mut grids: Vec<Grid> = by_root.into_values().collect();
        grids.sort_by_key(|g| g.label);
        grids
    }

    /// A building's need this tick: its drain, and a printer's print drain
    /// while it prints.
    fn need_of(&self, id: EntityId) -> Num {
        let m = &self.machines[&id];
        let md = self.data.model(m.model.name());
        if m.model == Model::Printer && m.printing > 0 {
            add(md.drain, md.print_drain)
        } else {
            md.drain
        }
    }

    /// The charge every battery on a grid holds.
    fn stored_on(&self, grid: &Grid) -> Num {
        grid.buildings
            .iter()
            .fold(Num::ZERO, |sum, id| add(sum, self.machines[id].charge))
    }

    /// The charge a grid's batteries can give: every one but a flooded
    /// building's, which neither gives nor is filled (`docs/08`).
    fn drawable_on(&self, grid: &Grid) -> Num {
        grid.buildings
            .iter()
            .map(|id| &self.machines[id])
            .filter(|m| !m.flooded)
            .fold(Num::ZERO, |sum, m| add(sum, m.charge))
    }

    /// Serve one grid, oldest building first, three times over (`docs/08`,
    /// The power step): each building's own generation covers its own need
    /// and the rest is pooled; each still short is paid from the pool and
    /// then the batteries, or is dark and takes nothing; what is left
    /// fills the batteries. With `moving` false no charge moves and every
    /// building is powered: the form a match is built in. Returns the
    /// grid's figures and whether any building's `powered` changed.
    fn serve(&mut self, grid: &Grid, moving: bool) -> (GridFigures, bool) {
        let mut figures = GridFigures {
            grid: grid.label,
            generation: Num::ZERO,
            drain: Num::ZERO,
            stored: Num::ZERO,
            capacity: Num::ZERO,
        };
        // Its own first.
        let mut pool = Num::ZERO;
        let mut rows: Vec<(EntityId, Num, Num)> = Vec::new();
        // A flooded building takes no part in the three (`docs/08`): it
        // is dark without being tried, and its generation is not pooled.
        let mut flooded: Vec<EntityId> = Vec::new();
        for &id in &grid.buildings {
            let need = self.need_of(id);
            let md = self.data.model(self.machines[&id].model.name());
            figures.generation = add(figures.generation, md.generation);
            figures.drain = add(figures.drain, need);
            figures.capacity = add(figures.capacity, md.battery);
            if self.machines[&id].flooded {
                flooded.push(id);
                continue;
            }
            let kept = md.generation.min(need);
            pool = add(pool, sub(md.generation, kept));
            rows.push((id, kept, sub(need, kept)));
        }
        let mut changed = false;
        for id in flooded {
            let m = self.machines.get_mut(&id).expect("on the grid");
            changed |= m.powered;
            m.powered = false;
            m.grid = Some(grid.label);
        }
        // The shortfalls.
        for (id, kept, short) in rows {
            let powered = if !moving || short <= Num::ZERO {
                true
            } else if add(pool, self.drawable_on(grid)) >= short {
                let from_pool = pool.min(short);
                pool = sub(pool, from_pool);
                let mut rest = sub(short, from_pool);
                for battery in &grid.buildings {
                    if rest <= Num::ZERO {
                        break;
                    }
                    let m = self.machines.get_mut(battery).expect("on the grid");
                    if m.flooded {
                        continue;
                    }
                    let take = m.charge.min(rest);
                    m.charge = sub(m.charge, take);
                    rest = sub(rest, take);
                }
                true
            } else {
                pool = add(pool, kept);
                false
            };
            let m = self.machines.get_mut(&id).expect("on the grid");
            changed |= m.powered != powered;
            m.powered = powered;
            m.grid = Some(grid.label);
        }
        // What is left.
        if moving {
            for id in &grid.buildings {
                let capacity = self.data.model(self.machines[id].model.name()).battery;
                let m = self.machines.get_mut(id).expect("on the grid");
                if m.flooded {
                    continue;
                }
                let give = pool.min(sub(capacity, m.charge));
                m.charge = add(m.charge, give);
                pool = sub(pool, give);
            }
        }
        figures.stored = self.stored_on(grid);
        (figures, changed)
    }

    /// Trace the team's grids, serve them and keep their figures. A site
    /// is on no grid and never powered.
    fn serve_team(&mut self, team: TeamId, own: &[EntityId], moving: bool) {
        let grids = self.trace_grids(team, own);
        let mut changed = false;
        let mut figures = Vec::new();
        let mut live: BTreeSet<TilePos> = BTreeSet::new();
        for grid in &grids {
            let (f, c) = self.serve(grid, moving);
            figures.push(f);
            changed |= c;
            live.extend(grid.wires.iter().copied());
        }
        for id in own {
            if let Some(m) = self.machines.get_mut(id)
                && m.team == team
                && m.model == Model::Site
            {
                m.powered = false;
                m.grid = None;
            }
        }
        if let Some(t) = self.teams.get_mut(&team) {
            t.grids = figures;
            t.live_wires = live;
        }
        // A dark building sees nothing, so what the team sees changed.
        if changed {
            self.sight_changed();
        }
    }

    /// A match's first power state: grids traced, every building powered,
    /// the figures kept as they stand, no charge moved.
    pub(crate) fn trace_and_keep(&mut self, team: TeamId, own: &[EntityId]) {
        self.serve_team(team, own, false);
    }

    /// The power step for one team, up to its prints (`docs/08`): its
    /// grids traced and served, its bots' solar added, and its flat bots
    /// that have the charge woken, which raises `low_power` for each.
    /// `own` is the team's machines in id order ([`World::rosters`]).
    pub(crate) fn power_step(&mut self, team: TeamId, own: &[EntityId]) {
        self.serve_team(team, own, true);
        let bots: Vec<EntityId> = own
            .iter()
            .filter_map(|id| self.machines.get(id))
            .filter(|m| m.team == team && m.model == Model::Bot)
            .map(|m| m.id)
            .collect();
        let md = self.data.model("bot");
        let (generation, capacity) = (md.generation, md.battery);
        for &id in &bots {
            if let Some(m) = self.machines.get_mut(&id) {
                m.charge = add(m.charge, generation).min(capacity);
            }
            self.charge_changed(id);
        }
        let wake = self.data.machines.wake_charge();
        for &id in &bots {
            let Some(m) = self.machines.get_mut(&id) else {
                continue;
            };
            if m.flat && m.charge >= wake {
                m.flat = false;
                if let Some(p) = m.program.as_mut() {
                    p.raise(Interrupt::LowPower);
                }
            }
        }
    }
}
