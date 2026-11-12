//! Forks (`docs/06`, Forks): a private copy of the world for the debugger
//! and a replay's playback. A [`Fork`] is the match's sim's code run on a
//! copy, with entry points the match's [`Sim`] does not have: it stops at a
//! position mid-tick and resumes, is restored to a saved state, and takes
//! edits and what-if deploys into a log of its own. Nothing here reaches
//! the match's world, log or hash.
//!
//! The match's side of this file is small and read-only: [`Sim::save`]
//! copies the world, and [`Watch`]es passed to [`Sim::step`] report the
//! statements that began on watched lines.

use crate::command::{Bundle, Command, CommandKind, TeamId};
use crate::map::Map;
use crate::snapshot::{MachineSnapshot, Snapshot};
use crate::tick::{Sim, StepReport};
use crate::world::{EntityId, MachineRecord, World};
use lang::{EditTarget, Inspection, Interrupt, Tracer};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// The match's full state at the end of a tick, or a fork's.
pub struct Saved {
    pub(crate) world: World,
    pub(crate) ended: Option<Option<TeamId>>,
    /// For a fork's own state: how many entries of its log the state holds.
    pub(crate) entries: usize,
}

impl Saved {
    /// The tick whose end this is.
    pub fn tick(&self) -> u64 {
        self.world.tick
    }

    /// A second copy, sharing nothing either will change.
    pub fn copy(&self) -> Saved {
        Saved {
            world: self.world.copy(),
            ended: self.ended,
            entries: self.entries,
        }
    }

    pub(crate) fn sim(&self) -> Sim {
        Sim {
            world: self.world.copy(),
            ended: self.ended,
        }
    }
}

/// A watched line, for one team (`docs/06`, Watches).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    pub team: TeamId,
    /// The file as its bundle names it: `main.py`, `lib/jobs.py`.
    pub file: String,
    /// The content hash of the text the line was set on.
    pub content: u64,
    pub line: u32,
    /// The deployment whose machines it watches, or any.
    pub deployment: Option<String>,
    /// The one machine it watches, or any.
    pub machine: Option<EntityId>,
}

/// A point in a tick (`docs/06`, Positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// The `n`-th boundary the machine's slice passes this tick.
    Slice {
        tick: u64,
        machine: EntityId,
        n: u64,
    },
    /// Step 4, after the tick's damage and before the machine's removal.
    Damage { tick: u64, machine: EntityId },
    /// After step 9.
    End { tick: u64 },
}

impl Position {
    pub fn tick(self) -> u64 {
        match self {
            Position::Slice { tick, .. }
            | Position::Damage { tick, .. }
            | Position::End { tick } => tick,
        }
    }

    fn key(self) -> (u64, u8, u32, u64) {
        match self {
            Position::Slice { tick, machine, n } => (tick, 0, machine.0, n),
            Position::Damage { tick, machine } => (tick, 1, machine.0, 0),
            Position::End { tick } => (tick, 2, 0, 0),
        }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Position) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Position) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A machine's first beginning on a watched line in a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub machine: EntityId,
    pub position: Position,
    pub file: String,
    pub line: u32,
    /// How many more such beginnings the machine made that tick.
    pub more: u64,
}

/// What a watch is matched against.
pub(crate) struct Who {
    pub id: EntityId,
    pub team: TeamId,
    pub deployment: Option<String>,
}

fn fits(w: &Watch, who: &Who) -> bool {
    w.team == who.team
        && w.deployment
            .as_ref()
            .is_none_or(|d| Some(d) == who.deployment.as_ref())
        && w.machine.is_none_or(|m| m == who.id)
}

/// A tick's watches by file and line.
pub(crate) struct WatchIndex<'a> {
    by: BTreeMap<(&'a str, u32), Vec<&'a Watch>>,
    all: &'a [Watch],
}

impl<'a> WatchIndex<'a> {
    pub(crate) fn new(watches: &'a [Watch]) -> WatchIndex<'a> {
        let mut by: BTreeMap<(&'a str, u32), Vec<&'a Watch>> = BTreeMap::new();
        for w in watches {
            by.entry((w.file.as_str(), w.line)).or_default().push(w);
        }
        WatchIndex { by, all: watches }
    }

    /// Whether any watch could ever hit this machine: one that cannot is
    /// run with no tracer at all.
    pub(crate) fn fits(&self, who: &Who) -> bool {
        self.all.iter().any(|w| fits(w, who))
    }

    /// The watched line the machine begins a statement on at the boundary
    /// it stands at, if it does: a watch on that file and line, for the
    /// text the machine runs, that fits the machine.
    pub(crate) fn matches(&self, who: &Who, vm: &lang::Machine) -> Option<(String, u32)> {
        let (file, line) = vm.beginning()?;
        let list = self.by.get(&(file, line))?;
        let content = *vm.program().content.get(file)?;
        list.iter()
            .any(|w| w.content == content && fits(w, who))
            .then(|| (file.to_string(), line))
    }
}

/// The match's tracer: it counts beginnings on watched lines and never
/// stops a slice.
pub(crate) struct Watcher<'a> {
    index: &'a WatchIndex<'a>,
    who: Who,
    tick: u64,
    hit: Option<Hit>,
}

impl<'a> Watcher<'a> {
    pub(crate) fn new(index: &'a WatchIndex<'a>, who: Who, tick: u64) -> Watcher<'a> {
        Watcher {
            index,
            who,
            tick,
            hit: None,
        }
    }

    pub(crate) fn hit(self) -> Option<Hit> {
        self.hit
    }
}

impl Tracer for Watcher<'_> {
    fn boundary(&mut self, vm: &lang::Machine) -> bool {
        if let Some((file, line)) = self.index.matches(&self.who, vm) {
            match &mut self.hit {
                Some(h) => h.more = h.more.saturating_add(1),
                None => {
                    self.hit = Some(Hit {
                        machine: self.who.id,
                        position: Position::Slice {
                            tick: self.tick,
                            machine: self.who.id,
                            n: vm.boundary(),
                        },
                        file,
                        line,
                        more: 0,
                    });
                }
            }
        }
        false
    }
}

/// The match's command log as a fork reads it (`docs/06`, Its commands).
pub trait Timeline {
    /// The commands the log holds for `tick` at this moment. Tick 1's
    /// include tick 0's opening set.
    fn commands(&self, tick: u64) -> Vec<Command>;
    /// The last tick whose sets are final: the match's latest completed
    /// tick, or a replay's last.
    fn final_tick(&self) -> u64;
}

/// How far a step runs (`docs/06`, Steps). *Run* is [`Fork::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Line,
    Over,
    Tick,
    Continue,
}

/// Why a fork stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The fork was opened here.
    Opened,
    /// The selected machine begins a statement.
    Statement,
    /// A machine begins a statement on a watched line.
    Watch,
    /// The selected machine is about to be delivered `dying`.
    Dying,
    /// The selected machine's program restarted.
    Restart,
    /// The selected machine's `death`: in its slice, or in step 4.
    Death,
    /// The selected machine's team is out and step 5 removed it.
    TeamOut,
    /// A *tick* step reached the tick's end.
    TickEnd,
    /// A *run* was paused.
    Paused,
    /// The step ran its limit of ticks with no other stop.
    StepLimit,
    /// A live PvP match's latest completed tick: the fork goes no further.
    LiveTick,
    /// The fork's world's match ended.
    MatchEnded,
}

/// A stop, with its cause and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub cause: Cause,
    pub position: Position,
    /// The machine the stop is about; a stop on another machine than the
    /// selected one selects it.
    pub machine: Option<EntityId>,
    /// A watch stop's one hit, or a tick step's hits as `sim.step`
    /// reports them.
    pub hits: Vec<Hit>,
}

/// A fork's state (`docs/06`, Its state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    /// Past the last tick whose sets are final.
    pub ahead: bool,
    /// The first tick the match's log later gained a command for that the
    /// fork did not take.
    pub stale: Option<u64>,
    /// The fork's log has an entry.
    pub diverged: bool,
}

impl Flags {
    pub fn faithful(self) -> bool {
        !self.ahead && self.stale.is_none() && !self.diverged
    }
}

/// What a fork reports of one machine at the position it stands at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspected {
    /// Its attributes, action in progress and `progress` included.
    pub record: MachineRecord,
    /// `None` for a machine with no program.
    pub program: Option<Inspection>,
}

/// One entry of a fork's log.
#[derive(Clone)]
struct Entry {
    at: Position,
    what: What,
}

#[derive(Clone)]
enum What {
    /// A what-if deploy, applied in step 1 of the tick after `at`'s.
    Deploy { deployment: String, bundle: Bundle },
    Edit {
        machine: EntityId,
        target: EditTarget,
        literal: String,
    },
}

/// Where in a tick a fork stands.
enum Stage {
    /// At a tick's end.
    Rest,
    /// In step 2, before or inside the slice of `ids[at]`; `mid` when that
    /// slice is stopped at a boundary.
    Slices {
        ids: Vec<EntityId>,
        at: usize,
        mid: bool,
        report: StepReport,
    },
    /// In step 4, the damage applied, before the removal of `dead[at]`;
    /// `asked` once the fork has stopped at that machine's position.
    Damage {
        dead: Vec<EntityId>,
        killed: BTreeSet<EntityId>,
        at: usize,
        asked: bool,
        report: StepReport,
    },
}

/// What a run of the engine is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Line,
    Over,
    Tick,
    Continue,
    Run,
    /// To the very next position.
    Nudge,
    /// To a position, or the first one after it.
    To(Position),
    /// One whole tick, as `sim.step` runs it.
    Advance,
}

impl Kind {
    fn selects(self) -> bool {
        matches!(self, Kind::Line | Kind::Over | Kind::Continue | Kind::Run)
    }

    /// Whether a run of this kind may stop inside the tick `tick`, so the
    /// fork needs the snapshot of the tick before it.
    fn mid_tick(self, tick: u64) -> bool {
        match self {
            Kind::Tick | Kind::Advance => false,
            Kind::To(target) => target.tick() == tick && !matches!(target, Position::End { .. }),
            _ => true,
        }
    }
}

/// How a run of the engine came to rest.
enum Arrival {
    Stop(Cause, Option<EntityId>),
    Reached,
}

/// One run of the engine: what it is after and what it has seen.
struct Ctl<'a> {
    kind: Kind,
    team: TeamId,
    selected: Option<EntityId>,
    index: Option<WatchIndex<'a>>,
    /// The position of the fork's next edit not yet applied.
    edit_at: Option<Position>,
    /// *Over*'s frame: the least depth the selected machine's stack has
    /// had since the step began.
    floor: usize,
    abandons: u64,
    restarts: u64,
    ticks: u64,
    limit: u64,
    hits: BTreeMap<EntityId, Hit>,
    arrival: Option<Arrival>,
    closed: Option<StepReport>,
}

impl Ctl<'_> {
    fn watched(&self, who: &Who) -> bool {
        who.team == self.team && self.index.as_ref().is_some_and(|i| i.fits(who))
    }

    /// Whether this machine's slice needs a tracer.
    fn traces(&self, who: &Who, tick: u64) -> bool {
        let here = |p: Position| matches!(p, Position::Slice { tick: t, machine, .. } if t == tick && machine == who.id);
        if self.edit_at.is_some_and(here) {
            return true;
        }
        match self.kind {
            Kind::Nudge => true,
            Kind::To(target) => here(target),
            Kind::Tick | Kind::Advance => self.watched(who),
            Kind::Line | Kind::Over | Kind::Continue | Kind::Run => {
                self.selected == Some(who.id) || self.watched(who)
            }
        }
    }

    fn hit(&self, who: &Who, pos: Position, vm: &lang::Machine) -> Option<Hit> {
        if who.team != self.team {
            return None;
        }
        let (file, line) = self.index.as_ref()?.matches(who, vm)?;
        Some(Hit {
            machine: who.id,
            position: pos,
            file,
            line,
            more: 0,
        })
    }

    /// Whether the run ends at this boundary, and how.
    fn arrives(&mut self, who: &Who, pos: Position, vm: &lang::Machine) -> Option<Arrival> {
        match self.kind {
            Kind::To(target) => return (pos >= target).then_some(Arrival::Reached),
            Kind::Nudge => return Some(Arrival::Reached),
            Kind::Tick | Kind::Advance => {
                if let Some(hit) = self.hit(who, pos, vm) {
                    self.hits
                        .entry(who.id)
                        .and_modify(|h| h.more = h.more.saturating_add(1))
                        .or_insert(hit);
                }
                return None;
            }
            Kind::Line | Kind::Over | Kind::Continue | Kind::Run => {}
        }
        let own = self.selected == Some(who.id);
        if own {
            match vm.delivering() {
                Some(Interrupt::Dying) => return Some(Arrival::Stop(Cause::Dying, Some(who.id))),
                Some(Interrupt::Death) => return Some(Arrival::Stop(Cause::Death, Some(who.id))),
                _ => {}
            }
            let trace = vm.trace();
            if trace.restarts != self.restarts {
                return Some(Arrival::Stop(Cause::Restart, Some(who.id)));
            }
            self.floor = if trace.abandons != self.abandons {
                usize::MAX
            } else {
                self.floor.min(vm.depth())
            };
        }
        vm.beginning()?;
        if let Some(hit) = self.hit(who, pos, vm) {
            self.hits.clear();
            self.hits.insert(who.id, hit);
            return Some(Arrival::Stop(Cause::Watch, Some(who.id)));
        }
        let stops = match self.kind {
            Kind::Line => own,
            Kind::Over => own && vm.depth() <= self.floor,
            _ => false,
        };
        stops.then_some(Arrival::Stop(Cause::Statement, Some(who.id)))
    }

    /// A boundary of a traced slice: `true` stops the slice at it, for an
    /// arrival or for an edit the fork's log holds at this position.
    fn boundary(&mut self, who: &Who, tick: u64, vm: &lang::Machine) -> bool {
        let pos = Position::Slice {
            tick,
            machine: who.id,
            n: vm.boundary(),
        };
        if let Some(arrival) = self.arrives(who, pos, vm) {
            self.arrival = Some(arrival);
            return true;
        }
        self.edit_at.is_some_and(|at| at <= pos)
    }

    /// Whether the run ends at a machine's damage position.
    fn at_damage(&self, pos: Position, id: EntityId) -> Option<Arrival> {
        match self.kind {
            Kind::To(target) => (pos >= target).then_some(Arrival::Reached),
            Kind::Nudge => Some(Arrival::Reached),
            Kind::Tick | Kind::Advance => None,
            Kind::Line | Kind::Over | Kind::Continue | Kind::Run => {
                (self.selected == Some(id)).then_some(Arrival::Stop(Cause::Death, Some(id)))
            }
        }
    }
}

/// The tracer a fork runs a slice with.
struct Tap<'c, 'a> {
    ctl: &'c mut Ctl<'a>,
    who: Who,
    tick: u64,
}

impl Tracer for Tap<'_, '_> {
    fn boundary(&mut self, vm: &lang::Machine) -> bool {
        self.ctl.boundary(&self.who, self.tick, vm)
    }
}

/// A private copy of the world (`docs/06`, Forks).
pub struct Fork {
    sim: Sim,
    stage: Stage,
    team: TeamId,
    live_pvp: bool,
    /// The tick the fork opened at: every tick up to it was final in the
    /// match's log then, and a re-run reads the log for them.
    opened_at: u64,
    /// The last tick the fork has run: for the ticks between, a re-run
    /// reads `taken`, never the match's log again.
    ran_through: u64,
    /// The commands the fork took, for the ticks it ran that had any.
    taken: BTreeMap<u64, Vec<Command>>,
    /// Ticks taken before their sets were final.
    unfinal: BTreeSet<u64>,
    stale: Option<u64>,
    /// The fork's own log, in position order.
    log: Vec<Entry>,
    /// How many of the log's entries the world stands after.
    applied: usize,
    saved: BTreeMap<u64, Saved>,
    /// The interval and count [`Fork::keep_every`] set, if it was called.
    every: Option<(u64, usize)>,
    stops: Vec<Stop>,
    /// The snapshot of the last completed tick, kept while the fork may
    /// stop inside the next.
    base: Option<Snapshot>,
}

fn settled(mut commands: Vec<Command>) -> Vec<Command> {
    commands.retain(|c| !matches!(c.kind, CommandKind::SetSpeed(_)));
    commands.sort_by(|a, b| {
        a.tick
            .cmp(&b.tick)
            .then(a.sender.cmp(&b.sender))
            .then(a.seq.cmp(&b.seq))
    });
    commands
}

impl Fork {
    fn at(sim: Sim, team: TeamId, live_pvp: bool) -> Fork {
        let tick = sim.world.tick;
        Fork {
            sim,
            stage: Stage::Rest,
            team,
            live_pvp,
            opened_at: tick,
            ran_through: tick,
            taken: BTreeMap::new(),
            unfinal: BTreeSet::new(),
            stale: None,
            log: Vec::new(),
            applied: 0,
            saved: BTreeMap::new(),
            every: None,
            stops: Vec::new(),
            base: None,
        }
    }

    /// A fork of a saved state, standing at that tick's end, stopped as
    /// *opened*. `team` is the team whose debugger it is; `live_pvp`
    /// bounds it to the match's latest tick and refuses it a log.
    pub fn open(saved: &Saved, team: TeamId, live_pvp: bool) -> Fork {
        let mut fork = Fork::at(saved.sim(), team, live_pvp);
        fork.record(Cause::Opened, None, Vec::new());
        fork
    }

    /// A fork opened at a hit: run from the saved state to the hit's
    /// position, its first stop the watch.
    pub fn open_at(
        saved: &Saved,
        hit: &Hit,
        team: TeamId,
        live_pvp: bool,
        timeline: &dyn Timeline,
    ) -> Result<Fork, String> {
        let mut fork = Fork::at(saved.sim(), team, live_pvp);
        fork.go_to(hit.position, &BTreeMap::new(), timeline);
        if fork.position() != hit.position {
            return Err("the saved state does not lead to the hit".into());
        }
        let hit = Hit {
            more: 0,
            ..hit.clone()
        };
        fork.record(Cause::Watch, Some(hit.machine), vec![hit]);
        Ok(fork)
    }

    /// A fork at tick 0 of a map: where a replay's playback starts.
    pub fn from_map(map: Map, team: TeamId) -> Fork {
        let mut fork = Fork::at(Sim::new(map), team, false);
        fork.record(Cause::Opened, None, Vec::new());
        fork
    }

    pub fn team(&self) -> TeamId {
        self.team
    }

    /// The fork's world, to read.
    pub fn world(&self) -> &World {
        &self.sim.world
    }

    /// The winner, or `None` for a draw, once the fork's match has ended.
    pub fn ended(&self) -> Option<Option<TeamId>> {
        self.sim.ended
    }

    /// Whether a command can ever apply in this fork's world, as the
    /// match's validation at submission checks one.
    pub fn validate(&self, c: &Command) -> Result<(), String> {
        self.sim.validate(c)
    }

    fn program(&self, id: EntityId) -> Option<&lang::Machine> {
        self.sim.world.machines.get(&id)?.program.as_ref()
    }

    /// The position the fork stands at.
    pub fn position(&self) -> Position {
        let tick = self.sim.world.tick;
        match &self.stage {
            Stage::Rest => Position::End { tick },
            Stage::Slices { ids, at, .. } => {
                let machine = ids.get(*at).copied().unwrap_or(EntityId(0));
                Position::Slice {
                    tick,
                    machine,
                    n: self.program(machine).map_or(0, |p| p.boundary()),
                }
            }
            Stage::Damage { dead, at, .. } => Position::Damage {
                tick,
                machine: dead.get(*at).copied().unwrap_or(EntityId(0)),
            },
        }
    }

    /// The state hash, which a fork stopped mid-tick does not have.
    pub fn state_hash(&self) -> Option<u64> {
        matches!(self.stage, Stage::Rest).then(|| self.sim.state_hash())
    }

    /// The stops so far, the latest last.
    pub fn stops(&self) -> &[Stop] {
        &self.stops
    }

    fn record(&mut self, cause: Cause, machine: Option<EntityId>, hits: Vec<Hit>) -> Stop {
        let stop = Stop {
            cause,
            position: self.position(),
            machine,
            hits,
        };
        match self.stops.last_mut() {
            // A fork that can go no further has not stopped anywhere new.
            Some(last)
                if matches!(cause, Cause::MatchEnded | Cause::LiveTick)
                    && last.position == stop.position =>
            {
                last.cause = cause;
            }
            _ => self.stops.push(stop.clone()),
        }
        stop
    }

    /// The commands the fork takes for `tick` (`docs/06`, Its commands).
    fn commands_for(&mut self, tick: u64, timeline: &dyn Timeline) -> Vec<Command> {
        if tick <= self.opened_at {
            return timeline.commands(tick);
        }
        if tick <= self.ran_through {
            return self.taken.get(&tick).cloned().unwrap_or_default();
        }
        let commands = timeline.commands(tick);
        if !commands.is_empty() {
            self.taken.insert(tick, commands.clone());
        }
        self.ran_through = tick;
        if tick > timeline.final_tick() {
            self.unfinal.insert(tick);
        }
        commands
    }

    /// The what-if deploys made in `tick`, as commands for `next`.
    fn what_ifs(&self, tick: u64, next: u64) -> Vec<Command> {
        let deploys = self.log.iter().filter(|e| e.at.tick() == tick);
        let deploys = deploys.filter_map(|e| match &e.what {
            What::Deploy { deployment, bundle } => Some((deployment, bundle)),
            What::Edit { .. } => None,
        });
        deploys
            .enumerate()
            .map(|(i, (deployment, bundle))| {
                Command::new(
                    next,
                    self.team,
                    u32::try_from(i).unwrap_or(u32::MAX),
                    CommandKind::Deploy {
                        deployment: deployment.clone(),
                        bundle: bundle.clone(),
                    },
                )
            })
            .collect()
    }

    /// The position of the next edit the world does not yet hold.
    fn edit_at(&self) -> Option<Position> {
        let rest = self.log.get(self.applied..)?;
        rest.iter()
            .find(|e| matches!(e.what, What::Edit { .. }))
            .map(|e| e.at)
    }

    /// Apply every entry of the log up to `pos`: standing at a position,
    /// the world holds every entry at or before it and none after.
    fn settle(&mut self, pos: Position) {
        while let Some(entry) = self.log.get(self.applied) {
            if entry.at > pos {
                break;
            }
            if let What::Edit {
                machine,
                target,
                literal,
            } = &entry.what
            {
                let program = self.sim.world.machines.get_mut(machine);
                if let Some(program) = program.and_then(|m| m.program.as_mut()) {
                    // A re-run reaches the state the edit was accepted in,
                    // so a refusal here means the machine is gone.
                    let _ = program.edit(target, literal);
                }
            }
            self.applied = self.applied.saturating_add(1);
        }
    }

    fn own(&self, id: Option<EntityId>) -> Option<EntityId> {
        let m = self.sim.world.machines.get(&id?)?;
        (m.team == self.team && m.program.is_some()).then_some(m.id)
    }

    fn ctl<'a>(
        &self,
        kind: Kind,
        selected: Option<EntityId>,
        watches: Option<&'a [Watch]>,
        limit: u64,
    ) -> Ctl<'a> {
        let selected = if kind.selects() {
            self.own(selected)
        } else {
            None
        };
        let program = selected.and_then(|s| self.program(s));
        let trace = program.map(|p| p.trace()).unwrap_or_default();
        Ctl {
            kind,
            team: self.team,
            selected,
            index: watches.map(WatchIndex::new),
            edit_at: self.edit_at(),
            floor: program.map_or(usize::MAX, |p| p.depth()),
            abandons: trace.abandons,
            restarts: trace.restarts,
            ticks: 0,
            limit,
            hits: BTreeMap::new(),
            arrival: None,
            closed: None,
        }
    }
}

impl Fork {
    /// Run the tick loop of `Sim::step`, phase for phase, until `ctl`
    /// says the run has arrived. The fork is left standing at a position:
    /// a tick's end, a boundary of a stopped slice, or a damage position.
    fn drive(&mut self, ctl: &mut Ctl, timeline: &dyn Timeline) -> Arrival {
        loop {
            match std::mem::replace(&mut self.stage, Stage::Rest) {
                Stage::Rest => {
                    let tick = self.sim.world.tick;
                    if matches!(ctl.kind, Kind::To(target) if Position::End { tick } >= target) {
                        return Arrival::Reached;
                    }
                    if self.sim.ended.is_some() {
                        return Arrival::Stop(Cause::MatchEnded, None);
                    }
                    if self.live_pvp && tick >= timeline.final_tick() {
                        return Arrival::Stop(Cause::LiveTick, None);
                    }
                    let next = tick.saturating_add(1);
                    let commands = self.commands_for(next, timeline);
                    let extra = self.what_ifs(tick, next);
                    self.base = ctl.kind.mid_tick(next).then(|| self.sim.snapshot());
                    let mut report = StepReport::default();
                    self.sim.open_tick(&commands, &extra, &mut report);
                    self.stage = Stage::Slices {
                        ids: self.sim.slice_ids(),
                        at: 0,
                        mid: false,
                        report,
                    };
                }
                Stage::Slices {
                    ids,
                    at,
                    mid,
                    report,
                } => {
                    let Some(&id) = ids.get(at) else {
                        self.sim.completions();
                        let killed = self.sim.damage_apply();
                        self.stage = Stage::Damage {
                            dead: self.sim.world.dead.iter().copied().collect(),
                            killed,
                            at: 0,
                            asked: false,
                            report,
                        };
                        continue;
                    };
                    let tick = self.sim.world.tick;
                    let who = self.sim.who(id).filter(|who| ctl.traces(who, tick));
                    let stopped = match who {
                        Some(who) => {
                            let mut tap = Tap {
                                ctl: &mut *ctl,
                                who,
                                tick,
                            };
                            self.sim.run_slice_with(id, Some(&mut tap), mid)
                        }
                        None => self.sim.run_slice_with(id, None, mid),
                    };
                    if !stopped {
                        self.stage = Stage::Slices {
                            ids,
                            at: at.saturating_add(1),
                            mid: false,
                            report,
                        };
                        continue;
                    }
                    self.stage = Stage::Slices {
                        ids,
                        at,
                        mid: true,
                        report,
                    };
                    self.settle(self.position());
                    ctl.edit_at = self.edit_at();
                    if let Some(arrival) = ctl.arrival.take() {
                        return arrival;
                    }
                }
                Stage::Damage {
                    dead,
                    killed,
                    at,
                    asked,
                    mut report,
                } => {
                    let tick = self.sim.world.tick;
                    let Some(&id) = dead.get(at) else {
                        self.sim.world.dead.clear();
                        let machines = &self.sim.world.machines;
                        let present = ctl.selected.is_some_and(|s| machines.contains_key(&s));
                        self.sim.close_tick(&mut report);
                        self.settle(Position::End { tick });
                        self.base = None;
                        self.keep_due();
                        ctl.edit_at = self.edit_at();
                        if let Some(arrival) = self.at_end(ctl, present, report) {
                            return arrival;
                        }
                        continue;
                    };
                    if killed.contains(&id) && !asked {
                        let pos = Position::Damage { tick, machine: id };
                        if let Some(arrival) = ctl.at_damage(pos, id) {
                            self.stage = Stage::Damage {
                                dead,
                                killed,
                                at,
                                asked: true,
                                report,
                            };
                            return arrival;
                        }
                    }
                    self.sim.world.remove(id);
                    self.stage = Stage::Damage {
                        dead,
                        killed,
                        at: at.saturating_add(1),
                        asked: false,
                        report,
                    };
                }
            }
        }
    }

    /// Whether the run ends at the end of the tick just closed. `present`
    /// is whether the selected machine stood before step 5.
    fn at_end(&self, ctl: &mut Ctl, present: bool, mut report: StepReport) -> Option<Arrival> {
        let tick = self.sim.world.tick;
        match ctl.kind {
            Kind::To(target) => {
                return (Position::End { tick } >= target).then_some(Arrival::Reached);
            }
            Kind::Nudge => return Some(Arrival::Reached),
            Kind::Advance => {
                report.hits = std::mem::take(&mut ctl.hits).into_values().collect();
                ctl.closed = Some(report);
                return Some(Arrival::Reached);
            }
            Kind::Line | Kind::Over | Kind::Tick | Kind::Continue | Kind::Run => {}
        }
        if self.sim.ended.is_some() {
            return Some(Arrival::Stop(Cause::MatchEnded, None));
        }
        let world = &self.sim.world;
        let gone = ctl.selected.filter(|s| !world.machines.contains_key(s));
        if let Some(id) = gone {
            // Step 5 took it with its team; any other removal stopped the
            // run at the machine's death, and the step after has no
            // machine to follow past this tick.
            let out = world.teams.get(&self.team).is_none_or(|t| t.out);
            return Some(if present && out {
                Arrival::Stop(Cause::TeamOut, Some(id))
            } else {
                Arrival::Stop(Cause::TickEnd, None)
            });
        }
        match ctl.kind {
            Kind::Tick => Some(Arrival::Stop(Cause::TickEnd, None)),
            Kind::Run => Some(Arrival::Reached),
            _ => {
                ctl.ticks = ctl.ticks.saturating_add(1);
                (ctl.ticks >= ctl.limit).then_some(Arrival::Stop(Cause::StepLimit, None))
            }
        }
    }

    /// Stand at `target`: run on to it from here, or from the latest
    /// usable saved state before it, or from the map (`docs/06`, Saved
    /// states and back).
    fn go_to(
        &mut self,
        target: Position,
        match_states: &BTreeMap<u64, Saved>,
        timeline: &dyn Timeline,
    ) {
        self.flags(timeline);
        let before = |s: &u64| Position::End { tick: *s } <= target;
        let own = self.saved.keys().rev().copied().find(before);
        let first = self.log.first().map(|e| e.at);
        let usable = |s: &u64| {
            before(s)
                && *s <= self.ran_through
                && first.is_none_or(|at| at >= Position::End { tick: *s })
                && self.stale.is_none_or(|t| *s < t)
        };
        let theirs = match_states.keys().rev().copied().find(usable);
        let from = own.max(theirs).unwrap_or(0);
        let here = self.position();
        if here > target || here < (Position::End { tick: from }) {
            match (own, theirs) {
                (Some(s), _) if s == from => {
                    let state = &self.saved[&s];
                    self.applied = state.entries;
                    self.sim = state.sim();
                }
                (_, Some(s)) if s == from => {
                    self.applied = 0;
                    self.sim = match_states[&s].sim();
                }
                _ => {
                    self.applied = 0;
                    self.sim = self.sim.fresh();
                }
            }
            self.stage = Stage::Rest;
            self.base = None;
            let tick = self.sim.world.tick;
            self.settle(Position::End { tick });
        }
        if self.position() != target {
            let mut ctl = self.ctl(Kind::To(target), None, None, 0);
            self.drive(&mut ctl, timeline);
        }
    }
}

impl Fork {
    fn stop(&mut self, arrival: Arrival, ctl: Ctl) -> Option<Stop> {
        match arrival {
            Arrival::Reached => None,
            Arrival::Stop(cause, machine) => {
                Some(self.record(cause, machine, ctl.hits.into_values().collect()))
            }
        }
    }

    /// Take a step (`docs/06`, Steps) and report where it stopped.
    /// `selected` is the machine the debugger follows, `watches` the
    /// team's, and `limit` the ticks a step runs with no other stop.
    /// *Line* and *over* are refused without a selected machine of the
    /// fork's team that runs a program.
    pub fn step(
        &mut self,
        step: Step,
        selected: Option<EntityId>,
        watches: &[Watch],
        limit: u64,
        timeline: &dyn Timeline,
    ) -> Result<Stop, String> {
        let kind = match step {
            Step::Line => Kind::Line,
            Step::Over => Kind::Over,
            Step::Tick => Kind::Tick,
            Step::Continue => Kind::Continue,
        };
        if matches!(kind, Kind::Line | Kind::Over) && self.own(selected).is_none() {
            return Err("select one of your machines that runs a program to step it".into());
        }
        let mut ctl = self.ctl(kind, selected, Some(watches), limit.max(1));
        let arrival = self.drive(&mut ctl, timeline);
        self.stop(arrival, ctl)
            .ok_or_else(|| "the step did not stop".to_string())
    }

    /// One tick of *run*: on to the tick's end, or to the stop *continue*
    /// would make before it. `None` while it is still running.
    pub fn run(
        &mut self,
        selected: Option<EntityId>,
        watches: &[Watch],
        timeline: &dyn Timeline,
    ) -> Option<Stop> {
        let mut ctl = self.ctl(Kind::Run, selected, Some(watches), 0);
        let arrival = self.drive(&mut ctl, timeline);
        self.stop(arrival, ctl)
    }

    /// Pause a *run*: finish the tick in progress, if one is, and stop at
    /// its end.
    pub fn pause(&mut self, timeline: &dyn Timeline) -> Stop {
        let tick = self.sim.world.tick;
        self.go_to(Position::End { tick }, &BTreeMap::new(), timeline);
        let cause = match self.sim.ended {
            Some(_) => Cause::MatchEnded,
            None => Cause::Paused,
        };
        self.record(cause, None, Vec::new())
    }

    /// Run one whole tick as `sim.step` does and report as it does, hits
    /// included: how a replay's playback moves. It records no stop. A
    /// fork that cannot run a tick, or that stands inside one, reports
    /// nothing.
    pub fn advance(&mut self, watches: &[Watch], timeline: &dyn Timeline) -> StepReport {
        if !matches!(self.stage, Stage::Rest) {
            return StepReport::default();
        }
        let mut ctl = self.ctl(Kind::Advance, None, Some(watches), 0);
        self.drive(&mut ctl, timeline);
        ctl.closed.unwrap_or_default()
    }

    /// Stop at the very next position and resume from it, whichever
    /// machine's it is. It records no stop: this is the determinism
    /// gate's walk through every boundary of a tick.
    pub fn nudge(&mut self, timeline: &dyn Timeline) -> Position {
        let mut ctl = self.ctl(Kind::Nudge, None, None, 0);
        self.drive(&mut ctl, timeline);
        self.position()
    }

    /// Stand at the end of `tick`, from the fork's own saved states or by
    /// running on from where it stands, without watches: a playback's
    /// seek.
    pub fn seek(&mut self, tick: u64, timeline: &dyn Timeline) {
        self.go_to(Position::End { tick }, &BTreeMap::new(), timeline);
    }

    /// Save the fork's state at the tick's end it stands at, and name the
    /// tick.
    pub fn keep(&mut self) -> Result<u64, String> {
        let state = self
            .save()
            .ok_or("a fork saves its state at a tick's end")?;
        let tick = state.tick();
        self.saved.insert(tick, state);
        Ok(tick)
    }

    /// The fork's state at the tick's end it stands at, for another fork
    /// to open from: a playback stands for the match. `None` mid-tick.
    pub fn save(&self) -> Option<Saved> {
        matches!(self.stage, Stage::Rest).then(|| Saved {
            world: self.sim.world.copy(),
            ended: self.sim.ended,
            entries: self.applied,
        })
    }

    /// Drop a saved state.
    pub fn forget(&mut self, tick: u64) {
        self.saved.remove(&tick);
    }

    /// The ticks the fork holds a saved state of.
    pub fn kept(&self) -> Vec<u64> {
        self.saved.keys().copied().collect()
    }

    /// The fork's saved states, to read: a playback stands for the match,
    /// so the debugger's fork of one opens from these and goes *back* on
    /// them as it would on the match's.
    pub fn states(&self) -> &BTreeMap<u64, Saved> {
        &self.saved
    }

    /// From here on, save the fork's state at the end of every tick that
    /// is a multiple of `interval` as the fork runs through it, whatever
    /// the step, and keep the last `count` states it holds (`docs/07`,
    /// Saved states: a fork saves its own on the match's terms). A fork
    /// never told this saves only when [`Fork::keep`] is called.
    pub fn keep_every(&mut self, interval: u64, count: usize) {
        self.every = Some((interval.max(1), count.max(1)));
    }

    /// The fork stands at the end of a tick it just ran: save it if
    /// [`Fork::keep_every`] asks for this tick.
    fn keep_due(&mut self) {
        let Some((interval, count)) = self.every else {
            return;
        };
        let tick = self.sim.world.tick;
        if !tick.is_multiple_of(interval) {
            return;
        }
        self.saved.entry(tick).or_insert_with(|| Saved {
            world: self.sim.world.copy(),
            ended: self.sim.ended,
            entries: self.applied,
        });
        while self.saved.len() > count {
            self.saved.pop_first();
        }
    }

    /// Return to the previous stop, from the latest usable saved state
    /// before it — the fork's own, or one of `match_states` — or from the
    /// map. Not offered at a first stop.
    pub fn back(
        &mut self,
        match_states: &BTreeMap<u64, Saved>,
        timeline: &dyn Timeline,
    ) -> Result<Stop, String> {
        let [.., previous, _] = self.stops.as_slice() else {
            return Err("there is no earlier stop".into());
        };
        let previous = previous.clone();
        self.stops.pop();
        self.go_to(previous.position, match_states, timeline);
        Ok(previous)
    }

    /// What the fork reports of one of its team's machines where it
    /// stands (`docs/06`, Stopping). Values are cut at `value_chars`.
    pub fn inspect(&self, machine: EntityId, value_chars: usize) -> Option<Inspected> {
        let m = self.sim.world.machines.get(&machine)?;
        (m.team == self.team).then(|| Inspected {
            record: MachineRecord::of(m),
            program: m.program.as_ref().map(|p| p.inspect(value_chars)),
        })
    }

    /// Where an entry of the fork's log would go, if one may be made.
    fn branch_point(&self) -> Result<Position, String> {
        if self.live_pvp {
            return Err("a fork of a live PvP match takes no edit and no what-if".into());
        }
        if matches!(self.stage, Stage::Damage { .. }) {
            return Err("nothing is changed at a stop in step 4".into());
        }
        Ok(self.position())
    }

    /// Put an entry in the log at `here`: a new branch, so later entries
    /// and the saved states after `here` go.
    fn branch(&mut self, here: Position, what: What) {
        self.log.retain(|e| e.at <= here);
        self.saved.retain(|s, _| Position::End { tick: *s } <= here);
        self.log.push(Entry { at: here, what });
        self.applied = self.log.len();
    }

    /// Whether a variable of the machine may be edited where the fork
    /// stands (`docs/06`, Editing), or the reason it may not: what
    /// [`Fork::edit`] refuses before it reads the value.
    pub fn can_edit(&self, machine: EntityId) -> Result<(), String> {
        self.branch_point()?;
        let m = self.sim.world.machines.get(&machine);
        let m = m
            .filter(|m| m.team == self.team)
            .ok_or("no such machine of this team")?;
        let program = m.program.as_ref().ok_or("the machine runs no program")?;
        if program.is_dead() || program.delivering() == Some(Interrupt::Death) {
            return Err("the machine is at its death".into());
        }
        Ok(())
    }

    /// Set a local or a global of one of the team's machines (`docs/06`,
    /// Editing). A refusal names its reason and changes nothing.
    pub fn edit(
        &mut self,
        machine: EntityId,
        target: EditTarget,
        literal: &str,
    ) -> Result<(), String> {
        self.can_edit(machine)?;
        let here = self.branch_point()?;
        let m = self.sim.world.machines.get_mut(&machine);
        let program = m.and_then(|m| m.program.as_mut());
        let program = program.ok_or("the machine runs no program")?;
        program.edit(&target, literal)?;
        self.branch(
            here,
            What::Edit {
                machine,
                target,
                literal: literal.to_string(),
            },
        );
        Ok(())
    }

    /// Put a what-if deploy in the log for the fork's next tick, checked
    /// as a submission is (`docs/06`, What-if).
    pub fn what_if(&mut self, deployment: &str, bundle: &Bundle) -> Result<(), String> {
        let here = self.branch_point()?;
        let what = What::Deploy {
            deployment: deployment.to_string(),
            bundle: bundle.clone(),
        };
        let next = here.tick().saturating_add(1);
        let kind = CommandKind::Deploy {
            deployment: deployment.to_string(),
            bundle: bundle.clone(),
        };
        self.sim.validate(&Command::new(next, self.team, 0, kind))?;
        self.branch(here, what);
        Ok(())
    }

    /// The fork's state against the match's log as it stands now
    /// (`docs/06`, Its state).
    pub fn flags(&mut self, timeline: &dyn Timeline) -> Flags {
        let final_tick = timeline.final_tick();
        let unfinal: Vec<u64> = self.unfinal.iter().copied().collect();
        for tick in unfinal {
            let now = settled(timeline.commands(tick));
            let then = settled(self.taken.get(&tick).cloned().unwrap_or_default());
            if now != then {
                self.stale = Some(self.stale.map_or(tick, |s| s.min(tick)));
                self.unfinal.remove(&tick);
            } else if tick <= final_tick {
                self.unfinal.remove(&tick);
            }
        }
        Flags {
            ahead: self.sim.world.tick > final_tick,
            stale: self.stale,
            diverged: !self.log.is_empty(),
        }
    }

    /// A snapshot at the position the fork stands at (`docs/06`, Drawing
    /// a fork): the team's own machines as they stand, and everything
    /// else as at the end of the last tick the fork completed.
    pub fn snapshot(&self) -> Snapshot {
        let base = match (&self.stage, &self.base) {
            (Stage::Rest, _) | (_, None) => return self.sim.snapshot(),
            (_, Some(base)) => base,
        };
        let mut snapshot = base.clone();
        snapshot.machines.retain(|m| m.record.team != self.team);
        let own = self.sim.world.machines.values();
        for m in own.filter(|m| m.team == self.team) {
            snapshot.machines.push(MachineSnapshot::of(m));
        }
        snapshot.machines.sort_by_key(|m| m.record.id);
        snapshot
    }
}
