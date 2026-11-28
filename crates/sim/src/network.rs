//! The water network (`docs/09`, The water network; Q67): a team's
//! buildings joined by adjacency, its pipe and its aqueducts, and the
//! network step that serves them — pumps lifting water off the map,
//! buildings using it, spouts pouring it back and tanks holding the rest.
//! The order inside it is that doc's, The network step;
//! [`Sim`](crate::tick::Sim) runs it in step 7, before the water step.
//!
//! Every amount here is a whole number of parts, and every part a pump
//! lifts leaves a tile for a tank, a tile or the ground water: the tiles,
//! the ground water and the tanks together never change
//! ([`World::water_total`]).

use crate::command::TeamId;
use crate::map::{Terrain, TilePos};
use crate::power::Grid;
use crate::world::{EntityId, Model, NetworkFigures, World};
use std::collections::{BTreeMap, BTreeSet};

/// A pump with a source this tick: the tile it lifts from, and what it
/// may still lift of its `pump_rate`.
struct Lift {
    source: usize,
    left: u32,
}

/// The team's pumps that found a source, by entity id.
type Lifts = BTreeMap<EntityId, Lift>;

fn sum(a: u64, b: u32) -> u64 {
    a.saturating_add(u64::from(b))
}

impl World {
    /// The team's water networks (`docs/09`, Networks): `docs/08`'s grids
    /// in every respect but what joins — its pipe where a grid has its
    /// wire, and an aqueduct's span where a grid has a pylon's. Ascending
    /// label.
    pub(crate) fn trace_networks(&self, team: TeamId, own: &[EntityId]) -> Vec<Grid> {
        let none = BTreeSet::new();
        let pipe = self.teams.get(&team).map_or(&none, |t| &t.pipe_tiles);
        self.trace_joined(team, own, pipe, Model::Aqueduct)
    }

    /// A building's need for water this tick: its `water_drain`, and a
    /// printer's `print_water` while it prints, less its `well`, and
    /// nothing if that is below zero.
    fn water_need_of(&self, id: EntityId) -> u32 {
        let m = &self.machines[&id];
        let md = self.data.model(m.model.name());
        let gross = if m.model == Model::Printer && m.printing > 0 {
            md.water_drain.saturating_add(md.print_water)
        } else {
            md.water_drain
        };
        gross.saturating_sub(md.well)
    }

    /// A pump's source: of the wet tiles beside it, the one holding the
    /// most water, the first of `n`, `e`, `s`, `w` among equals. None for
    /// a building that lifts nothing, is not powered or is flooded, or
    /// has no wet tile beside it.
    fn source_of(&self, id: EntityId) -> Option<Lift> {
        let m = &self.machines[&id];
        let rate = self.data.model(m.model.name()).pump_rate;
        if rate == 0 || !m.powered || m.flooded {
            return None;
        }
        let mut best: Option<(u32, usize)> = None;
        for p in m.pos.neighbours() {
            let Some(idx) = self.tile_index(p) else {
                continue;
            };
            let water = self.tiles[idx].water;
            if self.is_wet(idx) && best.is_none_or(|(most, _)| water > most) {
                best = Some((water, idx));
            }
        }
        best.map(|(_, source)| Lift { source, left: rate })
    }

    /// What a draw on the network could take now: what its pumps may
    /// still lift — each no more than its source holds once the pumps
    /// before it have lifted, since two may share a tile — and, with
    /// `tanks`, what its tanks hold. A flooded building's tank gives
    /// nothing.
    fn drawable(&self, net: &Grid, lifts: &Lifts, tanks: bool) -> u64 {
        let mut taken: BTreeMap<usize, u32> = BTreeMap::new();
        let mut total = 0u64;
        for id in &net.buildings {
            let Some(lift) = lifts.get(id) else {
                continue;
            };
            let gone = taken.entry(lift.source).or_insert(0);
            let can = lift
                .left
                .min(self.tiles[lift.source].water.saturating_sub(*gone));
            *gone = gone.saturating_add(can);
            total = sum(total, can);
        }
        if tanks {
            for m in net.buildings.iter().map(|id| &self.machines[id]) {
                if !m.flooded {
                    total = sum(total, m.water);
                }
            }
        }
        total
    }

    /// Draw up to `want` parts from the network: lifted by its pumps in
    /// ascending entity id, each as far as it may and never more than its
    /// source holds as it lifts, and what they cannot give taken, with
    /// `tanks`, from its tanks in ascending entity id. Returns the parts
    /// drawn, which the caller owes a home. A tile lifted from is
    /// disturbed.
    fn draw(&mut self, net: &Grid, lifts: &mut Lifts, want: u32, tanks: bool) -> u32 {
        let mut rest = want;
        for id in &net.buildings {
            if rest == 0 {
                break;
            }
            let Some(lift) = lifts.get_mut(id) else {
                continue;
            };
            let take = lift.left.min(self.tiles[lift.source].water).min(rest);
            if take == 0 {
                continue;
            }
            lift.left = lift.left.saturating_sub(take);
            let source = lift.source;
            self.tiles[source].water = self.tiles[source].water.saturating_sub(take);
            self.stir(source);
            rest = rest.saturating_sub(take);
        }
        if tanks {
            for id in &net.buildings {
                if rest == 0 {
                    break;
                }
                let m = self.machines.get_mut(id).expect("on the network");
                if m.flooded {
                    continue;
                }
                let take = m.water.min(rest);
                m.water = m.water.saturating_sub(take);
                rest = rest.saturating_sub(take);
            }
        }
        want.saturating_sub(rest)
    }

    /// Serve one network, oldest building first, three times over
    /// (`docs/09`, The network step): each need drawn in full, to the
    /// ground water, or its building dry; each pouring spout's water
    /// drawn and put on the tile it faces; and the tanks filled by what
    /// the pumps may still lift. With `moving` false no water moves and
    /// every building is watered: the form a match is built in. Returns
    /// the network's figures and whether any building's `watered`
    /// changed.
    fn serve_network(
        &mut self,
        net: &Grid,
        lifts: &mut Lifts,
        moving: bool,
    ) -> (NetworkFigures, bool) {
        let mut figures = NetworkFigures {
            network: net.label,
            supply: 0,
            drain: 0,
            stored: 0,
            capacity: 0,
        };
        let mut changed = false;
        // The needs.
        for &id in &net.buildings {
            let need = self.water_need_of(id);
            let md = self.data.model(self.machines[&id].model.name());
            figures.drain = sum(figures.drain, need);
            figures.capacity = sum(figures.capacity, md.tank);
            if lifts.contains_key(&id) {
                figures.supply = sum(figures.supply, md.pump_rate);
            }
            // A flooded building takes no part: it is dry without being
            // tried.
            let watered = if self.machines[&id].flooded {
                false
            } else if !moving || need == 0 {
                true
            } else if self.drawable(net, lifts, true) >= u64::from(need) {
                // What a building uses goes to the ground water.
                let used = self.draw(net, lifts, need, true);
                self.ground_water = self.ground_water.saturating_add(u64::from(used));
                true
            } else {
                false
            };
            let m = self.machines.get_mut(&id).expect("on the network");
            changed |= m.watered != watered;
            m.watered = watered;
            m.network = Some(net.label);
        }
        if moving {
            // The spouts: one that is dark, dry or flooded pours nothing,
            // and one facing rock or the map's edge pours nowhere.
            for &id in &net.buildings {
                let m = &self.machines[&id];
                let Some(dir) = m.pouring else {
                    continue;
                };
                if m.flooded || !m.powered || !m.watered {
                    continue;
                }
                let rate = self.data.model(m.model.name()).pour_rate;
                let Some(onto) = self
                    .tile_index(dir.of(m.pos))
                    .filter(|idx| self.tiles[*idx].terrain == Terrain::Ground)
                else {
                    continue;
                };
                // Never more than the tile can count, so no part is lost.
                let room = u32::MAX.saturating_sub(self.tiles[onto].water);
                let poured = self.draw(net, lifts, rate.min(room), true);
                if poured > 0 {
                    self.tiles[onto].water = self.tiles[onto].water.saturating_add(poured);
                    self.stir(onto);
                }
            }
            // What is left: the tanks, by what the pumps may still lift.
            for &id in &net.buildings {
                let m = &self.machines[&id];
                if m.flooded {
                    continue;
                }
                let room = self.data.model(m.model.name()).tank.saturating_sub(m.water);
                if room == 0 {
                    continue;
                }
                let lifted = self.draw(net, lifts, room, false);
                let m = self.machines.get_mut(&id).expect("on the network");
                m.water = m.water.saturating_add(lifted);
            }
        }
        figures.stored = net
            .buildings
            .iter()
            .fold(0, |held, id| sum(held, self.machines[id].water));
        (figures, changed)
    }

    /// Trace the team's networks, find its pumps' sources, serve the
    /// networks and keep their figures. A site is on no network and never
    /// watered.
    fn serve_team_water(&mut self, team: TeamId, own: &[EntityId], moving: bool) {
        let networks = self.trace_networks(team, own);
        // Each pump finds its source, in ascending entity id, from the
        // water as it stands before any of the team's pumps has lifted.
        let mut lifts = Lifts::new();
        if moving {
            for net in &networks {
                for &id in &net.buildings {
                    if let Some(lift) = self.source_of(id) {
                        lifts.insert(id, lift);
                    }
                }
            }
        }
        let mut changed = false;
        let mut figures = Vec::new();
        let mut live: BTreeSet<TilePos> = BTreeSet::new();
        for net in &networks {
            let (f, c) = self.serve_network(net, &mut lifts, moving);
            figures.push(f);
            changed |= c;
            live.extend(net.lines.iter().copied());
        }
        for id in own {
            if let Some(m) = self.machines.get_mut(id)
                && m.team == team
                && m.model == Model::Site
            {
                m.watered = false;
                m.network = None;
            }
        }
        if let Some(t) = self.teams.get_mut(&team) {
            t.networks = figures;
            t.live_pipes = live;
        }
        // A dry building sees nothing, so what the team sees changed.
        if changed {
            self.sight_changed();
        }
    }

    /// A match's first water networks: traced, every building watered,
    /// the figures kept as they stand, no water moved.
    pub(crate) fn trace_and_keep_networks(&mut self, team: TeamId, own: &[EntityId]) {
        self.serve_team_water(team, own, false);
    }

    /// The network step for one team (`docs/09`): its networks traced,
    /// its pumps' sources found, and each network served. `own` is the
    /// team's machines in id order ([`World::rosters`]).
    pub(crate) fn network_step(&mut self, team: TeamId, own: &[EntityId]) {
        self.serve_team_water(team, own, true);
    }
}
