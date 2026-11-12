//! The debugger's half of the driver (`docs/07`, Debugging; `docs/06`,
//! Forks): the team's watches, the states saved for forks, a tick's hits,
//! and the one open fork with its steps. Plain Rust — no Bevy type — so it
//! is tested headless; the windows that drive it are the interface's.
//!
//! Nothing here changes the match. A watch is read-only and no part of the
//! hash, `Sim::save` reads, and a fork is a private copy that never reaches
//! the match's world, log or hash (`CLAUDE.md`, rule 1). The one thing the
//! debugger sends is the pause a hit asks for, and it sends it as the
//! player would: a `SetSpeed(0)` through [`Driver::submit`].

use crate::driver::{Driver, Engine, Snapshots};
use crate::palette::Debugging;
use lang::EditTarget;
use net::CommandLog;
use replay::ReplayLog;
use sim::{
    Command, CommandKind, Data, EntityId, Flags, Fork, Hit, Inspected, Map, Saved, Sim, Step, Stop,
    TeamId, Timeline, Watch,
};
use std::collections::BTreeMap;

/// The most ticks one frame of a fork's *run* runs, as the match's loop
/// bounds its own.
const RUN_TICKS_A_FRAME: u32 = 64;

/// The log a fork reads: the match's as it stands this frame, or a
/// replay's, which is whole.
enum Line<'a> {
    Match { log: &'a CommandLog, tick: u64 },
    Replay(&'a ReplayLog),
}

impl Timeline for Line<'_> {
    fn commands(&self, tick: u64) -> Vec<Command> {
        match self {
            Line::Match { log, .. } => {
                // Tick 0 is never stepped: its set applies with tick 1.
                let mut commands = if tick == 1 {
                    log.sets_for(0)
                } else {
                    Vec::new()
                };
                commands.extend(log.sets_for(tick));
                commands
            }
            Line::Replay(log) => log.commands(tick),
        }
    }

    fn final_tick(&self) -> u64 {
        match self {
            Line::Match { tick, .. } => *tick,
            Line::Replay(log) => log.final_tick(),
        }
    }
}

fn line_of<'a>(engine: &'a Engine, log: &'a CommandLog, tick: u64) -> Line<'a> {
    match engine {
        Engine::Match(_) => Line::Match { log, tick },
        Engine::Replay(viewer) => Line::Replay(viewer.log()),
    }
}

/// The saved states a fork opens from and goes *back* on: the match's,
/// which the debugger holds, or the playback's, which stands for it.
fn states_of<'a>(engine: &'a Engine, states: &'a BTreeMap<u64, Saved>) -> &'a BTreeMap<u64, Saved> {
    match engine {
        Engine::Match(_) => states,
        Engine::Replay(viewer) => viewer.fork().states(),
    }
}

/// A further hit, listed while a fork is open.
pub struct Listed {
    pub hit: Hit,
    /// The saved state the hit's fork would open from, kept while the hit
    /// is listed.
    from: u64,
}

/// The open fork and what the debugger window holds of it.
pub struct ForkSession {
    pub fork: Fork,
    /// The saved state it was opened from, kept while it is open.
    from: u64,
    /// The machine the debugger follows: *line* and *over* step it.
    pub selected: Option<EntityId>,
    /// *Run* is going.
    pub running: bool,
    /// Ticks per real second of *run*; `0` holds it.
    pub run_speed: u64,
    accumulated: f64,
    /// The fork's snapshots, for the map to draw in the match's place.
    pub snapshots: Snapshots,
    /// Faithful, or ahead, stale, diverged: against the log as it stood
    /// on the last frame.
    pub flags: Flags,
}

impl ForkSession {
    /// The stop the fork stands at, with its cause.
    pub fn stop(&self) -> Option<&Stop> {
        self.fork.stops().last()
    }

    /// *Back* is not offered at a fork's first stop, nor while it runs.
    pub fn can_back(&self) -> bool {
        !self.running && self.fork.stops().len() > 1
    }

    /// The fork stands somewhere new: a stop on a machine selects it, and
    /// the map draws from here. A tick of *run* is a step to interpolate;
    /// anything else is a jump.
    fn arrived(&mut self, stop: Option<&Stop>, ran: bool, line: &Line) {
        if let Some(machine) = stop.and_then(|s| s.machine) {
            self.selected = Some(machine);
        }
        let cur = self.fork.snapshot();
        let prev = std::mem::replace(&mut self.snapshots.cur, cur);
        self.snapshots.prev = ran.then_some(prev);
        self.snapshots.since = 0.0;
        self.flags = self.fork.flags(line);
    }
}

/// The debugger's state beside the driver's (`docs/07`, Debugging).
pub struct Debugger {
    /// `data/interface.toml`'s `[debugging]`.
    pub tuning: Debugging,
    watches: Vec<Watch>,
    /// The match's saved states. A replay's are its playback's.
    states: BTreeMap<u64, Saved>,
    /// The match's states an open fork or a listed hit needs, kept beyond
    /// the count, each with how many need it.
    pinned: BTreeMap<u64, u32>,
    /// The open fork: one at a time.
    pub session: Option<ForkSession>,
    /// Further hits while the fork is open, the latest position first.
    pub hits: Vec<Listed>,
    /// *Show the match*: the map draws the match though a fork is open.
    pub show_match: bool,
}

impl Debugger {
    pub fn new(tuning: Debugging) -> Debugger {
        Debugger {
            tuning,
            watches: Vec::new(),
            states: BTreeMap::new(),
            pinned: BTreeMap::new(),
            session: None,
            hits: Vec::new(),
            show_match: false,
        }
    }

    /// The team's watches, passed to every tick the match or the playback
    /// runs.
    pub fn watches(&self) -> &[Watch] {
        &self.watches
    }

    /// Whether a breakpoint is set or a fork is open: while it is, the
    /// match's states are saved.
    pub fn debugging(&self) -> bool {
        !self.watches.is_empty() || self.session.is_some()
    }

    /// The ticks a state of the match is saved of.
    pub fn kept(&self) -> Vec<u64> {
        self.states.keys().copied().collect()
    }

    /// Hold the match's state at `tick`, which `save` makes if it is not
    /// held already, and drop what the count no longer keeps.
    fn keep(&mut self, tick: u64, save: impl FnOnce() -> Saved) {
        self.states.entry(tick).or_insert_with(save);
        self.trim();
    }

    /// Keep the last `debug_checkpoints_kept` states, and beyond them the
    /// ones a fork or a listed hit needs.
    fn trim(&mut self) {
        let count = self.tuning.debug_checkpoints_kept.max(1);
        let drop: Vec<u64> = self
            .states
            .keys()
            .rev()
            .skip(count)
            .filter(|t| !self.pinned.contains_key(t))
            .copied()
            .collect();
        for tick in drop {
            self.states.remove(&tick);
        }
    }

    fn pin(&mut self, tick: u64) {
        let n = self.pinned.entry(tick).or_insert(0);
        *n += 1;
    }

    fn unpin(&mut self, tick: u64) {
        let Some(n) = self.pinned.get_mut(&tick) else {
            return;
        };
        *n = n.saturating_sub(1);
        if *n == 0 {
            self.pinned.remove(&tick);
            self.trim();
        }
    }

    /// No breakpoint is set and no fork is open: the match's states go.
    fn rest(&mut self) {
        if !self.debugging() {
            self.states.clear();
            self.pinned.clear();
        }
    }
}

impl Driver {
    /// The team the debugger is: the player's, or the one a replay
    /// records. A replay that records none is watched, not debugged (Q55).
    pub fn debug_team(&self) -> Result<TeamId, String> {
        match &self.engine {
            Engine::Match(_) => Ok(self.player),
            Engine::Replay(v) if v.can_debug() => Ok(v.team()),
            Engine::Replay(_) => {
                Err("this replay records no team: it can be watched, not debugged".into())
            }
        }
    }

    /// A live PvP match bounds a fork to its latest completed tick and
    /// lets it take no edit (`docs/07`, PvP). A match that is over is no
    /// longer live, and a replay never is.
    pub fn live_pvp(&self) -> bool {
        matches!(self.engine, Engine::Match(_)) && self.is_networked() && self.ended.is_none()
    }

    /// Set the team's watches: every breakpoint the interface holds, as
    /// `docs/06`'s watches. From the first one on, the match's state is
    /// saved, at once and then at every checkpoint.
    pub fn set_watches(&mut self, watches: Vec<Watch>) -> Result<(), String> {
        if !watches.is_empty() {
            let team = self.debug_team()?;
            if watches.iter().any(|w| w.team != team) {
                return Err("a breakpoint watches the team's own machines".into());
            }
        }
        if watches == self.debug.watches {
            return Ok(());
        }
        let was = self.debug.debugging();
        self.debug.watches = watches;
        if !was {
            self.save_at_once();
        }
        self.debug.rest();
        Ok(())
    }

    /// Debugging begins: the match's state at its latest completed tick is
    /// held, so a hit always has one to start from. A playback saves its
    /// own, whether or not it is debugged.
    fn save_at_once(&mut self) {
        if !self.debug.debugging() {
            return;
        }
        if let Engine::Match(sim) = &self.engine {
            self.debug.keep(self.tick, || sim.save());
        }
    }

    /// A tick of the match or of the playback just ran: take its hits,
    /// then save the match's state if a checkpoint is due.
    pub(crate) fn after_tick(&mut self, hits: Vec<Hit>) {
        if !self.debug.debugging() {
            return;
        }
        self.take_hits(hits);
        let every = self.debug.tuning.debug_checkpoint_ticks.max(1);
        if self.tick.is_multiple_of(every)
            && let Engine::Match(sim) = &self.engine
        {
            self.debug.keep(self.tick, || sim.save());
        }
    }

    /// A hit opens a fork at the earliest position among the tick's hits,
    /// which the tick's report holds in position order, and pauses what
    /// the player watches; the tick's other hits, and every hit while a
    /// fork is open, are listed.
    fn take_hits(&mut self, hits: Vec<Hit>) {
        let mut hits = hits.into_iter();
        if self.debug.session.is_none()
            && let Some(first) = hits.next()
        {
            match self.fork_at(&first) {
                Ok(session) => {
                    self.install(session);
                    self.pause_for_hit();
                }
                Err(e) => {
                    // No fork is open to list the tick's others under.
                    self.events.push(format!(
                        "no fork at the breakpoint hit in tick {}: {e}",
                        first.position.tick()
                    ));
                    return;
                }
            }
        }
        for hit in hits {
            self.list(hit);
        }
    }

    /// The latest saved state at or before `tick`, kept from now until
    /// [`Driver::unpin`]. A match with none re-runs from the start: the
    /// map's own state, before tick 1.
    fn pin_before(&mut self, tick: u64) -> Result<u64, String> {
        match &mut self.engine {
            Engine::Replay(viewer) => Ok(viewer.pin_before(tick)),
            Engine::Match(_) => {
                let held = self.debug.states.range(..=tick).next_back();
                let from = match held.map(|(t, _)| *t) {
                    Some(t) => t,
                    None => {
                        let data = Data::load().map_err(|e| e.to_string())?;
                        let map = Map::parse(&self.map_text, &data).map_err(|e| e.to_string())?;
                        self.debug.states.insert(0, Sim::new(map).save());
                        0
                    }
                };
                self.debug.pin(from);
                Ok(from)
            }
        }
    }

    fn unpin(&mut self, tick: u64) {
        match &mut self.engine {
            Engine::Replay(viewer) => viewer.unpin(tick),
            Engine::Match(_) => self.debug.unpin(tick),
        }
    }

    /// A fork at a hit: from the latest saved state before the hit's
    /// tick, run to the hit's position.
    fn fork_at(&mut self, hit: &Hit) -> Result<ForkSession, String> {
        let team = self.debug_team()?;
        let live = self.live_pvp();
        let from = self.pin_before(hit.position.tick().saturating_sub(1))?;
        let line = line_of(&self.engine, &self.log, self.tick);
        let made = match states_of(&self.engine, &self.debug.states).get(&from) {
            Some(saved) => Fork::open_at(saved, hit, team, live, &line),
            None => Err(format!("the state saved at tick {from} is gone")),
        };
        match made {
            Ok(fork) => Ok(self.session_of(fork, from)),
            Err(e) => {
                self.unpin(from);
                Err(e)
            }
        }
    }

    /// The debugger's hold on a fork just opened. A fork saves its own
    /// states on the match's terms: at once, if it stands at a tick's end,
    /// and at every checkpoint it runs through.
    fn session_of(&self, mut fork: Fork, from: u64) -> ForkSession {
        let tuning = self.debug.tuning;
        fork.keep_every(tuning.debug_checkpoint_ticks, tuning.debug_checkpoints_kept);
        // Refused mid-tick, where a hit's fork stands: it has none yet.
        let _ = fork.keep();
        let line = line_of(&self.engine, &self.log, self.tick);
        let flags = fork.flags(&line);
        let last = self.debug.session.as_ref();
        ForkSession {
            from,
            selected: fork.stops().last().and_then(|s| s.machine),
            running: false,
            run_speed: last.map_or(self.start_speed, |s| s.run_speed),
            accumulated: 0.0,
            snapshots: Snapshots {
                prev: None,
                cur: fork.snapshot(),
                since: 0.0,
            },
            flags,
            fork,
        }
    }

    /// Open `session`'s fork in the place of the one open, if one is:
    /// there is one fork at a time.
    fn install(&mut self, session: ForkSession) {
        if let Some(old) = self.debug.session.replace(session) {
            self.unpin(old.from);
        }
        self.debug.show_match = false;
    }

    /// A hit opened a fork: pause what the player is watching (`docs/07`,
    /// A hit). A replay's playback pauses at once; a single-player match
    /// is sent a `SetSpeed(0)` unless the speed it is on its way to is
    /// already `0`; a PvP match runs on.
    fn pause_for_hit(&mut self) {
        if self.is_replay() {
            // With no time left to spend, the playback's frame ends at
            // this tick.
            self.speed = 0;
            self.accumulated = 0.0;
            return;
        }
        if self.is_networked() || self.ended.is_some() {
            return;
        }
        // The speed once every `SetSpeed` the player has sent lands: the
        // last one's, whether it has landed or not, since only the
        // player's set a single-player match's speed.
        let last = self
            .submitted
            .iter()
            .filter_map(|c| match c.kind {
                CommandKind::SetSpeed(s) => Some(s),
                _ => None,
            })
            .next_back();
        if last.unwrap_or(self.speed) == 0 {
            return;
        }
        if let Err(e) = self.submit(CommandKind::SetSpeed(0)) {
            self.events
                .push(format!("the breakpoint's pause was refused: {e}"));
        }
    }

    /// A hit while a fork is open: listed by position, the latest first,
    /// the last `debug_hits_kept` of them, each keeping the state its fork
    /// would open from.
    fn list(&mut self, hit: Hit) {
        if self.debug.hits.iter().any(|l| l.hit == hit) {
            // A playback sought back plays through the same hit again.
            return;
        }
        let from = match self.pin_before(hit.position.tick().saturating_sub(1)) {
            Ok(from) => from,
            Err(e) => {
                self.events.push(format!(
                    "the breakpoint hit in tick {} is not listed: {e}",
                    hit.position.tick()
                ));
                return;
            }
        };
        self.debug.hits.push(Listed { hit, from });
        self.debug
            .hits
            .sort_by_key(|l| std::cmp::Reverse(l.hit.position));
        let kept = self.debug.tuning.debug_hits_kept.max(1);
        while self.debug.hits.len() > kept {
            if let Some(dropped) = self.debug.hits.pop() {
                self.unpin(dropped.from);
            }
        }
    }

    /// Open a fork by hand: at the match's latest completed tick, or at
    /// the tick the playback stands at, from a state saved there as it
    /// opens, stopped as *opened*. It replaces the fork that is open.
    pub fn open_fork(&mut self) -> Result<(), String> {
        let team = self.debug_team()?;
        let live = self.live_pvp();
        let from = match &mut self.engine {
            Engine::Match(sim) => {
                self.debug.keep(self.tick, || sim.save());
                self.debug.pin(self.tick);
                self.tick
            }
            Engine::Replay(viewer) => viewer.keep_now()?,
        };
        let fork = match states_of(&self.engine, &self.debug.states).get(&from) {
            Some(saved) => Fork::open(saved, team, live),
            None => {
                self.unpin(from);
                return Err(format!("the state saved at tick {from} is gone"));
            }
        };
        let session = self.session_of(fork, from);
        self.install(session);
        Ok(())
    }

    /// Reopen the fork at a listed hit, replacing the open fork, its log
    /// and its own saved states. The hit stays listed.
    pub fn reopen(&mut self, index: usize) -> Result<(), String> {
        let hit = match self.debug.hits.get(index) {
            Some(listed) => listed.hit.clone(),
            None => return Err("no such hit is listed".into()),
        };
        let session = self.fork_at(&hit)?;
        self.install(session);
        Ok(())
    }

    /// Close the fork: the map and the windows return to the match, the
    /// listed hits go with it, and so do the states they kept.
    pub fn close_fork(&mut self) {
        if let Some(old) = self.debug.session.take() {
            self.unpin(old.from);
        }
        for listed in std::mem::take(&mut self.debug.hits) {
            self.unpin(listed.from);
        }
        self.debug.show_match = false;
        self.debug.rest();
    }

    /// The snapshots the map draws: the fork's while one is open, unless
    /// *show the match* is on.
    pub fn shown(&self) -> &Snapshots {
        match &self.debug.session {
            Some(session) if !self.debug.show_match => &session.snapshots,
            _ => &self.snapshots,
        }
    }

    /// Whether the map draws a fork, so the mark tools rest.
    pub fn draws_fork(&self) -> bool {
        self.debug.session.is_some() && !self.debug.show_match
    }

    /// The ticks per second what the map draws moves at, for
    /// interpolation: the fork's *run*, or the match's speed.
    pub fn shown_speed(&self) -> u64 {
        match &self.debug.session {
            Some(s) if !self.debug.show_match => {
                if s.running {
                    s.run_speed
                } else {
                    0
                }
            }
            _ => self.speed,
        }
    }

    /// *Line*, *over*, *tick* or *continue* (`docs/06`, Steps), with the
    /// tick limit `debug_step_max_ticks`.
    pub fn fork_step(&mut self, step: Step) -> Result<Stop, String> {
        let limit = self.debug.tuning.debug_step_max_ticks;
        let line = line_of(&self.engine, &self.log, self.tick);
        let Debugger {
            session, watches, ..
        } = &mut self.debug;
        let s = session.as_mut().ok_or("no fork is open")?;
        if s.running {
            return Err("pause the run to step".into());
        }
        let stop = s.fork.step(step, s.selected, watches, limit, &line)?;
        s.arrived(Some(&stop), false, &line);
        Ok(stop)
    }

    /// Start *run*: [`Driver::advance`] then runs the fork at the
    /// debugger window's speed until it stops or is paused.
    pub fn fork_run(&mut self) -> Result<(), String> {
        let s = self.debug.session.as_mut().ok_or("no fork is open")?;
        s.running = true;
        s.accumulated = 0.0;
        Ok(())
    }

    /// Set the speed of *run*, in ticks per real second.
    pub fn set_run_speed(&mut self, speed: u64) -> Result<(), String> {
        let s = self.debug.session.as_mut().ok_or("no fork is open")?;
        s.run_speed = speed;
        s.accumulated = 0.0;
        Ok(())
    }

    /// Pause a *run*: the fork finishes the tick it is in and stops at its
    /// end.
    pub fn fork_pause(&mut self) -> Result<Stop, String> {
        let line = line_of(&self.engine, &self.log, self.tick);
        let s = self.debug.session.as_mut().ok_or("no fork is open")?;
        if !s.running {
            return Err("the fork is not running".into());
        }
        s.running = false;
        s.accumulated = 0.0;
        let stop = s.fork.pause(&line);
        s.arrived(Some(&stop), false, &line);
        Ok(stop)
    }

    /// One frame of the fork: a *run* that is going runs its ticks, and
    /// the fork's state is read against the log as it now stands, since
    /// the match's later commands can make it stale.
    pub(crate) fn run_fork(&mut self, dt: f64) {
        let line = line_of(&self.engine, &self.log, self.tick);
        let Debugger {
            session, watches, ..
        } = &mut self.debug;
        let Some(s) = session else {
            return;
        };
        s.snapshots.since += dt;
        if s.running && s.run_speed > 0 {
            s.accumulated += dt;
            let period = 1.0 / s.run_speed as f64;
            let mut ran = 0u32;
            while s.accumulated >= period && ran < RUN_TICKS_A_FRAME {
                s.accumulated -= period;
                ran += 1;
                let stop = s.fork.run(s.selected, watches, &line);
                s.arrived(stop.as_ref(), true, &line);
                if stop.is_some() {
                    s.running = false;
                    s.accumulated = 0.0;
                }
            }
        }
        s.flags = s.fork.flags(&line);
    }

    /// *Back*: return to the previous stop, from the fork's saved states,
    /// the match's or the playback's.
    pub fn fork_back(&mut self) -> Result<Stop, String> {
        let line = line_of(&self.engine, &self.log, self.tick);
        let Debugger {
            session, states, ..
        } = &mut self.debug;
        let s = session.as_mut().ok_or("no fork is open")?;
        if s.running {
            return Err("pause the run to go back".into());
        }
        let stop = s.fork.back(states_of(&self.engine, states), &line)?;
        s.arrived(Some(&stop), false, &line);
        Ok(stop)
    }

    /// Select a machine of the fork for the debugger to follow, or none.
    /// Only the team's machines can be selected.
    pub fn fork_select(&mut self, machine: Option<EntityId>) -> Result<(), String> {
        let s = self.debug.session.as_mut().ok_or("no fork is open")?;
        if let Some(id) = machine
            && s.fork.inspect(id, 0).is_none()
        {
            return Err("only your own machines can be selected in a fork".into());
        }
        s.selected = machine;
        Ok(())
    }

    /// The selected machine as the fork holds it at the stop, values cut
    /// at `value_chars`: from the fork, not from a snapshot.
    pub fn inspect(&self, value_chars: usize) -> Option<Inspected> {
        let s = self.debug.session.as_ref()?;
        s.fork.inspect(s.selected?, value_chars)
    }

    /// Set a local or a global of the selected machine in the fork, which
    /// is diverged from then on. The fork refuses what `docs/06` does: a
    /// stop in step 4, a machine at its death, a live PvP match.
    pub fn fork_edit(&mut self, target: EditTarget, literal: &str) -> Result<(), String> {
        let line = line_of(&self.engine, &self.log, self.tick);
        let s = self.debug.session.as_mut().ok_or("no fork is open")?;
        if s.running {
            return Err("pause the run to edit".into());
        }
        let machine = s.selected.ok_or("select a machine to edit")?;
        s.fork.edit(machine, target, literal)?;
        s.arrived(None, false, &line);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{Cause, Position, bundle_of};

    /// Every machine that runs it begins line 3 once a tick.
    const COUNTER: &str = "x = 0\nwhile True:\n    x = x + 1\n    wait(1)\n";
    const PRINTER: EntityId = EntityId(1);
    const BOT: EntityId = EntityId(3);

    fn map_text() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/maps/first.toml"
        ))
        .unwrap()
    }

    /// The team's printer and its bots all run `COUNTER`.
    fn opening() -> Vec<CommandKind> {
        ["printer", "bot"]
            .into_iter()
            .map(|d| CommandKind::Deploy {
                deployment: d.into(),
                bundle: bundle_of(COUNTER),
            })
            .collect()
    }

    fn tuning() -> Debugging {
        Debugging {
            debug_checkpoint_ticks: 5,
            debug_checkpoints_kept: 3,
            debug_step_max_ticks: 100,
            debug_hits_kept: 3,
        }
    }

    /// A single-player match against the Fool, as the build opens it.
    fn shipped() -> Driver {
        let fool = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/opposition/fool"
        ));
        Driver::new(&map_text(), &[&fool], opening()).unwrap()
    }

    /// The same match on the tests' tuning.
    fn driver() -> Driver {
        let mut d = shipped();
        d.debug.tuning = tuning();
        d
    }

    #[test]
    fn a_match_is_debugged_on_the_tuning_the_interface_ships() {
        let shipped = shipped().debug.tuning;
        assert_eq!(
            shipped,
            crate::palette::Interface::load().unwrap().debugging
        );
        assert_ne!(shipped, tuning(), "or the tests' tuning shows nothing");
    }

    fn watch(line: u32) -> Watch {
        Watch {
            team: TeamId(0),
            file: "main.py".into(),
            content: lang::content_hash(COUNTER.as_bytes()),
            line,
            deployment: None,
            machine: None,
        }
    }

    fn run_to(d: &mut Driver, tick: u64) {
        for _ in 0..10_000 {
            if d.tick >= tick {
                return;
            }
            d.advance(0.1);
        }
        panic!("the driver stopped at tick {} short of {tick}", d.tick);
    }

    /// A watch that hits nothing once the printer is past its first line.
    fn quiet() -> Watch {
        Watch {
            machine: Some(PRINTER),
            ..watch(1)
        }
    }

    fn frames(d: &mut Driver, n: usize) {
        for _ in 0..n {
            d.advance(0.1);
        }
    }

    fn session(d: &Driver) -> &ForkSession {
        d.debug.session.as_ref().expect("an open fork")
    }

    /// The `SetSpeed`s the player's side submitted, as (tick, speed).
    fn set_speeds(d: &Driver) -> Vec<(u64, u64)> {
        d.submitted
            .iter()
            .filter_map(|c| match c.kind {
                CommandKind::SetSpeed(s) => Some((c.tick, s)),
                _ => None,
            })
            .collect()
    }

    /// `x` of the selected machine's `main`, as the fork holds it.
    fn x(d: &Driver) -> Option<String> {
        let program = d.inspect(80)?.program?;
        let main = program.modules.into_iter().find(|m| m.name == "main")?;
        main.globals.into_iter().find(|g| g.0 == "x").map(|g| g.1)
    }

    fn x_of_main() -> EditTarget {
        EditTarget::Global {
            module: "main".into(),
            name: "x".into(),
        }
    }

    fn listed(d: &Driver) -> Vec<(u64, EntityId)> {
        d.debug
            .hits
            .iter()
            .map(|l| (l.hit.position.tick(), l.hit.machine))
            .collect()
    }

    fn end(tick: u64) -> Position {
        Position::End { tick }
    }

    /// A match whose watch on the counter's line first hits in tick 39,
    /// the printer and a bot both beginning it.
    fn hit_at_39() -> Driver {
        let mut d = driver();
        run_to(&mut d, 38);
        d.set_watches(vec![watch(3)]).unwrap();
        d.advance(0.1);
        assert_eq!(d.tick, 39);
        d
    }

    #[test]
    fn while_a_breakpoint_is_set_the_match_is_saved_at_once_and_at_every_checkpoint() {
        let mut d = driver();
        run_to(&mut d, 12);
        assert!(!d.debug.debugging());
        assert_eq!(d.debug.kept(), Vec::<u64>::new(), "nothing is debugged");
        d.set_watches(vec![quiet()]).unwrap();
        assert_eq!(d.debug.kept(), vec![12], "at once, at the latest tick");
        run_to(&mut d, 14);
        assert_eq!(d.debug.kept(), vec![12], "no tick between checkpoints");
        run_to(&mut d, 31);
        assert_eq!(d.debug.kept(), vec![20, 25, 30], "the last three");
        assert!(d.debug.session.is_none(), "the quiet watch hit nothing");
        // The same list again starts nothing over.
        d.set_watches(vec![quiet()]).unwrap();
        assert_eq!(d.debug.kept(), vec![20, 25, 30]);
        d.set_watches(Vec::new()).unwrap();
        assert_eq!(d.debug.kept(), Vec::<u64>::new(), "no breakpoint, no fork");
        run_to(&mut d, 41);
        assert_eq!(d.debug.kept(), Vec::<u64>::new());
        assert!(set_speeds(&d).is_empty());
    }

    #[test]
    fn a_hit_opens_a_fork_at_the_ticks_earliest_hit_from_a_state_before_it() {
        let mut d = hit_at_39();
        let s = session(&d);
        assert_eq!(s.from, 38, "the state saved as the breakpoint was set");
        let stop = s.stop().unwrap().clone();
        assert_eq!(stop.cause, Cause::Watch);
        assert_eq!(stop.machine, Some(PRINTER), "the earliest position");
        assert_eq!(stop.position.tick(), 39);
        assert_eq!(s.fork.position(), stop.position);
        assert_eq!(s.selected, Some(PRINTER), "the window opens on it");
        assert!(s.flags.faithful());
        assert!(!s.can_back(), "a fork's first stop");
        // The tick's other hit is a further hit.
        assert_eq!(listed(&d), vec![(39, BOT)]);
        assert!(d.debug.hits[0].hit.position > stop.position);
        // The fork is the match's tick 39, stopped inside it.
        let stop = d.fork_step(Step::Tick).unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::TickEnd, end(39)));
        assert_eq!(session(&d).fork.state_hash(), Some(d.state_hash()));
    }

    #[test]
    fn a_hit_pauses_a_single_player_match_and_further_hits_are_listed_with_no_other_pause() {
        let mut d = hit_at_39();
        assert_eq!(set_speeds(&d), vec![(42, 0)], "delay ticks on");
        assert_eq!(d.speed, 10, "not yet landed");
        frames(&mut d, 10);
        assert_eq!((d.tick, d.speed), (42, 0));
        assert_eq!(
            set_speeds(&d),
            vec![(42, 0)],
            "one pause, however many hits"
        );
        // By position, the latest first, the last three.
        assert_eq!(listed(&d), vec![(42, BOT), (42, PRINTER), (41, BOT)]);
        assert!(d.debug.hits.iter().all(|l| l.hit.more == 0));
        // The fork is as the first hit left it.
        let s = session(&d);
        assert_eq!(s.fork.stops().len(), 1);
        assert_eq!(s.fork.position().tick(), 39);
        // Its state, and the one the listed hits need, are held.
        assert_eq!(d.debug.kept(), vec![38, 40]);
        assert!(d.events.is_empty(), "{:?}", d.events);
    }

    #[test]
    fn a_pause_already_in_flight_is_not_sent_again() {
        let mut d = driver();
        run_to(&mut d, 38);
        assert_eq!(d.submit(CommandKind::SetSpeed(0)).unwrap(), 41);
        d.set_watches(vec![watch(3)]).unwrap();
        frames(&mut d, 10);
        assert!(d.debug.session.is_some());
        assert_eq!(set_speeds(&d), vec![(41, 0)]);
        assert_eq!((d.tick, d.speed), (41, 0));
    }

    #[test]
    fn a_hit_in_a_checkpoint_tick_opens_from_the_state_before_it() {
        let mut d = driver();
        d.debug.tuning.debug_checkpoints_kept = 1;
        run_to(&mut d, 39);
        d.set_watches(vec![watch(3)]).unwrap();
        assert_eq!(d.debug.kept(), vec![39]);
        d.advance(0.1);
        assert_eq!(d.tick, 40);
        // Tick 40's own state is saved after its hits are taken, so the
        // count never drops the one the hit's fork needs.
        assert_eq!(session(&d).from, 39);
        assert_eq!(d.debug.kept(), vec![39, 40]);
    }

    #[test]
    fn the_pause_looks_at_the_last_speed_in_flight() {
        let mut d = driver();
        run_to(&mut d, 38);
        d.submit(CommandKind::SetSpeed(0)).unwrap();
        d.submit(CommandKind::SetSpeed(20)).unwrap();
        d.set_watches(vec![watch(3)]).unwrap();
        d.advance(0.1);
        assert_eq!(set_speeds(&d), vec![(41, 0), (41, 20), (42, 0)]);
        frames(&mut d, 10);
        assert_eq!(d.speed, 0);
    }

    /// A match paused at tick 23 with the counter's line watched.
    fn paused_at_23() -> Driver {
        let mut d = driver();
        run_to(&mut d, 20);
        d.submit(CommandKind::SetSpeed(0)).unwrap();
        frames(&mut d, 10);
        assert_eq!((d.tick, d.speed), (23, 0));
        d.set_watches(vec![watch(3)]).unwrap();
        d
    }

    #[test]
    fn a_pause_in_force_is_not_woken_by_a_redundant_one() {
        let mut d = paused_at_23();
        // A command the paused match runs to; its ticks hit.
        let agreed = d
            .submit(CommandKind::Deploy {
                deployment: "bot".into(),
                bundle: bundle_of(COUNTER),
            })
            .unwrap();
        assert_eq!(agreed, 26);
        frames(&mut d, 10);
        assert_eq!(session(&d).fork.position().tick(), 24, "the first hit");
        assert_eq!(set_speeds(&d), vec![(23, 0)], "no second pause");
        assert_eq!((d.tick, d.speed), (26, 0), "it ran to the command only");
    }

    #[test]
    fn a_resume_in_flight_is_overtaken_by_the_pause() {
        let mut d = paused_at_23();
        assert_eq!(d.submit(CommandKind::SetSpeed(20)).unwrap(), 26);
        d.advance(0.001);
        assert_eq!((d.tick, d.speed), (26, 20), "the resume landed");
        assert_eq!(session(&d).fork.position().tick(), 24);
        assert_eq!(set_speeds(&d), vec![(23, 0), (26, 20), (27, 0)]);
        d.advance(0.05);
        assert_eq!((d.tick, d.speed), (27, 0), "and the pause after it");
        frames(&mut d, 10);
        assert_eq!((d.tick, d.speed), (27, 0));
    }

    #[test]
    fn a_fork_opened_by_hand_stands_at_the_latest_tick_and_its_state_outlasts_the_count() {
        let mut d = driver();
        run_to(&mut d, 7);
        assert!(d.fork_step(Step::Tick).is_err(), "no fork is open");
        d.open_fork().unwrap();
        let s = session(&d);
        let stop = s.stop().unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::Opened, end(7)));
        assert_eq!(s.selected, None);
        assert_eq!(s.fork.state_hash(), Some(d.state_hash()));
        assert_eq!(s.fork.kept(), vec![7], "a fork saves its own at once");
        assert_eq!(s.run_speed, 10, "the map's speed until the window sets one");
        assert_eq!(d.debug.kept(), vec![7], "saved as it opened");
        assert!(set_speeds(&d).is_empty(), "opening by hand pauses nothing");
        run_to(&mut d, 31);
        assert_eq!(d.debug.kept(), vec![7, 20, 25, 30], "kept beyond the count");
        assert_eq!(
            session(&d).fork.position(),
            end(7),
            "the match ran, not the fork"
        );
        d.set_watches(vec![quiet()]).unwrap();
        d.close_fork();
        assert!(d.debug.session.is_none());
        assert_eq!(d.debug.kept(), vec![20, 25, 30], "the count again");
        d.set_watches(Vec::new()).unwrap();
        assert_eq!(d.debug.kept(), Vec::<u64>::new());
        // A fork alone keeps the match saved, and closing it stops that.
        d.open_fork().unwrap();
        run_to(&mut d, 36);
        assert_eq!(d.debug.kept(), vec![31, 35]);
        d.close_fork();
        assert_eq!(d.debug.kept(), Vec::<u64>::new());
    }

    #[test]
    fn the_fork_steps_a_selected_machine_of_the_team_and_goes_back() {
        let mut d = driver();
        run_to(&mut d, 10);
        d.open_fork().unwrap();
        let refused = d.fork_step(Step::Line).unwrap_err();
        assert!(refused.contains("select"), "{refused}");
        assert!(d.inspect(80).is_none(), "nothing is selected");
        let refused = d.fork_select(Some(EntityId(2))).unwrap_err();
        assert!(refused.contains("your own"), "{refused}");
        assert_eq!(session(&d).selected, None);
        d.fork_select(Some(PRINTER)).unwrap();
        let before = x(&d).expect("the printer's x");

        let line = d.fork_step(Step::Line).unwrap();
        assert_eq!(line.cause, Cause::Statement);
        assert_eq!(line.machine, Some(PRINTER));
        assert_eq!(line.position.tick(), 11);
        let seen = d.inspect(80).unwrap();
        assert_eq!(seen.record.id, PRINTER);
        assert!(!seen.program.unwrap().frames.is_empty());
        assert!(session(&d).snapshots.prev.is_none(), "a step is a jump");

        let tick = d.fork_step(Step::Tick).unwrap();
        assert_eq!((tick.cause, tick.position), (Cause::TickEnd, end(11)));
        let after: u64 = x(&d).unwrap().parse().unwrap();
        assert_eq!(after, before.parse::<u64>().unwrap() + 1);
        assert_eq!(session(&d).snapshots.cur.tick, 11, "the map draws the fork");

        assert!(session(&d).can_back());
        assert_eq!(d.fork_back().unwrap(), line);
        assert_eq!(session(&d).fork.position(), line.position);
        let opened = d.fork_back().unwrap();
        assert_eq!((opened.cause, opened.position), (Cause::Opened, end(10)));
        assert_eq!(x(&d), Some(before));
        assert!(!session(&d).can_back());
        assert!(d.fork_back().is_err(), "not at a fork's first stop");
        assert_eq!(d.tick, 10, "the match stood still throughout");
    }

    #[test]
    fn a_run_goes_at_the_windows_speed_until_paused_and_saves_the_forks_own_states() {
        let mut d = driver();
        run_to(&mut d, 10);
        d.open_fork().unwrap();
        assert!(d.fork_pause().is_err(), "nothing runs yet");
        d.fork_step(Step::Tick).unwrap();
        d.set_run_speed(20).unwrap();
        d.fork_run().unwrap();
        d.advance(0.5);
        assert_eq!(d.tick, 15, "the match at its own speed");
        let s = session(&d);
        let at = s.fork.position().tick();
        assert!((20..=22).contains(&at), "twice the match's speed: {at}");
        assert!(s.running);
        assert!(s.flags.ahead, "past the match's latest tick");
        assert_eq!(s.snapshots.cur.tick, at);
        assert_eq!(s.snapshots.prev.as_ref().map(|p| p.tick), Some(at - 1));
        assert_eq!(d.shown_speed(), 20);
        assert!(
            s.fork.stops().len() == 2,
            "a run records no stop as it goes"
        );
        // A fork that is running is paused before anything else is asked.
        assert!(!s.can_back());
        let refused = d.fork_step(Step::Tick).unwrap_err();
        assert!(refused.contains("pause"), "{refused}");
        let refused = d.fork_back().unwrap_err();
        assert!(refused.contains("pause"), "{refused}");
        let refused = d.fork_edit(x_of_main(), "1").unwrap_err();
        assert!(refused.contains("pause"), "{refused}");
        assert_eq!(session(&d).fork.position(), end(at));

        let stop = d.fork_pause().unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::Paused, end(at)));
        assert!(!session(&d).running);
        assert_eq!(d.shown_speed(), 0);
        d.advance(0.5);
        assert_eq!(session(&d).fork.position(), end(at), "paused");
        // On the match's terms: every multiple of five, the last three.
        let multiples: Vec<u64> = (10..=at).filter(|t| t % 5 == 0).collect();
        let last: Vec<u64> = multiples[multiples.len() - 3..].to_vec();
        assert_eq!(session(&d).fork.kept(), last);

        // A speed of zero holds a run without stopping it.
        d.set_run_speed(0).unwrap();
        d.fork_run().unwrap();
        d.advance(0.5);
        assert_eq!(session(&d).fork.position(), end(at));
        assert!(session(&d).running);
        d.fork_pause().unwrap();

        // Every other step has the tick limit.
        let stop = d.fork_step(Step::Continue).unwrap();
        assert_eq!(
            (stop.cause, stop.position),
            (Cause::StepLimit, end(at + 100))
        );
    }

    #[test]
    fn a_run_stops_at_a_watched_line_and_selects_its_machine() {
        let mut d = driver();
        run_to(&mut d, 38);
        d.open_fork().unwrap();
        d.fork_select(Some(BOT)).unwrap();
        d.set_watches(vec![watch(3)]).unwrap();
        d.fork_run().unwrap();
        d.advance(0.1);
        let s = session(&d);
        assert!(!s.running, "the stop ends the run");
        let stop = s.stop().unwrap();
        assert_eq!(stop.cause, Cause::Watch);
        assert_eq!(stop.machine, Some(PRINTER));
        assert_eq!(
            s.selected,
            Some(PRINTER),
            "a stop on another machine selects it"
        );
        assert_eq!(s.fork.stops().len(), 2);
        // The match hit too, with a fork already open: listed, no pause.
        assert_eq!(d.tick, 39);
        assert_eq!(listed(&d), vec![(39, BOT), (39, PRINTER)]);
        assert!(set_speeds(&d).is_empty());
    }

    #[test]
    fn an_edit_diverges_the_fork_and_the_debugger_never_changes_the_match() {
        let mut d = driver();
        let mut plain = driver();
        run_to(&mut d, 10);
        d.set_watches(vec![quiet()]).unwrap();
        d.open_fork().unwrap();
        let refused = d.fork_edit(x_of_main(), "100").unwrap_err();
        assert!(refused.contains("select"), "{refused}");
        d.fork_select(Some(PRINTER)).unwrap();
        assert!(!session(&d).flags.diverged);
        d.fork_edit(x_of_main(), "100").unwrap();
        assert!(session(&d).flags.diverged);
        assert_eq!(x(&d).as_deref(), Some("100"));
        assert!(d.fork_edit(x_of_main(), "nonsense(").is_err());
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(x(&d).as_deref(), Some("101"));
        d.fork_run().unwrap();
        run_to(&mut d, 40);
        d.fork_pause().unwrap();
        d.fork_back().unwrap();
        run_to(&mut plain, 40);
        assert_eq!(d.state_hash(), plain.state_hash());
        assert_eq!(d.log.canonical_bytes(), plain.log.canonical_bytes());
    }

    #[test]
    fn a_fork_run_ahead_falls_stale_when_the_match_takes_a_command_it_ran_past() {
        let mut d = driver();
        run_to(&mut d, 10);
        d.open_fork().unwrap();
        let stop = d.fork_step(Step::Continue).unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(110)));
        let flags = session(&d).flags;
        assert!(flags.ahead && flags.stale.is_none() && !flags.diverged);
        // A speed is the match's alone and stales nothing; a deploy does.
        assert_eq!(d.submit(CommandKind::SetSpeed(10)).unwrap(), 13);
        d.advance(0.1);
        assert_eq!(session(&d).flags.stale, None);
        let redeploy = CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle_of(COUNTER),
        };
        assert_eq!(d.submit(redeploy).unwrap(), 14);
        d.advance(0.1);
        assert_eq!(session(&d).flags.stale, Some(14), "read every frame");
    }

    #[test]
    fn with_no_saved_state_a_fork_reruns_from_the_start() {
        let mut d = driver();
        run_to(&mut d, 10);
        d.set_watches(vec![watch(3)]).unwrap();
        d.debug.states.clear();
        d.advance(0.1);
        let s = session(&d);
        assert_eq!(s.from, 0);
        assert_eq!(s.fork.position().tick(), 11);
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(session(&d).fork.state_hash(), Some(d.state_hash()));
        assert!(d.events.is_empty(), "{:?}", d.events);
    }

    #[test]
    fn reopening_replaces_the_fork_and_closing_returns_the_map_to_the_match() {
        let mut d = hit_at_39();
        frames(&mut d, 10);
        d.set_run_speed(40).unwrap();
        d.fork_edit(x_of_main(), "7").unwrap();
        assert!(session(&d).flags.diverged);
        assert_eq!(listed(&d), vec![(42, BOT), (42, PRINTER), (41, BOT)]);
        assert_eq!(d.debug.pinned, BTreeMap::from([(38, 1), (40, 3)]));
        assert!(d.reopen(3).is_err(), "no such hit");
        assert!(session(&d).flags.diverged, "a refusal leaves the fork");

        d.debug.show_match = true;
        d.reopen(2).unwrap();
        assert!(d.draws_fork(), "a fork just opened is the one drawn");
        let s = session(&d);
        let stop = s.stop().unwrap();
        assert_eq!(stop.cause, Cause::Watch);
        assert_eq!((stop.position.tick(), stop.machine), (41, Some(BOT)));
        assert_eq!(s.fork.position(), d.debug.hits[2].hit.position);
        assert_eq!(
            s.fork.stops().len(),
            1,
            "a new fork, its log and stops gone"
        );
        assert!(s.flags.faithful());
        assert_eq!(s.selected, Some(BOT));
        assert_eq!(s.from, 40);
        assert_eq!(s.run_speed, 40, "the window's speed stays");
        assert_eq!(listed(&d).len(), 3, "the hit stays listed");
        assert_eq!(
            d.debug.pinned,
            BTreeMap::from([(40, 4)]),
            "the first fork's state is let go; this one and three hits need 40's"
        );

        // The map draws the fork, or the match on request.
        assert!(d.draws_fork());
        assert!(std::ptr::eq(d.shown(), &session(&d).snapshots));
        d.debug.show_match = true;
        assert!(!d.draws_fork());
        assert!(std::ptr::eq(d.shown(), &d.snapshots));
        assert_eq!(d.shown_speed(), 0, "the paused match's");
        assert!(d.debug.session.is_some(), "the fork stays open");
        d.fork_run().unwrap();
        assert_eq!(d.shown_speed(), 0, "the match's still");
        d.debug.show_match = false;
        assert_eq!(d.shown_speed(), 40, "the fork's run");
        d.debug.show_match = true;

        d.close_fork();
        assert!(d.debug.session.is_none() && d.debug.hits.is_empty());
        assert!(!d.debug.show_match && !d.draws_fork());
        assert!(std::ptr::eq(d.shown(), &d.snapshots));
        assert!(d.debug.pinned.is_empty(), "{:?}", d.debug.pinned);
        assert!(d.debug.debugging(), "the breakpoint is still set");
    }

    /// Two peers over a loopback, each team's machines running `COUNTER`.
    fn peers() -> (Driver, Driver) {
        let (la, lb) = net::exchange::Loopback::pair(TeamId(0), TeamId(1));
        let map = map_text();
        let mut a = Driver::with_exchange(&map, &[], TeamId(0), opening(), Box::new(la)).unwrap();
        let b = Driver::with_exchange(&map, &[], TeamId(1), opening(), Box::new(lb)).unwrap();
        a.debug.tuning = tuning();
        (a, b)
    }

    fn both(a: &mut Driver, b: &mut Driver, n: usize) {
        for _ in 0..n {
            a.advance(0.1);
            b.advance(0.1);
        }
    }

    #[test]
    fn a_pvp_match_runs_on_at_a_hit_and_its_fork_waits_at_the_live_tick_and_takes_no_edit() {
        let (mut a, mut b) = peers();
        both(&mut a, &mut b, 10);
        let theirs = Watch {
            team: TeamId(1),
            ..watch(3)
        };
        let refused = a.set_watches(vec![watch(3), theirs]).unwrap_err();
        assert!(refused.contains("own"), "{refused}");
        assert!(a.debug.watches().is_empty() && !a.debug.debugging());
        let set_at = a.tick;
        a.set_watches(vec![watch(3)]).unwrap();
        both(&mut a, &mut b, 20);
        assert_eq!(session(&a).fork.position().tick(), set_at + 1);
        assert!(a.submitted.is_empty(), "no pause is sent to a PvP match");
        assert_eq!(a.speed, 10);
        assert!(a.tick >= set_at + 15, "it ran on: {}", a.tick);
        assert!(a.live_pvp());
        // The watches are one peer's alone and no part of the hash.
        assert!(a.desync.is_none() && b.desync.is_none());
        assert!(a.hashes_compared >= 20 && b.hashes_compared >= 20);

        let refused = a.fork_edit(x_of_main(), "5").unwrap_err();
        assert!(refused.contains("live PvP"), "{refused}");
        assert!(!session(&a).flags.diverged);
        a.set_watches(Vec::new()).unwrap();
        let stop = a.fork_step(Step::Continue).unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::LiveTick, end(a.tick)));
        assert!(!session(&a).flags.ahead);
        // It waits there: a run stops at once, until the match moves on.
        a.fork_run().unwrap();
        let at = a.tick;
        a.run_fork(1.0);
        assert!(!session(&a).running);
        assert_eq!(session(&a).fork.position(), end(at));

        // A match that is over is no longer live: its fork takes an edit.
        a.ended = Some(None);
        assert!(!a.live_pvp());
        a.open_fork().unwrap();
        a.fork_select(Some(PRINTER)).unwrap();
        a.fork_edit(x_of_main(), "5").unwrap();
        assert!(session(&a).flags.diverged);
    }

    /// Sixty ticks of the single-player match, as the replay it saves.
    fn recorded() -> replay::Replay {
        let mut d = driver();
        run_to(&mut d, 60);
        d.replay()
    }

    #[test]
    fn a_hit_on_a_replay_pauses_the_playback_at_once_and_opens_a_fork_of_it() {
        let mut d = Driver::open_replay(recorded(), tuning()).unwrap();
        frames(&mut d, 4);
        assert_eq!(d.tick, 4);
        d.set_watches(vec![watch(3)]).unwrap();
        d.advance(1.0);
        assert_eq!((d.tick, d.speed), (5, 0), "at the end of the hit's tick");
        let s = session(&d);
        // The playback saved tick 5 as it arrived; the fork needs one
        // before the hit's tick.
        assert_eq!(s.from, 0);
        let stop = s.stop().unwrap();
        assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(PRINTER)));
        assert_eq!(stop.position.tick(), 5);
        assert!(s.flags.faithful());
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(session(&d).fork.state_hash(), Some(d.state_hash()));
        d.advance(1.0);
        assert_eq!(d.tick, 5, "paused");

        // The pace is the viewer's: further hits are listed, not paused at.
        d.set_pace(10).unwrap();
        frames(&mut d, 3);
        assert_eq!((d.tick, d.speed), (8, 10));
        assert_eq!(listed(&d), vec![(8, PRINTER), (7, PRINTER), (6, PRINTER)]);
        assert!(d.submitted.is_empty() && d.events.is_empty());
        // Sought back, the playback plays through the same hits: each is
        // listed once.
        d.set_pace(0).unwrap();
        d.seek(5).unwrap();
        frames(&mut d, 3);
        assert_eq!((d.tick, d.seeking), (5, None));
        d.set_pace(10).unwrap();
        frames(&mut d, 3);
        assert_eq!(listed(&d), vec![(8, PRINTER), (7, PRINTER), (6, PRINTER)]);
        // The playback saves tick 10 as it arrives there; the hit in tick
        // 10 needs the state before it.
        frames(&mut d, 2);
        assert_eq!(listed(&d), vec![(10, PRINTER), (9, PRINTER), (8, PRINTER)]);
        assert_eq!(d.debug.hits[0].from, 5);
        // *Back* goes on the playback's states, and the fork runs on the
        // replay's log, past where the playback stands.
        let stop = d.fork_step(Step::Continue).unwrap();
        assert_eq!((stop.cause, stop.position.tick()), (Cause::Watch, 6));
        d.set_watches(Vec::new()).unwrap();
        d.fork_step(Step::Tick).unwrap();
        assert_eq!(d.fork_back().unwrap(), stop);
        d.reopen(0).unwrap();
        assert_eq!(session(&d).fork.position().tick(), 10);
        assert_eq!(session(&d).from, 5);
        d.close_fork();
        assert!(d.debug.hits.is_empty());
        assert_eq!(
            d.viewer().unwrap().kept(),
            vec![0, 5, 10],
            "the pins are let go"
        );
    }

    #[test]
    fn a_fork_opened_by_hand_on_a_replay_stands_where_the_playback_does() {
        let mut d = Driver::open_replay(recorded(), tuning()).unwrap();
        d.set_pace(0).unwrap();
        d.seek(22).unwrap();
        for _ in 0..1000 {
            if d.seeking.is_none() {
                break;
            }
            d.advance(0.1);
        }
        assert_eq!(d.tick, 22);
        d.open_fork().unwrap();
        let s = session(&d);
        let stop = s.stop().unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::Opened, end(22)));
        assert_eq!(s.fork.state_hash(), Some(d.state_hash()));
        assert!(
            d.viewer().unwrap().kept().contains(&22),
            "saved as it opened"
        );
        let stop = d.fork_step(Step::Tick).unwrap();
        assert_eq!(stop.position, end(23));
        assert_eq!(d.tick, 22, "the playback stands");
        d.close_fork();
        assert!(!d.viewer().unwrap().kept().contains(&22));
    }

    #[test]
    fn a_replay_that_records_no_team_is_watched_not_debugged() {
        let mut replay = recorded();
        replay.team = None;
        let mut d = Driver::open_replay(replay, tuning()).unwrap();
        let refused = d.set_watches(vec![watch(3)]).unwrap_err();
        assert!(refused.contains("watched, not debugged"), "{refused}");
        let refused = d.open_fork().unwrap_err();
        assert!(refused.contains("watched, not debugged"), "{refused}");
        d.set_watches(Vec::new()).unwrap();
        frames(&mut d, 10);
        assert_eq!(d.tick, 10, "it plays");
        assert!(d.debug.session.is_none() && !d.debug.debugging());
    }
}
