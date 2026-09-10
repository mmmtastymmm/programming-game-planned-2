//! The game builtins (`docs/02`, Senses and Actions) as the language's
//! host: the queries answer at once, the actions begin and the machine
//! waits, and the interrupt prologues do what `docs/02` says to the body.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::records::{list, machine_record, pos_value, sound_record, tile_record};
use crate::sense;
use crate::world::{
    Action, Effect, EntityId, LogEntry, MachineRecord, Model, PickSource, Plan, Slot, World,
};
use lang::errors::{ExcClass, Exception};
use lang::value::{R, Value};
use lang::{Host, HostCall, Interrupt, Num, Program};

/// One machine's view of the world for one slice.
pub struct GameHost<'a> {
    pub world: &'a mut World,
    pub id: EntityId,
}

fn value_error() -> Exception {
    Exception::value_error()
}

fn type_error() -> Exception {
    Exception::type_error()
}

fn coord(v: &Value) -> R<i32> {
    let i = v.expect_integral()?;
    i32::try_from(i).map_err(|_| value_error())
}

fn str_arg(v: &Value) -> R<String> {
    Ok(v.expect_str()?.to_string())
}

impl<'a> GameHost<'a> {
    fn me(&self) -> &crate::world::Machine {
        self.world
            .machines
            .get(&self.id)
            .expect("the running machine exists")
    }

    fn me_mut(&mut self) -> &mut crate::world::Machine {
        self.world
            .machines
            .get_mut(&self.id)
            .expect("the running machine exists")
    }

    fn team(&self) -> TeamId {
        self.me().team
    }

    fn pos(&self) -> TilePos {
        self.me().pos
    }

    /// The elements a list query returned, priced by `factor.traverse`
    /// each (`docs/02`, Costs).
    fn charged(&self, value: Value, n: usize) -> HostCall {
        let extra = (n as u64).saturating_mul(self.world.costs.factor_traverse);
        HostCall::Charged { value, extra }
    }

    // ---------------------------------------------------------- queries ---

    fn see(&mut self) -> HostCall {
        let team = self.team();
        let here = self.pos();
        let seen = sense::team_sees(self.world, team);
        let mut records: Vec<MachineRecord> = seen
            .iter()
            .filter_map(|id| self.world.machines.get(id))
            .map(MachineRecord::of)
            .collect();
        records.sort_by(|a, b| {
            here.dist2(a.pos)
                .cmp(&here.dist2(b.pos))
                .then(a.id.cmp(&b.id))
        });
        let n = records.len();
        let values: Vec<Value> = records
            .iter()
            .map(|r| machine_record(self.world, r))
            .collect();
        let v = list(self.world, values);
        self.charged(v, n)
    }

    fn hear(&mut self) -> HostCall {
        let team = self.team();
        let here = self.pos();
        let idx = sense::team_hears(self.world, team);
        let mut sounds: Vec<crate::world::Sound> = idx
            .into_iter()
            .map(|i| self.world.sounds_prev[i].clone())
            .collect();
        sounds.sort_by(|a, b| {
            here.dist2(a.pos)
                .cmp(&here.dist2(b.pos))
                .then(crate::map::row_then_column(&a.pos, &b.pos))
                .then(a.cause.cmp(&b.cause))
        });
        let n = sounds.len();
        let values: Vec<Value> = sounds.iter().map(sound_record).collect();
        let v = list(self.world, values);
        self.charged(v, n)
    }

    fn tiles(&mut self, args: &[Value]) -> R<HostCall> {
        let [x0, y0, x1, y1] = args else {
            return Err(type_error());
        };
        let (x0, y0, x1, y1) = (coord(x0)?, coord(y0)?, coord(x1)?, coord(y1)?);
        let (lo, hi) = self.world.map.bounds();
        let xa = x0.min(x1).max(lo.x);
        let xb = x0.max(x1).min(hi.x);
        let ya = y0.min(y1).max(lo.y);
        let yb = y0.max(y1).min(hi.y);
        let team = self.team();
        let mut values = Vec::new();
        if xa <= xb && ya <= yb {
            // Row then column: north to south, then west to east.
            let mut y = yb;
            loop {
                for x in xa..=xb {
                    let p = TilePos::new(x, y);
                    let r = tile_record(self.world, team, p);
                    if !matches!(r, Value::None) {
                        values.push(r);
                    }
                }
                if y == ya {
                    break;
                }
                y = y.saturating_sub(1);
            }
        }
        let n = values.len();
        let v = list(self.world, values);
        Ok(self.charged(v, n))
    }

    fn plans(&mut self, args: &[Value]) -> R<HostCall> {
        let [kind] = args else {
            return Err(type_error());
        };
        let kind = str_arg(kind)?;
        let team = self.team();
        let mut out = Vec::new();
        for p in self.world.map.positions() {
            let Some(plans) = self.world.plans(team, p) else {
                continue;
            };
            let plan = match kind.as_str() {
                "paint" => &plans.paint,
                "overlay" => &plans.overlay,
                "building" => &plans.building,
                _ => return Err(value_error()),
            };
            if let Some(pl) = plan {
                let v = match pl {
                    Plan::Set(s) => Value::str(s),
                    Plan::Clear => Value::None,
                };
                out.push(Value::tuple(vec![
                    crate::records::int(i128::from(p.x)),
                    crate::records::int(i128::from(p.y)),
                    v,
                ]));
            }
        }
        let n = out.len();
        let v = list(self.world, out);
        Ok(self.charged(v, n))
    }

    /// `painted(color)` / `overlaid(label)`: tiles seen or remembered
    /// whose slot holds the value, row then column.
    fn marked(&mut self, args: &[Value], slot: Slot) -> R<HostCall> {
        let [wanted] = args else {
            return Err(type_error());
        };
        let wanted = str_arg(wanted)?;
        let team = self.team();
        let mut out = Vec::new();
        for p in self.world.map.positions() {
            let idx = self.world.tile_index(p).expect("in bounds");
            let mem = &self.world.teams[&team].memory[idx];
            let value = match mem.state {
                crate::world::TileState::Unknown => None,
                crate::world::TileState::Visible => {
                    let t = &self.world.tiles[idx];
                    match slot {
                        Slot::Paint => t.paint.clone(),
                        Slot::Overlay => t.overlay.clone(),
                    }
                }
                crate::world::TileState::Remembered => match slot {
                    Slot::Paint => mem.paint.clone(),
                    Slot::Overlay => mem.overlay.clone(),
                },
            };
            if value.as_deref() == Some(wanted.as_str()) {
                out.push(pos_value(p));
            }
        }
        let n = out.len();
        let v = list(self.world, out);
        Ok(self.charged(v, n))
    }

    // ---------------------------------------------------------- actions ---

    /// Begin an action: refuse if busy or dying (unless allowed), then set
    /// it and emit its sound.
    fn begin(
        &mut self,
        name: &'static str,
        ticks: u64,
        effect: Effect,
        noisy: bool,
    ) -> R<HostCall> {
        let me = self.me();
        if me.action.is_some() {
            return Err(value_error());
        }
        if me.dying && !matches!(name, "wait" | "drop") {
            return Err(value_error());
        }
        let pos = me.pos;
        self.me_mut().action = Some(Action {
            name,
            remaining: ticks,
            effect,
        });
        if noisy {
            self.world.sound(name, pos);
        }
        Ok(HostCall::Wait)
    }

    fn require_bot(&self) -> R<()> {
        if self.me().is_bot() {
            Ok(())
        } else {
            Err(value_error())
        }
    }

    fn model_data(&self) -> crate::data::ModelData {
        self.world.data.model(self.me().model.name()).clone()
    }

    fn move_to_tile(&mut self, to: TilePos) -> R<HostCall> {
        if !self.world.passable(to) || self.world.occupied(to) {
            return Err(value_error());
        }
        let ticks = self.model_data().move_ticks;
        self.begin("move", ticks, Effect::Move { to }, true)
    }

    fn action_move(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [dir] = args else {
            return Err(type_error());
        };
        let here = self.pos();
        let n = here.neighbours();
        let to = match &*dir.expect_str()? {
            "n" => n[0],
            "e" => n[1],
            "s" => n[2],
            "w" => n[3],
            _ => return Err(value_error()),
        };
        self.move_to_tile(to)
    }

    fn action_move_to(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [x, y] = args else {
            return Err(type_error());
        };
        let target = TilePos::new(coord(x)?, coord(y)?);
        let here = self.pos();
        if here == target {
            return Err(value_error());
        }
        let mut best: Option<(i64, TilePos)> = None;
        for n in here.neighbours() {
            if self.world.passable(n) && !self.world.occupied(n) {
                let d = n.dist2(target);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, n));
                }
            }
        }
        let Some((_, to)) = best else {
            return Err(value_error());
        };
        self.move_to_tile(to)
    }

    fn action_pick(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [kind] = args else {
            return Err(type_error());
        };
        let kind = str_arg(kind)?;
        if !self.world.data.machines.kinds.contains(&kind) {
            return Err(value_error());
        }
        let here = self.pos();
        // Adjacent deposits, ties by lower x then y; then depots by id.
        let mut deposits: Vec<TilePos> = here
            .neighbours()
            .into_iter()
            .filter(|p| self.world.tile(*p).is_some_and(|t| t.deposit.is_some()))
            .collect();
        deposits.sort_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
        let mut depots: Vec<EntityId> = self
            .world
            .machines
            .values()
            .filter(|m| m.model == Model::Depot && here.is_adjacent(m.pos))
            .map(|m| m.id)
            .collect();
        depots.sort();
        let from = match (deposits.first(), depots.first()) {
            (Some(p), _) => PickSource::Deposit(*p),
            (None, Some(id)) => PickSource::Depot(*id),
            (None, None) => return Err(value_error()),
        };
        if self.pick_amount(&kind, &from) <= Num::ZERO {
            return Err(value_error());
        }
        let ticks = self.model_data().pick_ticks;
        self.begin("pick", ticks, Effect::Pick { kind, from }, true)
    }

    /// `min(pick_rate, available, free capacity)` now.
    pub fn pick_amount(&self, kind: &str, from: &PickSource) -> Num {
        let me = self.me();
        let md = self.world.data.model("bot");
        let rate = md.pick_rate.get(kind).copied().unwrap_or(Num::ZERO);
        let free = md
            .capacity
            .get(kind)
            .copied()
            .unwrap_or(Num::ZERO)
            .checked_sub(me.load.get(kind).copied().unwrap_or(Num::ZERO))
            .unwrap_or(Num::ZERO);
        let available = match from {
            PickSource::Deposit(p) => self
                .world
                .tile(*p)
                .and_then(|t| t.deposit.as_ref())
                .and_then(|d| d.amount.get(kind).copied())
                .unwrap_or(Num::ZERO),
            PickSource::Depot(id) => self
                .world
                .machines
                .get(id)
                .and_then(|m| m.store.get(kind).copied())
                .unwrap_or(Num::ZERO),
        };
        rate.min(available).min(free).max(Num::ZERO)
    }

    fn action_drop(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [kind] = args else {
            return Err(type_error());
        };
        let kind = str_arg(kind)?;
        if !self.world.data.machines.kinds.contains(&kind) {
            return Err(value_error());
        }
        let here = self.pos();
        // The adjacent building or site with the most free capacity, ties
        // by lower id; a printer never qualifies.
        let mut best: Option<(Num, EntityId)> = None;
        for m in self.world.machines.values() {
            if !m.is_building() || m.model == Model::Printer || !here.is_adjacent(m.pos) {
                continue;
            }
            let free = m
                .store_capacity
                .get(&kind)
                .copied()
                .unwrap_or(Num::ZERO)
                .checked_sub(m.store.get(&kind).copied().unwrap_or(Num::ZERO))
                .unwrap_or(Num::ZERO);
            if free <= Num::ZERO {
                continue;
            }
            if best.is_none_or(|(bf, _)| free > bf) {
                best = Some((free, m.id));
            }
        }
        let Some((_, into)) = best else {
            return Err(value_error());
        };
        if self.me().load.get(&kind).copied().unwrap_or(Num::ZERO) <= Num::ZERO {
            return Err(value_error());
        }
        let ticks = self.model_data().drop_ticks;
        self.begin("drop", ticks, Effect::Drop { kind, into }, true)
    }

    fn action_print(&mut self, args: &[Value]) -> R<HostCall> {
        if self.me().model != Model::Printer {
            return Err(value_error());
        }
        let [color] = args else {
            return Err(type_error());
        };
        let color = str_arg(color)?;
        let team = self.team();
        if !self.world.unlocked_colors(team).contains(&color) {
            return Err(value_error());
        }
        let has_bundle = self.world.teams[&team]
            .deployments
            .get(&color)
            .is_some_and(|d| d.bundle.is_some());
        if !has_bundle {
            return Err(value_error());
        }
        if self.world.bots_of(team) >= self.world.bot_cap(team) {
            return Err(value_error());
        }
        let ticks = self.model_data().print_ticks;
        self.begin("print", ticks, Effect::Print { color }, true)
    }

    /// `build` on a given tile: the checks shared with `build_nearest`.
    fn build_at(&mut self, model: &str, at: TilePos, must_be_adjacent: bool) -> R<HostCall> {
        let model = match model {
            "depot" => Model::Depot,
            _ => return Err(value_error()),
        };
        let here = self.pos();
        if must_be_adjacent && !here.is_adjacent(at) {
            return Err(value_error());
        }
        let team = self.team();
        let Some(tile) = self.world.tile(at) else {
            return Err(value_error());
        };
        if tile.terrain != crate::map::Terrain::Ground || tile.deposit.is_some() {
            return Err(value_error());
        }
        let standing = self
            .world
            .machines
            .values()
            .find(|m| m.pos == at && m.is_building())
            .map(|m| (m.id, m.model));
        let planned = matches!(self.world.plans(team, at).and_then(|p| p.building.as_ref()), Some(Plan::Set(v)) if v == model.name());
        let (remove, ticks) = match standing {
            None => (None, 0),
            // A planned tile with a building: deconstruct first (Q28).
            Some((id, old)) if planned => (
                Some(id),
                self.world.data.model(old.name()).deconstruct_ticks,
            ),
            Some(_) => return Err(value_error()),
        };
        if remove.is_none() && self.world.machines.values().any(|m| m.pos == at) {
            // A bot stands there: not free.
            return Err(value_error());
        }
        self.begin("build", ticks, Effect::Build { model, at, remove }, true)
    }

    fn action_build(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [model, x, y] = args else {
            return Err(type_error());
        };
        let model = str_arg(model)?;
        let at = TilePos::new(coord(x)?, coord(y)?);
        self.build_at(&model, at, true)
    }

    fn action_build_nearest(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [model] = args else {
            return Err(type_error());
        };
        let model = str_arg(model)?;
        let here = self.pos();
        let reach = self.model_data().build_reach;
        let mut best: Option<(i64, TilePos)> = None;
        for p in self.world.map.positions() {
            let d = here.dist2(p);
            if d > reach.saturating_mul(reach) || d == 0 {
                continue;
            }
            if !self.world.buildable(p) || self.world.occupied(p) {
                continue;
            }
            let better = match best {
                None => true,
                Some((bd, bp)) => {
                    d < bd || (d == bd && (p.x < bp.x || (p.x == bp.x && p.y < bp.y)))
                }
            };
            if better {
                best = Some((d, p));
            }
        }
        let Some((_, at)) = best else {
            return Err(value_error());
        };
        self.build_at(&model, at, false)
    }

    fn action_paint(&mut self, args: &[Value], slot: Slot, set: bool) -> R<HostCall> {
        self.require_bot()?;
        let [x, y] = args else {
            return Err(type_error());
        };
        let at = TilePos::new(coord(x)?, coord(y)?);
        let here = self.pos();
        if !here.is_adjacent(at) {
            return Err(value_error());
        }
        let team = self.team();
        let Some(tile) = self.world.tile(at) else {
            return Err(value_error());
        };
        let current = match slot {
            Slot::Paint => tile.paint.clone(),
            Slot::Overlay => tile.overlay.clone(),
        };
        let md = self.model_data();
        let (name, value, ticks): (&'static str, Option<String>, u64) = if set {
            let plan = self.world.plans(team, at).and_then(|p| match slot {
                Slot::Paint => p.paint.clone(),
                Slot::Overlay => p.overlay.clone(),
            });
            let Some(Plan::Set(v)) = plan else {
                return Err(value_error());
            };
            let mut ticks = match slot {
                Slot::Paint => md.paint_ticks,
                Slot::Overlay => md.overlay_ticks,
            };
            if current.is_some() {
                ticks = ticks.saturating_add(match slot {
                    Slot::Paint => md.unpaint_ticks,
                    Slot::Overlay => md.unoverlay_ticks,
                });
            }
            (
                match slot {
                    Slot::Paint => "paint",
                    Slot::Overlay => "overlay",
                },
                Some(v),
                ticks,
            )
        } else {
            if current.is_none() {
                return Err(value_error());
            }
            let ticks = match slot {
                Slot::Paint => md.unpaint_ticks,
                Slot::Overlay => md.unoverlay_ticks,
            };
            (
                match slot {
                    Slot::Paint => "unpaint",
                    Slot::Overlay => "unoverlay",
                },
                None,
                ticks,
            )
        };
        self.begin(name, ticks, Effect::Paint { at, slot, value }, false)
    }

    fn action_deconstruct(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [x, y] = args else {
            return Err(type_error());
        };
        let at = TilePos::new(coord(x)?, coord(y)?);
        if !self.pos().is_adjacent(at) {
            return Err(value_error());
        }
        let Some((target, model)) = self
            .world
            .machines
            .values()
            .find(|m| m.pos == at && m.is_building())
            .map(|m| (m.id, m.model))
        else {
            return Err(value_error());
        };
        let ticks = self.world.data.model(model.name()).deconstruct_ticks;
        self.begin("deconstruct", ticks, Effect::Deconstruct { target }, true)
    }

    fn action_attack(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [id] = args else {
            return Err(type_error());
        };
        let target = EntityId(u32::try_from(id.expect_integral()?).map_err(|_| value_error())?);
        let team = self.team();
        if !sense::team_can_see(self.world, team, target) {
            return Err(value_error());
        }
        let md = self.model_data();
        let tpos = self.world.machines[&target].pos;
        let here = self.pos();
        if here.dist2(tpos) > md.attack_range.saturating_mul(md.attack_range)
            || !sense::clear(self.world, here, tpos)
        {
            return Err(value_error());
        }
        self.begin("attack", md.attack_ticks, Effect::Attack { target }, true)
    }

    fn action_convert(&mut self, args: &[Value]) -> R<HostCall> {
        self.require_bot()?;
        let [id] = args else {
            return Err(type_error());
        };
        let target = EntityId(u32::try_from(id.expect_integral()?).map_err(|_| value_error())?);
        let team = self.team();
        let here = self.pos();
        let ok = self.world.machines.get(&target).is_some_and(|m| {
            m.model == Model::Printer && m.team != team && here.is_adjacent(m.pos)
        });
        if !ok {
            return Err(value_error());
        }
        let ticks = self.model_data().convert_ticks;
        self.begin("convert", ticks, Effect::Convert { target }, true)
    }

    fn action_wait(&mut self, args: &[Value]) -> R<HostCall> {
        let [n] = args else {
            return Err(type_error());
        };
        let n = n.expect_integral()?;
        if n < 1 {
            return Err(value_error());
        }
        let ticks = u64::try_from(n).map_err(|_| value_error())?;
        self.begin("wait", ticks, Effect::Wait, false)
    }

    fn log(&mut self, args: &[Value], kwargs: &[(String, Value)]) -> R<HostCall> {
        let mut level = "info".to_string();
        for (k, v) in kwargs {
            if k == "level" {
                level = str_arg(v)?;
                if !matches!(level.as_str(), "debug" | "info" | "warn" | "error") {
                    return Err(value_error());
                }
            } else {
                return Err(type_error());
            }
        }
        let text = args
            .iter()
            .map(Value::to_str_value)
            .collect::<Vec<_>>()
            .join(" ");
        let tick = self.world.tick;
        self.me_mut().log.push(LogEntry { tick, level, text });
        Ok(HostCall::Value(Value::None))
    }

    fn rename(&mut self, args: &[Value]) -> R<HostCall> {
        let [name] = args else {
            return Err(type_error());
        };
        let name = str_arg(name)?;
        if name.chars().count() > self.world.data.machines.name_max {
            return Err(value_error());
        }
        self.me_mut().name = name;
        Ok(HostCall::Value(Value::None))
    }

    /// Cancel the action in progress (a fault, dying or redeploy prologue).
    fn cancel(&mut self) {
        self.me_mut().action = None;
    }
}

impl<'a> Host for GameHost<'a> {
    fn call(&mut self, name: &str, args: Vec<Value>, kwargs: Vec<(String, Value)>) -> R<HostCall> {
        if !kwargs.is_empty() && name != "log" {
            return Err(type_error());
        }
        match name {
            "see" => Ok(self.see()),
            "hear" => Ok(self.hear()),
            "tile" => {
                let [x, y] = &args[..] else {
                    return Err(type_error());
                };
                let p = TilePos::new(coord(x)?, coord(y)?);
                let team = self.team();
                Ok(HostCall::Value(tile_record(self.world, team, p)))
            }
            "tiles" => self.tiles(&args),
            "me" => {
                let r = MachineRecord::of(self.me());
                Ok(HostCall::Value(machine_record(self.world, &r)))
            }
            "plans" => self.plans(&args),
            "painted" => self.marked(&args, Slot::Paint),
            "overlaid" => self.marked(&args, Slot::Overlay),
            "move" => self.action_move(&args),
            "move_to" => self.action_move_to(&args),
            "pick" => self.action_pick(&args),
            "drop" => self.action_drop(&args),
            "print" => self.action_print(&args),
            "build" => self.action_build(&args),
            "build_nearest" => self.action_build_nearest(&args),
            "paint" => self.action_paint(&args, Slot::Paint, true),
            "unpaint" => self.action_paint(&args, Slot::Paint, false),
            "overlay" => self.action_paint(&args, Slot::Overlay, true),
            "unoverlay" => self.action_paint(&args, Slot::Overlay, false),
            "deconstruct" => self.action_deconstruct(&args),
            "attack" => self.action_attack(&args),
            "convert" => self.action_convert(&args),
            "wait" => self.action_wait(&args),
            "log" => self.log(&args, &kwargs),
            "rename" => self.rename(&args),
            _ => Ok(HostCall::Unknown),
        }
    }

    fn current_program(&mut self) -> Option<Program> {
        let team = self.team();
        let deployment = self.me().deployment.clone()?;
        Some(self.world.program_of(team, &deployment))
    }

    /// `docs/02`, the interrupts table: what each prologue does to the body.
    fn on_prologue(&mut self, kind: Interrupt) -> Option<Interrupt> {
        match kind {
            Interrupt::Fault => {
                self.cancel();
                let damage = self.world.data.machines.fault_damage;
                let threshold = self.model_data().death_threshold;
                let me = self.me_mut();
                if me.dying {
                    return None;
                }
                me.health = me.health.checked_sub(damage).unwrap_or(Num::MIN);
                if me.health <= threshold {
                    Some(Interrupt::Death)
                } else if me.health <= Num::ZERO {
                    Some(Interrupt::Dying)
                } else {
                    None
                }
            }
            Interrupt::Dying => {
                self.cancel();
                self.me_mut().dying = true;
                None
            }
            Interrupt::Redeploy => {
                self.cancel();
                None
            }
            Interrupt::Death => None,
        }
    }
}

/// Exception classes the host raises, for tests.
pub fn is_value_error(e: &Exception) -> bool {
    e.class == ExcClass::ValueError
}
