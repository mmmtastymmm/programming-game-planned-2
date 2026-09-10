//! The tick (`docs/06`, The tick): eight steps in a fixed order, and
//! `Sim::step` is the only entry point that changes the world.

use crate::command::{Bundle, Command, CommandKind, PlanKind, TeamId};
use crate::data::Data;
use crate::hash::Fnv1a;
use crate::host::GameHost;
use crate::map::{Map, TilePos, row_then_column};
use crate::sense;
use crate::world::{
    Effect, EntityId, LoadedBundle, MachineRecord, Model, PickSource, Plan, Slot, TileMemory,
    TileState, World,
};
use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, Event, Interrupt, Limits, Num, Program, Slice, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// What a tick reported back to the driver.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepReport {
    /// A `SetSpeed` applied this tick, the last one if several.
    pub speed: Option<u64>,
    /// Commands dropped because they could not apply on their tick.
    pub dropped: Vec<Command>,
    /// The winner, or `None` for a draw, once the match has ended.
    pub ended: Option<Option<TeamId>>,
}

pub struct Sim {
    pub world: World,
    ended: Option<Option<TeamId>>,
}

impl Sim {
    /// A match on `map` with the shipped tables.
    pub fn new(map: Map) -> Sim {
        let data = Rc::new(Data::load().expect("the shipped tables load"));
        let costs = Rc::new(Costs::parse(COSTS_TOML).expect("costs"));
        let limits = Rc::new(Limits::parse(LIMITS_TOML).expect("limits"));
        Sim {
            world: World::new(map, data, costs, limits),
            ended: None,
        }
    }

    pub fn ended(&self) -> Option<Option<TeamId>> {
        self.ended
    }

    /// Whether a command can ever apply (`docs/06`, Validation at
    /// submission): checked before it enters the log.
    pub fn validate(&self, c: &Command) -> Result<(), String> {
        let w = &self.world;
        if !w.teams.contains_key(&c.sender) {
            return Err(format!("no team {}", c.sender.0));
        }
        match &c.kind {
            CommandKind::Deploy { deployment, bundle } => {
                if !w.teams[&c.sender].deployments.contains_key(deployment) {
                    return Err(format!("no deployment `{deployment}`"));
                }
                load_bundle(bundle, &w.limits).map(|_| ())
            }
            CommandKind::Mark { at, plan, value } => {
                if !w.map.in_bounds(*at) {
                    return Err(format!("({}, {}) is off the map", at.x, at.y));
                }
                match (plan, value) {
                    (PlanKind::Building, Some(v)) if v == "printer" => {
                        Err("a building plan cannot name `printer` (Q31)".into())
                    }
                    (PlanKind::Building, Some(v))
                        if Model::from_name(v)
                            .is_none_or(|m| m == Model::Site || m == Model::Bot) =>
                    {
                        Err(format!("`{v}` is not a building model"))
                    }
                    (PlanKind::Paint, Some(v)) if !w.data.machines.colors.contains(v) => {
                        Err(format!("`{v}` is not a color"))
                    }
                    (PlanKind::Overlay, Some(v)) if !w.data.world.overlays.contains(v) => {
                        Err(format!("`{v}` is not an overlay label"))
                    }
                    _ => Ok(()),
                }
            }
            CommandKind::Unmark { at, .. } => {
                if !w.map.in_bounds(*at) {
                    return Err(format!("({}, {}) is off the map", at.x, at.y));
                }
                Ok(())
            }
            CommandKind::SetSpeed(s) => {
                if !w.data.world.speed_steps.contains(s) {
                    return Err(format!("{s} is not a speed step"));
                }
                Ok(())
            }
            CommandKind::Resign => Ok(()),
        }
    }

    /// Run one tick with its full command set, in the doc's eight steps.
    pub fn step(&mut self, commands: &[Command]) -> StepReport {
        let mut report = StepReport::default();
        if self.ended.is_some() {
            return report;
        }
        let w = &mut self.world;
        w.tick = w.tick.checked_add(1).expect("tick counter");
        w.converted_this_tick.clear();
        w.pending_damage.clear();
        w.dead.clear();
        for m in w.machines.values_mut() {
            m.log.clear();
        }
        // 1. Commands, by sender then submission order.
        let mut ordered: Vec<&Command> = commands.iter().collect();
        ordered.sort_by(|a, b| a.sender.cmp(&b.sender).then(a.seq.cmp(&b.seq)));
        for c in ordered {
            if let Err(()) = self.apply(c, &mut report) {
                report.dropped.push(c.clone());
            }
        }
        // 2. Every machine's slice, ascending id.
        let ids: Vec<EntityId> = self.world.machines.keys().copied().collect();
        for id in ids {
            self.run_slice(id);
        }
        // 3. Completions, ascending id.
        let ids: Vec<EntityId> = self.world.machines.keys().copied().collect();
        for id in ids {
            self.complete(id);
        }
        // 4. Damage and death.
        self.damage();
        // 5. Out teams.
        self.out_teams();
        // 6. Regrowth.
        self.regrow();
        // 7. The vision pass.
        self.vision_pass();
        // The sounds emitted this tick are heard next tick.
        let w = &mut self.world;
        w.sounds_prev = std::mem::take(&mut w.sounds_now);
        // The match ends on the first tick with at most one team standing.
        let standing: Vec<TeamId> = w.teams.values().filter(|t| !t.out).map(|t| t.id).collect();
        if standing.len() <= 1 {
            self.ended = Some(standing.first().copied());
            report.ended = self.ended;
        }
        report
    }

    fn apply(&mut self, c: &Command, report: &mut StepReport) -> Result<(), ()> {
        let w = &mut self.world;
        let Some(team) = w.teams.get(&c.sender) else {
            return Err(());
        };
        if team.out {
            return Err(());
        }
        match &c.kind {
            CommandKind::Deploy { deployment, bundle } => {
                let loaded = load_bundle(bundle, &w.limits).map_err(|_| ())?;
                if !w.teams[&c.sender].deployments.contains_key(deployment) {
                    return Err(());
                }
                if w.is_unlocked(c.sender, deployment) {
                    self.deploy(c.sender, deployment, loaded);
                } else {
                    // Waits in the slot until the color unlocks (docs/02).
                    let slot = w
                        .teams
                        .get_mut(&c.sender)
                        .expect("team")
                        .deployments
                        .get_mut(deployment)
                        .expect("slot");
                    slot.pending = Some(loaded);
                }
                Ok(())
            }
            CommandKind::Mark { at, plan, value } => {
                if !w.map.in_bounds(*at) {
                    return Err(());
                }
                let p = match value {
                    Some(v) => Plan::Set(v.clone()),
                    None => Plan::Clear,
                };
                let plans = w.plans_mut(c.sender, *at).ok_or(())?;
                match plan {
                    PlanKind::Paint => plans.paint = Some(p),
                    PlanKind::Overlay => plans.overlay = Some(p),
                    PlanKind::Building => plans.building = Some(p),
                }
                Ok(())
            }
            CommandKind::Unmark { at, plan } => {
                let plans = w.plans_mut(c.sender, *at).ok_or(())?;
                match plan {
                    PlanKind::Paint => plans.paint = None,
                    PlanKind::Overlay => plans.overlay = None,
                    PlanKind::Building => plans.building = None,
                }
                w.tidy_plans(*at);
                Ok(())
            }
            CommandKind::SetSpeed(s) => {
                if !w.data.world.speed_steps.contains(s) {
                    return Err(());
                }
                w.speed = *s;
                report.speed = Some(*s);
                Ok(())
            }
            CommandKind::Resign => {
                w.teams.get_mut(&c.sender).expect("team").resigned = true;
                Ok(())
            }
        }
    }

    /// Write a bundle into a slot and redeploy every machine on it (Q11).
    fn deploy(&mut self, team: TeamId, deployment: &str, loaded: LoadedBundle) {
        let w = &mut self.world;
        let slot = w
            .teams
            .get_mut(&team)
            .expect("team")
            .deployments
            .get_mut(deployment)
            .expect("slot");
        slot.bundle = Some(loaded);
        for m in w.machines.values_mut() {
            if m.team == team
                && m.deployment.as_deref() == Some(deployment)
                && let Some(p) = &mut m.program
            {
                p.raise(Interrupt::Redeploy);
            }
        }
    }

    /// Colors that unlocked with a pending deploy take it now (docs/02,
    /// Locking).
    fn apply_pending_deploys(&mut self, team: TeamId) {
        let unlocked = self.world.unlocked_colors(team);
        for color in unlocked {
            let pending = self
                .world
                .teams
                .get_mut(&team)
                .and_then(|t| t.deployments.get_mut(&color))
                .and_then(|d| d.pending.take());
            if let Some(b) = pending {
                self.deploy(team, &color, b);
            }
        }
    }

    /// Step 2 for one machine: take its program out of its body, run it
    /// against a host over the world, put it back, act on its events.
    fn run_slice(&mut self, id: EntityId) {
        let Some(mut program) = self
            .world
            .machines
            .get_mut(&id)
            .and_then(|m| m.program.take())
        else {
            return;
        };
        if program.is_dead() {
            self.world.machines.get_mut(&id).expect("machine").program = Some(program);
            return;
        }
        program.tick = self.world.tick;
        let slice = {
            let mut host = GameHost {
                world: &mut self.world,
                id,
            };
            program.run_slice(&mut host)
        };
        let events = program.take_events();
        if let Some(m) = self.world.machines.get_mut(&id) {
            m.program = Some(program);
        }
        for ev in events {
            if let Event::Death = ev {
                self.world.dead.insert(id);
            }
        }
        let _ = slice;
    }

    /// Step 3 for one machine: count its action down and complete it, and
    /// a site's construction.
    fn complete(&mut self, id: EntityId) {
        let Some(m) = self.world.machines.get_mut(&id) else {
            return;
        };
        if let Some(remaining) = m.construction {
            let left = remaining.saturating_sub(1);
            if left == 0 {
                self.finish_site(id);
            } else {
                m.construction = Some(left);
            }
            return;
        }
        let Some(action) = m.action.as_mut() else {
            return;
        };
        if action.remaining > 0 {
            action.remaining = action.remaining.saturating_sub(1);
            if action.remaining > 0 {
                return;
            }
        }
        let effect = m.action.take().expect("action").effect;
        let result = self.finish(id, effect);
        if let Some(p) = self
            .world
            .machines
            .get_mut(&id)
            .and_then(|m| m.program.as_mut())
        {
            p.resume(result);
        }
    }

    /// The effect of an action completing (`docs/02`, Actions).
    fn finish(&mut self, id: EntityId, effect: Effect) -> Value {
        let w = &mut self.world;
        match effect {
            Effect::Move { to } => {
                if let Some(m) = w.machines.get_mut(&id) {
                    m.pos = to;
                }
            }
            Effect::Pick { kind, from } => {
                let amount = {
                    let host = GameHost { world: w, id };
                    host.pick_amount(&kind, &from)
                };
                let w = &mut self.world;
                if amount > Num::ZERO {
                    match &from {
                        PickSource::Deposit(p) => {
                            if let Some(d) = w.tile_mut(*p).and_then(|t| t.deposit.as_mut()) {
                                let a = d.amount.entry(kind.clone()).or_insert(Num::ZERO);
                                *a = a.checked_sub(amount).unwrap_or(Num::ZERO);
                            }
                        }
                        PickSource::Depot(depot) => {
                            if let Some(d) = w.machines.get_mut(depot) {
                                let s = d.store.entry(kind.clone()).or_insert(Num::ZERO);
                                *s = s.checked_sub(amount).unwrap_or(Num::ZERO);
                            }
                        }
                    }
                    if let Some(m) = w.machines.get_mut(&id) {
                        let l = m.load.entry(kind).or_insert(Num::ZERO);
                        *l = l.checked_add(amount).unwrap_or(Num::MAX);
                    }
                }
            }
            Effect::Drop { kind, into } => {
                let load = w
                    .machines
                    .get(&id)
                    .and_then(|m| m.load.get(&kind).copied())
                    .unwrap_or(Num::ZERO);
                let free = w
                    .machines
                    .get(&into)
                    .map(|t| {
                        t.store_capacity
                            .get(&kind)
                            .copied()
                            .unwrap_or(Num::ZERO)
                            .checked_sub(t.store.get(&kind).copied().unwrap_or(Num::ZERO))
                            .unwrap_or(Num::ZERO)
                    })
                    .unwrap_or(Num::ZERO);
                let amount = load.min(free).max(Num::ZERO);
                if amount > Num::ZERO {
                    if let Some(m) = w.machines.get_mut(&id) {
                        let l = m.load.entry(kind.clone()).or_insert(Num::ZERO);
                        *l = l.checked_sub(amount).unwrap_or(Num::ZERO);
                    }
                    let full = if let Some(t) = w.machines.get_mut(&into) {
                        let s = t.store.entry(kind.clone()).or_insert(Num::ZERO);
                        *s = s.checked_add(amount).unwrap_or(Num::MAX);
                        t.model == Model::Site
                            && t.construction.is_none()
                            && t.store_capacity.iter().all(|(k, cap)| {
                                t.store.get(k).copied().unwrap_or(Num::ZERO) >= *cap
                            })
                    } else {
                        false
                    };
                    if full {
                        // A full site starts construction (docs/02, build).
                        let becomes = w.machines[&into].becomes.unwrap_or(Model::Depot);
                        let ticks = w.data.model(becomes.name()).build_ticks;
                        if let Some(t) = w.machines.get_mut(&into) {
                            t.construction = Some(ticks.max(1));
                        }
                    }
                }
            }
            Effect::Print { color } => {
                let Some(printer) = w.machines.get(&id) else {
                    return Value::None;
                };
                let (team, pos) = (printer.team, printer.pos);
                let free = pos
                    .neighbours()
                    .into_iter()
                    .find(|p| w.passable(*p) && !w.occupied(*p));
                if let Some(p) = free
                    && w.teams[&team]
                        .deployments
                        .get(&color)
                        .is_some_and(|d| d.bundle.is_some())
                {
                    w.place(Model::Bot, team, p, Some(color));
                }
            }
            Effect::Build { model, at, remove } => {
                if let Some(old) = remove {
                    self.remove_machine(old);
                }
                let w = &mut self.world;
                if !w.buildable(at) || w.machines.values().any(|m| m.pos == at) {
                    return Value::None;
                }
                let team = w.machines[&id].team;
                let site = w.place(Model::Site, team, at, None);
                let cost = w.data.model(model.name()).cost.clone();
                if let Some(s) = w.machines.get_mut(&site) {
                    s.becomes = Some(model);
                    s.name = format!("{}{}", model.name(), site.0);
                    for k in w.data.machines.kinds.clone() {
                        s.store_capacity
                            .insert(k.clone(), cost.get(&k).copied().unwrap_or(Num::ZERO));
                    }
                }
                // Placing the site consumes the plan.
                if let Some(pl) = w.plans_mut(team, at) {
                    pl.building = None;
                }
                w.tidy_plans(at);
                w.sound("build", at);
                // A site with nothing to hold is complete at once.
                let free_capacity = w.machines[&site]
                    .store_capacity
                    .values()
                    .any(|c| *c > Num::ZERO);
                if !free_capacity {
                    let ticks = w.data.model(model.name()).build_ticks;
                    w.machines.get_mut(&site).expect("site").construction = Some(ticks.max(1));
                }
            }
            Effect::Paint { at, slot, value } => {
                let team = w.machines[&id].team;
                if let Some(t) = w.tile_mut(at) {
                    match slot {
                        Slot::Paint => t.paint = value.clone(),
                        Slot::Overlay => t.overlay = value.clone(),
                    }
                }
                // The plan this realised is consumed: a set plan by a set,
                // a clear plan by a clear.
                if let Some(pl) = w.plans_mut(team, at) {
                    let plan = match slot {
                        Slot::Paint => &mut pl.paint,
                        Slot::Overlay => &mut pl.overlay,
                    };
                    let consumed = matches!(
                        (&plan, &value),
                        (Some(Plan::Set(_)), Some(_)) | (Some(Plan::Clear), None)
                    );
                    if consumed {
                        *plan = None;
                    }
                }
                w.tidy_plans(at);
            }
            Effect::Deconstruct { target } => {
                let at = w.machines.get(&target).map(|m| m.pos);
                let team = w.machines[&id].team;
                self.remove_machine(target);
                let w = &mut self.world;
                if let Some(at) = at {
                    if let Some(pl) = w.plans_mut(team, at)
                        && matches!(pl.building, Some(Plan::Clear))
                    {
                        pl.building = None;
                    }
                    w.tidy_plans(at);
                }
            }
            Effect::Attack { target } => {
                let team = w.machines[&id].team;
                let here = w.machines[&id].pos;
                let md = w.data.model("bot").clone();
                if let Some(t) = w.machines.get(&target) {
                    let in_range =
                        here.dist2(t.pos) <= md.attack_range.saturating_mul(md.attack_range);
                    let tpos = t.pos;
                    if in_range
                        && sense::clear(w, here, tpos)
                        && sense::team_can_see(w, team, target)
                    {
                        let d = w.pending_damage.entry(target).or_insert(Num::ZERO);
                        *d = d.checked_add(md.attack_damage).unwrap_or(Num::MAX);
                    }
                }
            }
            Effect::Convert { target } => {
                let (team, here) = {
                    let m = &w.machines[&id];
                    (m.team, m.pos)
                };
                let old_team = w.machines.get(&target).map(|t| t.team);
                let ok = w.machines.get(&target).is_some_and(|t| {
                    t.model == Model::Printer && t.team != team && here.is_adjacent(t.pos)
                }) && !w.converted_this_tick.contains(&target);
                if ok {
                    w.converted_this_tick.insert(target);
                    if let Some(t) = w.machines.get_mut(&target) {
                        t.team = team;
                        t.deployment = Some("printer".to_string());
                        if let Some(p) = &mut t.program {
                            p.raise(Interrupt::Redeploy);
                        }
                    }
                    self.apply_pending_deploys(team);
                    if let Some(old) = old_team {
                        self.apply_pending_deploys(old);
                    }
                }
            }
            Effect::Wait => {}
        }
        Value::None
    }

    /// A full site's construction ran its course: it becomes its building.
    fn finish_site(&mut self, id: EntityId) {
        let w = &mut self.world;
        let Some(site) = w.machines.get(&id) else {
            return;
        };
        let (team, pos, model) = (site.team, site.pos, site.becomes.unwrap_or(Model::Depot));
        w.machines.remove(&id);
        let new_id = w.place(model, team, pos, Some(model.name().to_string()));
        // Keeps the site's entity? No: the building is a machine of its own,
        // named after its own id, as `place` names it.
        let _ = new_id;
        w.sound("build", pos);
    }

    fn remove_machine(&mut self, id: EntityId) {
        self.world.machines.remove(&id);
    }

    /// Step 4: attacks and printer defence, summed per target; `dying` and
    /// `death` raised; `death` epilogues remove machines.
    fn damage(&mut self) {
        let w = &mut self.world;
        let mut hits: BTreeMap<EntityId, Num> = std::mem::take(&mut w.pending_damage);
        // Every printer defends against adjacent bots of another team.
        let printers: Vec<(TeamId, TilePos, Num)> = w
            .machines
            .values()
            .filter(|m| m.model == Model::Printer)
            .map(|m| (m.team, m.pos, w.data.model("printer").defence_damage))
            .collect();
        for (team, pos, dmg) in printers {
            if dmg <= Num::ZERO {
                continue;
            }
            let victims: Vec<EntityId> = w
                .machines
                .values()
                .filter(|b| b.model == Model::Bot && b.team != team && pos.is_adjacent(b.pos))
                .map(|b| b.id)
                .collect();
            for v in victims {
                let d = hits.entry(v).or_insert(Num::ZERO);
                *d = d.checked_add(dmg).unwrap_or(Num::MAX);
            }
        }
        let mut to_deliver: Vec<(EntityId, Interrupt)> = Vec::new();
        for (id, dmg) in hits {
            let Some(m) = w.machines.get_mut(&id) else {
                continue;
            };
            if m.dying {
                continue;
            }
            m.health = m.health.checked_sub(dmg).unwrap_or(Num::MIN);
            let threshold = w.data.model(m.model.name()).death_threshold;
            if m.health <= threshold {
                to_deliver.push((id, Interrupt::Death));
            } else if m.health <= Num::ZERO {
                to_deliver.push((id, Interrupt::Dying));
            }
        }
        for (id, kind) in to_deliver {
            match kind {
                Interrupt::Death => {
                    // Delivered now: the epilogue removes the machine here.
                    if let Some(p) = w.machines.get_mut(&id).and_then(|m| m.program.as_mut()) {
                        p.raise(Interrupt::Death);
                    }
                    w.dead.insert(id);
                }
                _ => {
                    if let Some(m) = w.machines.get_mut(&id) {
                        match m.program.as_mut() {
                            Some(p) => p.raise(Interrupt::Dying),
                            // A site has no program: dying is death at once.
                            None => {
                                w.dead.insert(id);
                            }
                        }
                    }
                }
            }
        }
        let dead: Vec<EntityId> = w.dead.iter().copied().collect();
        for id in dead {
            w.machines.remove(&id);
        }
        w.dead.clear();
    }

    /// Step 5: every team that resigned or has no printer is out.
    fn out_teams(&mut self) {
        let w = &mut self.world;
        let teams: Vec<TeamId> = w.teams.keys().copied().collect();
        for team in teams {
            let t = &w.teams[&team];
            if t.out {
                continue;
            }
            if !t.resigned && w.printers_of(team) > 0 {
                continue;
            }
            w.machines.retain(|_, m| m.team != team);
            for tile in &mut w.tiles {
                tile.plans.remove(&team);
            }
            w.teams.get_mut(&team).expect("team").out = true;
        }
    }

    /// Step 6: every deposit grows to its cap.
    fn regrow(&mut self) {
        for tile in &mut self.world.tiles {
            if let Some(d) = tile.deposit.as_mut() {
                for (kind, amount) in d.amount.iter_mut() {
                    let growth = d.regrowth.get(kind).copied().unwrap_or(Num::ZERO);
                    let cap = d.cap.get(kind).copied().unwrap_or(Num::ZERO);
                    *amount = amount.checked_add(growth).unwrap_or(cap).min(cap);
                }
            }
        }
    }

    /// Step 7: each team's visible tiles, and their snapshots (Q21).
    fn vision_pass(&mut self) {
        let teams: Vec<TeamId> = self.world.teams.keys().copied().collect();
        let tick = self.world.tick;
        for team in teams {
            if self.world.teams[&team].out {
                continue;
            }
            let visible = sense::team_visible_tiles(&self.world, team);
            let positions = self.world.map.positions();
            for p in positions {
                let idx = self.world.tile_index(p).expect("in bounds");
                if visible.contains(&p) {
                    let tile = &self.world.tiles[idx];
                    let building = self
                        .world
                        .machines
                        .values()
                        .find(|m| m.pos == p && m.is_building())
                        .map(MachineRecord::of);
                    let mem = TileMemory {
                        state: TileState::Visible,
                        seen_at: tick,
                        terrain: tile.terrain,
                        deposit: tile.deposit.as_ref().map(|d| d.amount.clone()),
                        paint: tile.paint.clone(),
                        overlay: tile.overlay.clone(),
                        building,
                    };
                    self.world.teams.get_mut(&team).expect("team").memory[idx] = mem;
                } else {
                    let mem = &mut self.world.teams.get_mut(&team).expect("team").memory[idx];
                    if mem.state == TileState::Visible {
                        mem.state = TileState::Remembered;
                    }
                }
            }
        }
    }

    /// Step 8: the state hash (`docs/06`, The state hash), in its order.
    pub fn state_hash(&self) -> u64 {
        let w = &self.world;
        let mut h = Fnv1a::new();
        h.write_u64(w.tick);
        for (tid, team) in &w.teams {
            h.write_u32(tid.0);
            h.write_u8(u8::from(team.out));
            for (name, d) in &team.deployments {
                h.write_str(name);
                h.write_u64(d.bundle.as_ref().map(|b| b.version).unwrap_or(0));
                h.write_u64(d.pending.as_ref().map(|b| b.version).unwrap_or(0));
            }
            h.write_u64(w.bot_cap(*tid) as u64);
            let mut planned: Vec<(TilePos, &crate::world::Plans)> = Vec::new();
            for p in w.map.positions() {
                if let Some(pl) = w.plans(*tid, p) {
                    planned.push((p, pl));
                }
            }
            planned.sort_by(|a, b| row_then_column(&a.0, &b.0));
            h.write_u64(planned.len() as u64);
            for (p, pl) in planned {
                write_pos(&mut h, p);
                for plan in [&pl.paint, &pl.overlay, &pl.building] {
                    match plan {
                        None => h.write_u8(0),
                        Some(Plan::Clear) => h.write_u8(1),
                        Some(Plan::Set(v)) => {
                            h.write_u8(2);
                            h.write_str(v);
                        }
                    }
                }
            }
            for p in w.map.positions() {
                let idx = w.tile_index(p).expect("in bounds");
                let m = &team.memory[idx];
                h.write_u8(match m.state {
                    TileState::Unknown => 0,
                    TileState::Visible => 1,
                    TileState::Remembered => 2,
                });
                if m.state != TileState::Unknown {
                    h.write_u64(m.seen_at);
                    h.write_str(m.terrain.name());
                    write_amounts(&mut h, m.deposit.as_ref());
                    write_opt_str(&mut h, m.paint.as_deref());
                    write_opt_str(&mut h, m.overlay.as_deref());
                    match &m.building {
                        None => h.write_u8(0),
                        Some(b) => {
                            h.write_u8(1);
                            write_record(&mut h, b);
                        }
                    }
                }
            }
        }
        for p in w.map.positions() {
            let t = w.tile(p).expect("in bounds");
            h.write_str(t.terrain.name());
            write_amounts(&mut h, t.deposit.as_ref().map(|d| &d.amount));
            write_opt_str(&mut h, t.paint.as_deref());
            write_opt_str(&mut h, t.overlay.as_deref());
        }
        h.write_u32(w.next_id_value());
        h.write_u64(w.machines.len() as u64);
        for m in w.machines.values() {
            write_record(&mut h, &MachineRecord::of(m));
            h.write_u8(u8::from(m.dying));
            match &m.action {
                None => h.write_u8(0),
                Some(a) => {
                    h.write_u8(1);
                    h.write_str(a.name);
                    h.write_u64(a.remaining);
                    h.write_str(&format!("{:?}", a.effect));
                }
            }
            h.write_u64(m.construction.unwrap_or(0));
            write_opt_str(&mut h, m.becomes.map(Model::name));
            match &m.program {
                None => h.write_u8(0),
                Some(p) => {
                    h.write_u8(1);
                    match &p.fault_record {
                        None => h.write_u8(0),
                        Some(r) => {
                            h.write_u8(1);
                            write_opt_str(
                                &mut h,
                                r.exception.as_ref().map(|e| e.class_name()).as_deref(),
                            );
                            write_opt_str(&mut h, r.exhausted.map(Interrupt::name));
                            write_opt_str(&mut h, r.file.as_deref());
                            h.write_u32(r.line);
                            h.write_u64(r.tick);
                            h.write_u64(r.version);
                        }
                    }
                    h.write_bytes(&p.state_bytes());
                }
            }
        }
        h.finish()
    }
}

fn write_pos(h: &mut Fnv1a, p: TilePos) {
    h.write_i32(p.x);
    h.write_i32(p.y);
}

fn write_opt_str(h: &mut Fnv1a, s: Option<&str>) {
    match s {
        None => h.write_u8(0),
        Some(s) => {
            h.write_u8(1);
            h.write_str(s);
        }
    }
}

fn write_amounts(h: &mut Fnv1a, m: Option<&BTreeMap<String, Num>>) {
    match m {
        None => h.write_u8(0),
        Some(m) => {
            h.write_u8(1);
            h.write_u64(m.len() as u64);
            for (k, v) in m {
                h.write_str(k);
                h.write_bytes(&v.raw().to_le_bytes());
            }
        }
    }
}

fn write_record(h: &mut Fnv1a, r: &MachineRecord) {
    h.write_u32(r.id.0);
    h.write_str(&r.name);
    h.write_str(r.model.name());
    h.write_u32(r.team.0);
    write_opt_str(h, r.deployment.as_deref());
    write_pos(h, r.pos);
    h.write_bytes(&r.health.raw().to_le_bytes());
    write_opt_str(h, r.busy);
    write_amounts(h, r.load.as_ref());
    write_amounts(h, r.store.as_ref());
    h.write_u64(r.progress);
}

/// Load a bundle from the log's files (`docs/01`, The bundle).
pub fn load_bundle(bundle: &Bundle, limits: &Limits) -> Result<LoadedBundle, String> {
    let refs: Vec<(&str, &str)> = bundle
        .files
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    let program = Program::load(&refs, limits).map_err(|e| e.to_string())?;
    Ok(LoadedBundle {
        files: bundle.clone(),
        version: program.version,
        program,
    })
}

/// A bundle from one `main.py`, for tests and scripts.
pub fn bundle_of(main: &str) -> Bundle {
    Bundle {
        files: vec![("main.py".to_string(), main.to_string())],
    }
}

impl Sim {
    /// Every machine id, for tests.
    pub fn ids(&self) -> BTreeSet<EntityId> {
        self.world.machines.keys().copied().collect()
    }
}

/// Unused-import guard for `Slice`, which the run loop discards on purpose.
#[allow(dead_code)]
fn _slice(_: Slice) {}
