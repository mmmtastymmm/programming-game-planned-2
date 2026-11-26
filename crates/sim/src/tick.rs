//! The tick (`docs/06`, The tick): nine steps in a fixed order, and
//! `Sim::step` is the only entry point that changes the world.

use crate::command::{Bundle, Command, CommandKind, PlanKind, TeamId};
use crate::data::Data;
use crate::fork::{Hit, Saved, Watch, WatchIndex, Watcher};
use crate::hash::Fnv1a;
use crate::host::GameHost;
use crate::map::{Map, TilePos};
use crate::sense;
use crate::world::{
    Effect, EntityId, LoadedBundle, MachineRecord, Model, PickSource, Plan, Roster, Slot,
    TileState, WIRE, World,
};
use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, Event, Interrupt, Limits, Num, Program, Slice, Tracer, Value};
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
    /// For each machine that began a statement on a watched line this
    /// tick, the first such beginning and a count of the rest, in position
    /// order (`docs/06`, Forks, Watches). Read off the run, never part of
    /// it: no hash covers this.
    pub hits: Vec<Hit>,
}

pub struct Sim {
    pub world: World,
    pub(crate) ended: Option<Option<TeamId>>,
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

    /// A match on `map` with tables of the caller's: the shipped language
    /// tables and `data` in place of the shipped machines, world and net.
    /// For tests of rules the shipped numbers leave idle — a building
    /// that drains more than it makes, which no shipped model does.
    pub fn with_data(map: Map, data: Data) -> Sim {
        let costs = Rc::new(Costs::parse(COSTS_TOML).expect("costs"));
        let limits = Rc::new(Limits::parse(LIMITS_TOML).expect("limits"));
        Sim {
            world: World::new(map, Rc::new(data), costs, limits),
            ended: None,
        }
    }

    pub fn ended(&self) -> Option<Option<TeamId>> {
        self.ended
    }

    /// The match's full state, execution state included, as it stands at
    /// the end of the tick just completed (`docs/06`, Forks): a copy that
    /// shares nothing the sim will change. It is a read; a fork is opened
    /// from it, and nothing restores the match's sim to one.
    pub fn save(&self) -> Saved {
        Saved {
            world: self.world.copy(),
            ended: self.ended,
            entries: 0,
        }
    }

    /// A sim at tick 0 on this one's map and tables: where a fork's
    /// re-run from the map starts.
    pub(crate) fn fresh(&self) -> Sim {
        let w = &self.world;
        Sim {
            world: World::new(
                w.map.clone(),
                w.data.clone(),
                w.costs.clone(),
                w.limits.clone(),
            ),
            ended: None,
        }
    }

    /// The values a `Mark` of `plan` may name, in the data's order: the
    /// building models a plan may name, the paint colors, or the overlay
    /// labels (Q10, Q42). [`Sim::validate`] accepts every one of them, and
    /// the renderer's mark strip lists exactly these (Q37).
    pub fn markable(data: &Data, plan: PlanKind) -> Vec<String> {
        match plan {
            PlanKind::Building => Model::ALL
                .into_iter()
                .filter(|m| m.buildable())
                .map(|m| m.name().to_string())
                .collect(),
            PlanKind::Paint => data.machines.colors.clone(),
            PlanKind::Overlay => data.world.overlays.clone(),
            // A wire has no kinds: the one value is the plan to lay one.
            PlanKind::Wire => vec![WIRE.to_string()],
        }
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
                if !crate::world::is_deployment_name(deployment) {
                    return Err(format!(
                        "no deployment `{deployment}`: a team's are {} (Q42)",
                        crate::script::deployments_prose()
                    ));
                }
                load_bundle(bundle, &w.limits).map(|_| ())
            }
            CommandKind::Mark { at, plan, value } => {
                if !w.map.in_bounds(*at) {
                    return Err(format!("({}, {}) is off the map", at.x, at.y));
                }
                match (plan, value) {
                    (PlanKind::Building, Some(v))
                        if Model::from_name(v).is_none_or(|m| !m.buildable()) =>
                    {
                        Err(format!("`{v}` is not a building model"))
                    }
                    (PlanKind::Wire, Some(v)) if v != WIRE => {
                        Err(format!("a wire plan's value is `{WIRE}`, not `{v}`"))
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

    /// Run one tick with its full command set, in the doc's nine steps.
    /// `watches` are a team's watched lines (`docs/06`, Forks, Watches):
    /// they are no commands, charge nothing and change nothing, and the
    /// report's `hits` is all they produce.
    pub fn step(&mut self, commands: &[Command], watches: &[Watch]) -> StepReport {
        let mut report = StepReport::default();
        if self.ended.is_some() {
            return report;
        }
        self.open_tick(commands, &[], &mut report);
        // 2. Every machine's slice, ascending id.
        let index = WatchIndex::new(watches);
        for id in self.slice_ids() {
            match self.who(id).filter(|who| index.fits(who)) {
                Some(who) => {
                    let mut watcher = Watcher::new(&index, who, self.world.tick);
                    self.run_slice_with(id, Some(&mut watcher), false);
                    report.hits.extend(watcher.hit());
                }
                None => {
                    self.run_slice_with(id, None, false);
                }
            }
        }
        // 3. Completions, ascending id.
        self.completions();
        // 4. Damage and death.
        self.damage_apply();
        let dead: Vec<EntityId> = self.world.dead.iter().copied().collect();
        for id in dead {
            self.world.remove(id);
        }
        self.world.dead.clear();
        self.close_tick(&mut report);
        report
    }

    /// The tick's opening and its step 1: the commands by tick, sender
    /// and submission order, then `extra` in the order given — a fork's
    /// what-if deploys, which the match's sim never has (`docs/06`,
    /// Forks, What-if).
    pub(crate) fn open_tick(
        &mut self,
        commands: &[Command],
        extra: &[Command],
        report: &mut StepReport,
    ) {
        let w = &mut self.world;
        w.tick = w.tick.checked_add(1).expect("tick counter");
        w.pending_damage.clear();
        w.dead.clear();
        for m in w.machines.values_mut() {
            m.log.clear();
        }
        // 1. Commands, by sender then submission order.
        let mut ordered: Vec<&Command> = commands.iter().collect();
        // Tick 0's opening set, applied with tick 1, before tick 1's own
        // (`docs/06`, The command log); then by sender and submission.
        ordered.sort_by(|a, b| {
            a.tick
                .cmp(&b.tick)
                .then(a.sender.cmp(&b.sender))
                .then(a.seq.cmp(&b.seq))
        });
        for c in ordered.into_iter().chain(extra) {
            if let Err(()) = self.apply(c, report) {
                report.dropped.push(c.clone());
            }
        }
    }

    /// The machines step 2 runs, in its order.
    pub(crate) fn slice_ids(&self) -> Vec<EntityId> {
        self.world.machines.keys().copied().collect()
    }

    /// Step 3: completions, ascending id.
    pub(crate) fn completions(&mut self) {
        let ids: Vec<EntityId> = self.world.machines.keys().copied().collect();
        for id in ids {
            self.complete(id);
        }
    }

    /// Steps 5 to 8, and whether the match ended.
    pub(crate) fn close_tick(&mut self, report: &mut StepReport) {
        // 5. Out teams.
        self.out_teams();
        // 6. Power and printing, team by team (`docs/08`, The power step):
        // grids traced and served, solar, waking, then prints.
        // One pass over the machines serves every team's turn (T41): no
        // team's power or prints add or remove a machine, or change
        // another team's prints in progress.
        let teams: Vec<TeamId> = self.world.teams.keys().copied().collect();
        let rosters = self.world.rosters();
        let none = Roster::default();
        for team in teams {
            if self.world.teams[&team].out {
                continue;
            }
            let roster = rosters.get(&team).unwrap_or(&none);
            self.world.power_step(team, &roster.machines);
            self.print_step(team, roster);
        }
        // 7. Regrowth.
        self.regrow();
        // 8. The vision pass.
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
                if !crate::world::is_deployment_name(deployment) {
                    return Err(());
                }
                self.deploy(c.sender, deployment, loaded);
                Ok(())
            }
            CommandKind::Mark { at, plan, value } => {
                if !w.map.in_bounds(*at) {
                    return Err(());
                }
                // Paint and overlay plans only on ground the team has
                // seen (Q49): a team's knowledge changes over the match,
                // so this is checked on the tick the command applies, not
                // at submission, and a mark that fails it is dropped.
                w.may_plan(c.sender, *at, *plan).map_err(|_| ())?;
                let p = match value {
                    Some(v) => Plan::Set(v.clone()),
                    None => Plan::Clear,
                };
                // A wire plan asking for what the tile already has — to lay
                // where the team's wire lies, to clear where none does — is
                // dropped, so every wire plan is one a bot can carry out
                // (`docs/03`).
                if *plan == PlanKind::Wire && w.has_wire(c.sender, *at) == matches!(p, Plan::Set(_))
                {
                    return Err(());
                }
                let plans = w.plans_mut(c.sender, *at).ok_or(())?;
                *plans.of_mut(*plan) = Some(p);
                Ok(())
            }
            CommandKind::Unmark { at, plan } => {
                let plans = w.plans_mut(c.sender, *at).ok_or(())?;
                *plans.of_mut(*plan) = None;
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

    /// Step 2 for one machine: take its program out of its body, run it
    /// against a host over the world, put it back, act on its events.
    ///
    /// A `tracer` is shown every boundary the slice passes, and one that
    /// stops it leaves the slice open: the program goes back in its body
    /// as it stands, this returns `true`, and a call with `resume` goes on
    /// from there. Only a fork passes a tracer that stops (`docs/06`,
    /// Forks); the match's watches never do.
    pub(crate) fn run_slice_with(
        &mut self,
        id: EntityId,
        tracer: Option<&mut dyn Tracer>,
        resume: bool,
    ) -> bool {
        let Some(mut program) = self
            .world
            .machines
            .get_mut(&id)
            .and_then(|m| m.program.take())
        else {
            return false;
        };
        if program.is_dead() {
            self.world.machines.get_mut(&id).expect("machine").program = Some(program);
            return false;
        }
        // A dark building's main flow does not run, though a handler
        // does (`docs/08`, A dark building).
        let dark = self.world.machines.get(&id).is_some_and(|m| m.is_dark());
        let slice = {
            let mut host = GameHost {
                world: &mut self.world,
                id,
            };
            if resume {
                program.resume_slice(&mut host, tracer)
            } else {
                program.tick = host.world.tick;
                if dark {
                    program.run_slice_held(&mut host, tracer)
                } else {
                    program.run_slice_traced(&mut host, tracer)
                }
            }
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
        slice == Slice::Stopped
    }

    /// What a watch is matched against: the machine, if it runs a program.
    pub(crate) fn who(&self, id: EntityId) -> Option<crate::fork::Who> {
        let m = self.world.machines.get(&id)?;
        m.program.as_ref()?;
        Some(crate::fork::Who {
            id,
            team: m.team,
            deployment: m.deployment.clone(),
        })
    }

    /// Step 3 for one machine: count a printer's print down and complete
    /// it, count its action down and complete it, and a site's
    /// construction.
    fn complete(&mut self, id: EntityId) {
        self.complete_print(id);
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
        // A dark building's main-flow action is held (`docs/08`). A hook's
        // is not: a prologue cancels main flow's action, so one in
        // progress while a handler runs is the hook's own.
        if m.is_dark() && m.program.as_ref().is_some_and(|p| p.mode_name() == "main") {
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
                w.move_machine(id, to);
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
            Effect::Build { model, at, remove } => {
                if let Some(old) = remove {
                    self.remove_machine(old);
                }
                let w = &mut self.world;
                if !w.buildable(at) || w.occupied(at) {
                    return Value::None;
                }
                let team = w.machines[&id].team;
                // The price is counted before the new site exists (Q42).
                let price = w.price(team, model);
                let site = w.place(Model::Site, team, at, None);
                if let Some(s) = w.machines.get_mut(&site) {
                    s.becomes = Some(model);
                    s.name = format!("{}{}", model.name(), site.0);
                    s.store_capacity = price;
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
            Effect::Recharge { from } => {
                // From what the building then holds; nothing if it is
                // gone or dark (`docs/02`, `recharge`).
                let amount = {
                    let host = GameHost { world: w, id };
                    host.recharge_amount(from)
                };
                let w = &mut self.world;
                if amount > Num::ZERO {
                    if let Some(b) = w.machines.get_mut(&from) {
                        b.charge = b.charge.checked_sub(amount).unwrap_or(Num::ZERO);
                    }
                    if let Some(m) = w.machines.get_mut(&id) {
                        m.charge = m.charge.checked_add(amount).unwrap_or(Num::MAX);
                    }
                    w.charge_changed(id);
                }
            }
            Effect::Wire { at } => {
                // Laid only if still absent: another of the team's bots
                // may have laid it first this tick, and then nothing is
                // taken (`docs/02`, `wire`).
                let team = w.machines[&id].team;
                let cost = w.data.model("bot").wire_cost.clone();
                if w.lay_wire(team, at) {
                    if let Some(m) = w.machines.get_mut(&id) {
                        for (kind, c) in &cost {
                            let l = m.load.entry(kind.clone()).or_insert(Num::ZERO);
                            *l = l.checked_sub(*c).unwrap_or(Num::ZERO).max(Num::ZERO);
                        }
                    }
                    if let Some(pl) = w.plans_mut(team, at)
                        && matches!(pl.wire, Some(Plan::Set(_)))
                    {
                        pl.wire = None;
                    }
                    w.tidy_plans(at);
                }
            }
            Effect::Unwire { at } => {
                let team = w.machines[&id].team;
                if w.remove_wire(team, at) {
                    if let Some(pl) = w.plans_mut(team, at)
                        && matches!(pl.wire, Some(Plan::Clear))
                    {
                        pl.wire = None;
                    }
                    w.tidy_plans(at);
                }
            }
            Effect::Wait => {}
        }
        Value::None
    }

    /// A full site's construction ran its course: it becomes its building.
    pub(crate) fn finish_site(&mut self, id: EntityId) {
        let w = &mut self.world;
        let Some(site) = w.machines.get(&id) else {
            return;
        };
        let (team, pos, model) = (site.team, site.pos, site.becomes.unwrap_or(Model::Depot));
        w.remove(id);
        // A pylon runs no program and has no deployment (Q62).
        let new_id = w.place(model, team, pos, model.deployment().map(str::to_string));
        // Keeps the site's entity? No: the building is a machine of its own,
        // named after its own id, as `place` names it.
        let _ = new_id;
        w.sound("build", pos);
    }

    fn remove_machine(&mut self, id: EntityId) {
        self.world.remove(id);
    }

    /// Step 3, a printer's part: its print counts down, and at zero the
    /// bot appears on the first adjacent tile of `n`, `e`, `s`, `w` that is
    /// on the map, passable and unoccupied, or the print is cancelled
    /// (`docs/02`, Printing).
    fn complete_print(&mut self, id: EntityId) {
        let w = &mut self.world;
        let dead = w.dead.contains(&id);
        let Some(m) = w.machines.get_mut(&id) else {
            return;
        };
        if m.model != Model::Printer || m.printing == 0 {
            return;
        }
        // A printer whose `death` ran in its slice has lost its print
        // (docs/02, Printing); one at or below zero lost it in `set_health`.
        if dead {
            m.printing = 0;
            return;
        }
        // A dark printer's print holds (`docs/08`).
        if !m.powered {
            return;
        }
        m.printing = m.printing.saturating_sub(1);
        if m.printing > 0 {
            return;
        }
        let (team, pos) = (m.team, m.pos);
        let free = pos
            .neighbours()
            .into_iter()
            .find(|p| w.steps(pos, *p) && !w.occupied(*p));
        if let Some(p) = free {
            w.place(Model::Bot, team, p, Some("bot".to_string()));
        }
    }

    /// Step 6, the prints: the team's printers begin prints by turns while
    /// its bots and prints in progress fall short of its cap (Q42;
    /// `docs/02`, Printing). A dark printer begins none (`docs/08`).
    /// `roster` is the team's machines as the step found them
    /// ([`World::rosters`]); which of its printers are ready is read now,
    /// after its power step.
    fn print_step(&mut self, team: TeamId, roster: &Roster) {
        let w = &mut self.world;
        let ticks = w.data.model("printer").print_ticks;
        let shortfall = w
            .cap_of(roster.printers)
            .saturating_sub(roster.bots)
            .saturating_sub(roster.printing);
        // The ready printers in id order, rotated to start past the
        // print turn: each begins at most once this step, in that order.
        let ready: Vec<EntityId> = roster
            .machines
            .iter()
            .filter_map(|id| w.machines.get(id))
            .filter(|m| {
                m.team == team
                    && m.model == Model::Printer
                    && m.printing == 0
                    && m.powered
                    && m.health > Num::ZERO
            })
            .map(|m| m.id)
            .collect();
        let turn = w.teams[&team].print_turn;
        let split = ready.partition_point(|id| *id <= turn);
        let order = ready[split..].iter().chain(&ready[..split]);
        for &next in order.take(shortfall) {
            let pos = w.machines[&next].pos;
            w.machines.get_mut(&next).expect("printer").printing = ticks;
            w.sound("print", pos);
            w.teams.get_mut(&team).expect("team").print_turn = next;
        }
    }

    /// Step 4 up to the removals: attacks and printer defence, summed per
    /// target; `dying` and `death` raised. It leaves in `world.dead` every
    /// machine the step removes, in the order it removes them, and returns
    /// the ones this step's damage killed — the rest died in their own
    /// slices. The caller removes them; a fork stops between this and a
    /// removal, at a damage position (`docs/06`, Forks, Positions).
    pub(crate) fn damage_apply(&mut self) -> BTreeSet<EntityId> {
        let w = &mut self.world;
        let mut hits: BTreeMap<EntityId, Num> = std::mem::take(&mut w.pending_damage);
        // Every powered printer defends against adjacent bots of another
        // team; a dark one deals none (`docs/08`).
        let printers: Vec<(TeamId, TilePos, Num)> = w
            .machines
            .values()
            .filter(|m| m.model == Model::Printer && m.powered)
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
        let mut killed = BTreeSet::new();
        for (id, dmg) in hits {
            let Some(m) = w.machines.get(&id) else {
                continue;
            };
            if m.dying {
                continue;
            }
            let health = m.health.checked_sub(dmg).unwrap_or(Num::MIN);
            let threshold = w.data.model(m.model.name()).death_threshold;
            w.set_health(id, health);
            if health <= threshold {
                to_deliver.push((id, Interrupt::Death));
            } else if health <= Num::ZERO {
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
                    if w.dead.insert(id) {
                        killed.insert(id);
                    }
                }
                _ => {
                    if let Some(m) = w.machines.get_mut(&id) {
                        match m.program.as_mut() {
                            Some(p) => p.raise(Interrupt::Dying),
                            // A site has no program: dying is death at once.
                            None => {
                                if w.dead.insert(id) {
                                    killed.insert(id);
                                }
                            }
                        }
                    }
                }
            }
        }
        killed
    }

    /// Step 5: every team that resigned or has no printer is out.
    pub(crate) fn out_teams(&mut self) {
        let w = &mut self.world;
        let teams: Vec<TeamId> = w.teams.keys().copied().collect();
        // Counted once for every team (T41): a team going out removes its
        // own machines and no other's.
        let rosters = w.rosters();
        for team in teams {
            let t = &w.teams[&team];
            if t.out {
                continue;
            }
            if !t.resigned && rosters.get(&team).is_some_and(|r| r.printers > 0) {
                continue;
            }
            w.remove_team(team);
            for tile in &mut w.tiles {
                tile.plans.remove(&team);
            }
            // Its wires go with its machines, and it has no grids (Q62).
            w.clear_wires(team);
            let t = w.teams.get_mut(&team).expect("team");
            t.grids.clear();
            t.out = true;
        }
    }

    /// Step 7: every deposit grows to its cap.
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

    /// Step 8: each team's visible tiles, and their snapshots (Q21).
    ///
    /// Only a tile the team sees now, or saw at the last pass, can
    /// change, so those are the tiles visited: what it saw is demoted,
    /// what it sees is written. Every other tile is left as it stands,
    /// which is what a walk of the whole map wrote to it, at a cost of
    /// teams by tiles a tick whatever the teams saw (I9).
    fn vision_pass(&mut self) {
        let teams: Vec<TeamId> = self.world.teams.keys().copied().collect();
        let tick = self.world.tick;
        // The building on each tile, the least id first where two stand
        // on one (none can), looked up per tile rather than searched for:
        // the search was every machine for every visible tile (I8).
        let mut buildings: BTreeMap<TilePos, MachineRecord> = BTreeMap::new();
        for m in self.world.machines.values().filter(|m| m.is_building()) {
            buildings
                .entry(m.pos)
                .or_insert_with(|| MachineRecord::of(m));
        }
        for team in teams {
            if self.world.teams[&team].out {
                continue;
            }
            sense::ensure_sight(&mut self.world, team);
            let w = &mut self.world;
            let (width, height) = (w.map.width, w.map.height);
            let visible = &w.sight.teams[&team];
            let t = w.teams.get_mut(&team).expect("team");
            // What it saw at the last pass it now remembers; a tile it
            // sees still is written over below.
            for idx in t.seen.drain(..) {
                let mem = &mut t.memory[idx];
                if mem.state == TileState::Visible {
                    mem.state = TileState::Remembered;
                }
            }
            // A tile it sees is remembered as it stands. Only what differs
            // is replaced: on most ticks that is `seen_at` alone, and a
            // copy of every deposit, paint, overlay, wire set and building
            // record for every tile in sight was most of the pass (I12).
            for &p in visible {
                let idx = crate::map::index(p, width, height).expect("in bounds");
                let tile = &w.tiles[idx];
                let mem = &mut t.memory[idx];
                mem.state = TileState::Visible;
                mem.seen_at = tick;
                mem.terrain = tile.terrain;
                mem.height = tile.height;
                let amount = tile.deposit.as_ref().map(|d| &d.amount);
                if mem.deposit.as_ref() != amount {
                    mem.deposit = amount.cloned();
                }
                if mem.paint != tile.paint {
                    mem.paint.clone_from(&tile.paint);
                }
                if mem.overlay != tile.overlay {
                    mem.overlay.clone_from(&tile.overlay);
                }
                if mem.wires != tile.wires {
                    mem.wires.clone_from(&tile.wires);
                }
                let building = buildings.get(&p);
                if mem.building.as_ref() != building {
                    mem.building = building.cloned();
                }
                t.known[idx] = true;
                t.seen.push(idx);
            }
        }
    }

    /// Step 9: the state hash (`docs/06`, The state hash), in its order.
    pub fn state_hash(&self) -> u64 {
        let w = &self.world;
        let mut h = Fnv1a::new();
        h.write_u64(w.tick);
        // One pass over the machines counts every team's printers, and
        // one over the tiles, in the order the hash writes them, gives
        // each team its plans already in that order (T41): the bytes are
        // those a walk of the map and of the machines for each team wrote.
        let rosters = w.rosters();
        let mut planned: BTreeMap<TeamId, Vec<(TilePos, &crate::world::Plans)>> = BTreeMap::new();
        for (p, idx) in w.map.walk() {
            for (team, pl) in &w.tiles[idx].plans {
                planned.entry(*team).or_default().push((p, pl));
            }
        }
        for (tid, team) in &w.teams {
            h.write_u32(tid.0);
            h.write_u8(u8::from(team.out));
            for (name, d) in &team.deployments {
                h.write_str(name);
                h.write_u64(d.bundle.as_ref().map(|b| b.version).unwrap_or(0));
            }
            let printers = rosters.get(tid).map_or(0, |r| r.printers);
            h.write_u64(w.cap_of(printers) as u64);
            h.write_u32(team.print_turn.0);
            // Its grids' figures, in label order (Q62).
            h.write_u64(team.grids.len() as u64);
            for g in &team.grids {
                h.write_u32(g.grid.0);
                for v in [g.generation, g.drain, g.stored, g.capacity] {
                    h.write_bytes(&v.raw().to_le_bytes());
                }
            }
            let planned = planned.get(tid).map_or(&[][..], Vec::as_slice);
            h.write_u64(planned.len() as u64);
            for (p, pl) in planned {
                write_pos(&mut h, *p);
                for plan in [&pl.paint, &pl.overlay, &pl.building, &pl.wire] {
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
            // A tile the team has never seen is one zero byte, and a run
            // of them one step of the hash (`Fnv1a::write_zeros`): told
            // from `known`, so that its memory, a wide record, is not
            // read to learn it (T42). The bytes are those of a walk that
            // read every memory.
            let mut unknown = 0u64;
            for (_, idx) in w.map.walk() {
                debug_assert_eq!(
                    team.known[idx],
                    team.memory[idx].state != TileState::Unknown,
                    "a team's index of the tiles it knows fell out of step with its memory"
                );
                if !team.known[idx] {
                    unknown = unknown.saturating_add(1);
                    continue;
                }
                h.write_zeros(unknown);
                unknown = 0;
                let m = &team.memory[idx];
                h.write_u8(match m.state {
                    TileState::Unknown => 0,
                    TileState::Visible => 1,
                    TileState::Remembered => 2,
                });
                if m.state != TileState::Unknown {
                    h.write_u64(m.seen_at);
                    h.write_str(m.terrain.name());
                    h.write_u8(m.height.level());
                    write_amounts(&mut h, m.deposit.as_ref());
                    write_opt_str(&mut h, m.paint.as_deref());
                    write_opt_str(&mut h, m.overlay.as_deref());
                    write_teams(&mut h, &m.wires);
                    match &m.building {
                        None => h.write_u8(0),
                        Some(b) => {
                            h.write_u8(1);
                            write_record(&mut h, b);
                        }
                    }
                }
            }
            // The run the table ends in.
            h.write_zeros(unknown);
        }
        for (_, idx) in w.map.walk() {
            let t = &w.tiles[idx];
            h.write_str(t.terrain.name());
            h.write_u8(t.height.level());
            write_amounts(&mut h, t.deposit.as_ref().map(|d| &d.amount));
            write_opt_str(&mut h, t.paint.as_deref());
            write_opt_str(&mut h, t.overlay.as_deref());
            write_teams(&mut h, &t.wires);
        }
        h.write_u32(w.next_id_value());
        h.write_u64(w.machines.len() as u64);
        for m in w.machines.values() {
            write_record(&mut h, &MachineRecord::of(m));
            h.write_u8(u8::from(m.dying));
            // Whether a bot is armed for `low_power` (Q62).
            h.write_u8(u8::from(m.armed));
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
            // A site's capacity is its price, fixed at placement from the
            // team's count then (Q42): history, so hashed.
            write_amounts(&mut h, Some(&m.store_capacity));
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
        // This tick's sounds, which `hear()` reads next tick: stored, so
        // hashed (`docs/06`, The state hash).
        h.write_u64(w.sounds_prev.len() as u64);
        for snd in &w.sounds_prev {
            h.write_str(&snd.cause);
            write_pos(&mut h, snd.pos);
            h.write_i64(snd.loudness);
            h.write_u64(snd.tick);
        }
        h.finish()
    }
}

fn write_pos(h: &mut Fnv1a, p: TilePos) {
    h.write_i32(p.x);
    h.write_i32(p.y);
}

/// The teams with a wire on a tile, in order (Q62).
fn write_teams(h: &mut Fnv1a, teams: &BTreeSet<TeamId>) {
    h.write_u64(teams.len() as u64);
    for t in teams {
        h.write_u32(t.0);
    }
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
    match &r.color {
        None => h.write_u8(0),
        Some(c) => {
            h.write_u8(1);
            write_opt_str(h, c.as_deref());
        }
    }
    write_amounts(h, r.store.as_ref());
    h.write_u64(r.progress);
    match r.printing {
        None => h.write_u8(0),
        Some(t) => {
            h.write_u8(1);
            h.write_u64(t);
        }
    }
    // Power (Q62): the charge held, a bot's `flat`, and a building's
    // `powered` and `grid`. The battery's capacity is the model's, in data.
    h.write_bytes(&r.charge.raw().to_le_bytes());
    for flag in [r.flat, r.powered] {
        match flag {
            None => h.write_u8(0),
            Some(b) => h.write_u8(u8::from(b).saturating_add(1)),
        }
    }
    match r.grid {
        None => h.write_u8(0),
        Some(None) => h.write_u8(1),
        Some(Some(id)) => {
            h.write_u8(2);
            h.write_u32(id.0);
        }
    }
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
