//! The driver (`docs/06`, The driver): the loop that decides when a tick
//! runs. Plain Rust — no Bevy type — so it is tested headless; `app.rs`
//! calls [`Driver::advance`] once per frame with the wall-clock delta.
//!
//! With an [`Exchange`] the same loop is a peer of a match: it sends its
//! own sets and hashes, takes the others' sets into the log, stalls until
//! every set for the next tick is in hand (Q12), and stops on the first
//! tick two peers hash differently. Without one it is single-player, which
//! is lockstep with one peer.

use net::CommandLog;
use net::exchange::{Event, Exchange};
use net::tcp::{MatchIdentity, Session};
use net::wire::Message;
use sim::snapshot::Snapshot;
use sim::{Command, CommandKind, Data, Map, Sim, StepReport, TeamId, TilePos};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Own hashes kept for comparing against a peer's late-arriving one.
const HASHES_KEPT: u64 = 4096;

/// The last two completed-tick snapshots and the real time since the last.
pub struct Snapshots {
    pub prev: Option<Snapshot>,
    pub cur: Snapshot,
    /// Seconds since `cur` was published, for interpolation (Q6).
    pub since: f64,
}

impl Snapshots {
    /// The fraction of the current tick that has elapsed, in `0..=1`.
    pub fn fraction(&self, speed: u64) -> f32 {
        if speed == 0 {
            return 1.0;
        }
        (self.since * speed as f64).clamp(0.0, 1.0) as f32
    }
}

/// A tick whose sets have not all arrived, past `stall_report_ticks`
/// (Q12, Q38): the peers awaited and how long this driver has waited.
#[derive(Debug, Clone, PartialEq)]
pub struct Stall {
    /// The tick the driver cannot run yet.
    pub tick: u64,
    pub awaited: Vec<TeamId>,
    /// Real seconds waited so far.
    pub waited: f64,
}

impl Stall {
    /// The awaited peers as a list for the screen: `team 1, team 2`.
    pub fn peers(&self) -> String {
        self.awaited
            .iter()
            .map(|t| format!("team {}", t.0))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Two peers disagree about the match (`docs/06`, The state hash): the
/// driver runs no further tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desync {
    /// The tick the peers disagree on.
    pub tick: u64,
    /// This peer's hash for that tick, when the disagreement is over a
    /// hash; a refused set disagrees before any hash exists.
    pub own: Option<u64>,
    /// The report, naming the tick, the peer and both hashes.
    pub report: String,
}

pub struct Driver {
    sim: Sim,
    pub log: CommandLog,
    /// The last completed tick.
    pub tick: u64,
    /// Ticks per real second; `0` is pause (Q6).
    pub speed: u64,
    pub delay: u64,
    pub stall_report_ticks: u64,
    /// The team the player commands.
    pub player: TeamId,
    /// Peers whose sets are local — the player and every scripted team —
    /// and therefore always in hand.
    local_peers: BTreeSet<TeamId>,
    accumulated: f64,
    /// Real seconds spent waiting on a missing set.
    stalled: f64,
    pub stall: Option<Stall>,
    pub snapshots: Snapshots,
    pub last_report: StepReport,
    pub ended: Option<Option<TeamId>>,
    /// Commands the player submitted, for the log view.
    pub submitted: Vec<Command>,
    pub map_name: String,
    /// The map's bytes, which with the log are the replay (Q36).
    map_text: String,
    /// The map's south-west and north-east corners, for the view's frame.
    pub bounds: (TilePos, TilePos),
    /// The other peers, if this is a match; none in single-player.
    exchange: Option<Box<dyn Exchange>>,
    /// The peers whose sets arrive through the exchange.
    pub remote_peers: BTreeSet<TeamId>,
    /// The last tick whose local sets have been sent; a submission can no
    /// longer target it.
    sent_through: Option<u64>,
    own_hashes: BTreeMap<u64, u64>,
    /// Hashes peers reported for ticks this driver has not completed yet.
    peer_hashes: BTreeMap<(u64, TeamId), u64>,
    /// Two peers hashed a tick differently: the report, and the driver
    /// runs no further tick (`docs/06`, The state hash).
    pub desync: Option<Desync>,
    /// What happened on the exchange, for the log view; drained by it.
    pub events: Vec<String>,
    /// Peer hashes compared against this driver's own, so a test can tell
    /// agreement from silence.
    pub hashes_compared: u64,
}

impl Driver {
    /// A single-player match on `map`: the player as team 0, and scripted
    /// teams from `opposition` directories in team order for the rest.
    pub fn new(
        map_text: &str,
        opposition: &[&Path],
        player_opening: Vec<CommandKind>,
    ) -> Result<Driver, String> {
        Driver::build(map_text, opposition, TeamId(0), player_opening, None)
    }

    /// A peer of a match: the player as `player`, scripted teams from
    /// `opposition` in team order from 1 (the host's; a joiner passes none),
    /// and every other team's sets arriving through `exchange`.
    pub fn with_exchange(
        map_text: &str,
        opposition: &[&Path],
        player: TeamId,
        player_opening: Vec<CommandKind>,
        exchange: Box<dyn Exchange>,
    ) -> Result<Driver, String> {
        Driver::build(map_text, opposition, player, player_opening, Some(exchange))
    }

    /// What two peers must agree on before a match: the map's bytes and
    /// the delay (`docs/06`, The command log).
    pub fn identity(map_text: &str) -> Result<MatchIdentity, String> {
        let data = Data::load().map_err(|e| e.to_string())?;
        let mut h = sim::hash::Fnv1a::new();
        h.write_str(map_text);
        Ok(MatchIdentity {
            map_hash: h.finish(),
            delay: data.net.delay,
        })
    }

    /// The map's teams that are neither the player's nor scripted: the
    /// ones a host waits for.
    pub fn remote_teams(map_text: &str, opposition_count: usize) -> Result<Vec<TeamId>, String> {
        let data = Data::load().map_err(|e| e.to_string())?;
        let map = Map::parse(map_text, &data).map_err(|e| e.to_string())?;
        let first = opposition_count.saturating_add(1) as u32;
        Ok((first..map.teams.len() as u32).map(TeamId).collect())
    }

    /// Host a match on `addr`: wait for a peer per remote team, then build
    /// the driver as team 0 with the scripted opposition.
    pub fn host(
        map_text: &str,
        opposition: &[&Path],
        player_opening: Vec<CommandKind>,
        addr: &str,
        progress: impl FnMut(&str),
    ) -> Result<Driver, String> {
        let session = Driver::host_handshake(map_text, opposition.len(), addr, progress)?;
        Driver::with_exchange(
            map_text,
            opposition,
            TeamId(0),
            player_opening,
            Box::new(session),
        )
    }

    /// The host's half of [`Driver::host`] that blocks: listen on `addr`
    /// until a peer has joined for every remote team. It holds no sim, so
    /// it can run off the main thread while the window stays live.
    pub fn host_handshake(
        map_text: &str,
        opposition_count: usize,
        addr: &str,
        progress: impl FnMut(&str),
    ) -> Result<Session, String> {
        let remote = Driver::remote_teams(map_text, opposition_count)?;
        if remote.is_empty() {
            return Err(format!(
                "the map has no team left for a peer once the player and {opposition_count} scripted team(s) are seated"
            ));
        }
        let identity = Driver::identity(map_text)?;
        let (session, _) = net::tcp::host(addr, TeamId(0), &remote, identity, progress)?;
        Ok(session)
    }

    /// Join a match at `addr` as whichever team the host assigns.
    pub fn join(
        map_text: &str,
        player_opening: Vec<CommandKind>,
        addr: &str,
    ) -> Result<Driver, String> {
        let session = Driver::join_handshake(map_text, addr)?;
        let me = session.me;
        Driver::with_exchange(map_text, &[], me, player_opening, Box::new(session))
    }

    /// The joiner's half of [`Driver::join`] that blocks: connect, offer
    /// the map, and take a team the map has.
    pub fn join_handshake(map_text: &str, addr: &str) -> Result<Session, String> {
        let data = Data::load().map_err(|e| e.to_string())?;
        let map = Map::parse(map_text, &data).map_err(|e| e.to_string())?;
        let teams: Vec<TeamId> = (0..map.teams.len() as u32).map(TeamId).collect();
        let identity = Driver::identity(map_text)?;
        let session = net::tcp::join(addr, identity, &teams)?;
        if !teams.contains(&session.me) {
            return Err(format!(
                "the host seated this peer as team {}, which the map lacks",
                session.me.0
            ));
        }
        Ok(session)
    }

    fn build(
        map_text: &str,
        opposition: &[&Path],
        player: TeamId,
        player_opening: Vec<CommandKind>,
        exchange: Option<Box<dyn Exchange>>,
    ) -> Result<Driver, String> {
        let data = Data::load().map_err(|e| e.to_string())?;
        let map = Map::parse(map_text, &data).map_err(|e| e.to_string())?;
        let teams: Vec<TeamId> = (0..map.teams.len() as u32).map(TeamId).collect();
        if !teams.contains(&player) {
            return Err(format!(
                "the map has {} teams; none is team {}",
                teams.len(),
                player.0
            ));
        }
        let mut log = CommandLog::new(teams.iter().copied());
        let mut local_peers: BTreeSet<TeamId> = BTreeSet::new();
        local_peers.insert(player);
        let mut scripted = Vec::new();
        for (i, dir) in opposition.iter().enumerate() {
            let team = TeamId(i as u32 + 1);
            if !teams.contains(&team) || team == player {
                return Err(format!(
                    "the map has {} teams; no room for opposition {}",
                    teams.len(),
                    dir.display()
                ));
            }
            let script = sim::script::Script::load(dir)?;
            scripted.extend(script.resolve(team, map.teams[team.0 as usize][0]));
            local_peers.insert(team);
        }
        let remote_peers: BTreeSet<TeamId> = match &exchange {
            Some(x) => x.peers().into_iter().collect(),
            None => BTreeSet::new(),
        };
        let unseated: Vec<TeamId> = teams
            .iter()
            .copied()
            .filter(|t| !local_peers.contains(t) && !remote_peers.contains(t))
            .collect();
        if exchange.is_some() && !unseated.is_empty() {
            return Err(format!(
                "no peer for team(s) {}",
                unseated
                    .iter()
                    .map(|t| t.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if exchange.is_none() {
            // Single-player: every team not scripted is the player's
            // silent partner — its sets are empty and always in hand.
            local_peers.extend(unseated);
        }
        let map_name = map.name.clone();
        let bounds = map.bounds();
        let speed = map.speed;
        let sim = Sim::new(map);
        for c in &scripted {
            sim.validate(c)?;
            log.push(c.clone());
        }
        for kind in player_opening {
            let c = Command::new(0, player, 0, kind);
            sim.validate(&c)?;
            log.push(c);
        }
        let cur = sim.snapshot();
        Ok(Driver {
            sim,
            log,
            tick: 0,
            speed,
            delay: data.net.delay,
            stall_report_ticks: data.net.stall_report_ticks,
            player,
            local_peers,
            accumulated: 0.0,
            stalled: 0.0,
            stall: None,
            snapshots: Snapshots {
                prev: None,
                cur,
                since: 0.0,
            },
            last_report: StepReport::default(),
            ended: None,
            submitted: Vec::new(),
            map_name,
            map_text: map_text.to_string(),
            bounds,
            exchange,
            remote_peers,
            sent_through: None,
            own_hashes: BTreeMap::new(),
            peer_hashes: BTreeMap::new(),
            desync: None,
            events: Vec::new(),
            hashes_compared: 0,
        })
    }

    /// Submit a command from the player: validated as a peer would, agreed
    /// for `tick + delay`, and forgotten — what the player sees is the
    /// sim's answer, `delay` ticks later. Returns the agreed tick. A set
    /// already sent to the peers is final, so a submission during a stall
    /// lands on the first tick still open.
    pub fn submit(&mut self, kind: CommandKind) -> Result<u64, String> {
        let tick = self.next_open_tick();
        let c = Command::new(tick, self.player, 0, kind);
        self.sim.validate(&c)?;
        self.submitted.push(c.clone());
        self.log.push(c);
        Ok(tick)
    }

    /// The tick a submission made now is agreed for: `delay` ticks on, or
    /// the first tick whose set has not been sent, whichever is later — so
    /// a resign during a stall can say where it lands (Q38).
    pub fn next_open_tick(&self) -> u64 {
        let open = self.sent_through.map_or(0, |t| t.saturating_add(1));
        self.tick.saturating_add(self.delay).max(open)
    }

    /// The player's own state hash, for the display.
    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// Every local peer's set for every tick through `tick` is in hand by
    /// construction, and sent to the peers once — after which nothing may
    /// be added to it. An empty set is sent explicitly (Q12).
    fn flush_local_sets(&mut self, tick: u64) {
        let from = self.sent_through.map_or(0, |t| t.saturating_add(1));
        for t in from..=tick {
            for peer in self.local_peers.clone() {
                if !self.log.has_set(peer, t) {
                    self.log.submit_set(peer, t, vec![]);
                }
                if let Some(x) = &mut self.exchange {
                    let commands: Vec<Command> = self
                        .log
                        .sets_for(t)
                        .into_iter()
                        .filter(|c| c.sender == peer)
                        .collect();
                    x.send(&Message::Set {
                        sender: peer,
                        tick: t,
                        commands,
                    });
                }
            }
        }
        self.sent_through = Some(tick);
    }

    /// Take what the peers sent: their sets into the log, their hashes
    /// against this driver's own, their goodbyes out of the awaited set.
    fn receive(&mut self) {
        let Some(x) = &mut self.exchange else { return };
        let events = x.poll();
        for e in events {
            match e {
                Event::Message(Message::Set {
                    sender,
                    tick,
                    commands,
                }) => {
                    if !self.remote_peers.contains(&sender) {
                        continue;
                    }
                    for c in &commands {
                        if let Err(err) = self.sim.validate(c) {
                            // Every peer validates at submission, so a set
                            // this peer refuses means the peers disagree
                            // on the match itself.
                            let report = format!(
                                "team {} sent a command for tick {tick} this peer refuses: {err}",
                                sender.0
                            );
                            self.events.push(report.clone());
                            self.desync.get_or_insert(Desync {
                                tick,
                                own: None,
                                report,
                            });
                            return;
                        }
                    }
                    self.log.submit_set(sender, tick, commands);
                }
                Event::Message(Message::Hash { sender, tick, hash }) => {
                    if !self.remote_peers.contains(&sender) {
                        continue;
                    }
                    match self.own_hashes.get(&tick) {
                        Some(own) => self.compare(tick, sender, *own, hash),
                        None => {
                            self.peer_hashes.insert((tick, sender), hash);
                        }
                    }
                }
                Event::Message(Message::Bye { sender }) | Event::Lost(sender) => {
                    if self.remote_peers.remove(&sender) {
                        self.log.close(sender);
                        self.events.push(format!(
                            "team {} left; its sets are no longer awaited",
                            sender.0
                        ));
                    }
                }
                Event::Message(_) => {}
            }
        }
    }

    fn compare(&mut self, tick: u64, peer: TeamId, own: u64, theirs: u64) {
        self.hashes_compared = self.hashes_compared.saturating_add(1);
        if own != theirs {
            let report = format!(
                "desync at tick {tick}: this peer hashed {own:016x}, team {} hashed {theirs:016x}",
                peer.0
            );
            self.events.push(report.clone());
            self.desync.get_or_insert(Desync {
                tick,
                own: Some(own),
                report,
            });
        }
    }

    /// Whether this driver is a peer of a match rather than single-player.
    pub fn is_networked(&self) -> bool {
        self.exchange.is_some()
    }

    fn run_tick(&mut self) {
        let t = self.tick.saturating_add(1);
        let mut commands = if t == 1 {
            self.log.sets_for(0)
        } else {
            Vec::new()
        };
        commands.extend(self.log.sets_for(t));
        let report = self.sim.step(&commands);
        if let Some(s) = report.speed {
            self.speed = s;
        }
        if report.ended.is_some() {
            self.ended = report.ended;
        }
        self.last_report = report;
        self.tick = t;
        // A tick ran, so whatever set was awaited has arrived: the stall
        // clock restarts from here, not from the last frame that ran no tick.
        self.stalled = 0.0;
        self.stall = None;
        let hash = self.sim.state_hash();
        self.own_hashes.insert(t, hash);
        if let Some(old) = t.checked_sub(HASHES_KEPT) {
            self.own_hashes.remove(&old);
        }
        if let Some(x) = &mut self.exchange {
            x.send(&Message::Hash {
                sender: self.player,
                tick: t,
                hash,
            });
        }
        let pending: Vec<(TeamId, u64)> = self
            .peer_hashes
            .range((t, TeamId(0))..=(t, TeamId(u32::MAX)))
            .map(|((_, p), h)| (*p, *h))
            .collect();
        for (peer, theirs) in pending {
            self.peer_hashes.remove(&(t, peer));
            self.compare(t, peer, hash, theirs);
        }
        let cur = self.sim.snapshot();
        let prev = std::mem::replace(&mut self.snapshots.cur, cur);
        self.snapshots.prev = Some(prev);
        self.snapshots.since = 0.0;
    }

    /// Whether every peer's set for `tick` is in hand, sending the local
    /// peers' first. The first tick also needs the opening sets (tick 0).
    fn ready(&mut self, tick: u64) -> bool {
        self.flush_local_sets(tick);
        (tick != 1 || self.log.have_all_sets(0)) && self.log.have_all_sets(tick)
    }

    /// One frame of the driver loop `docs/06` specifies, `dt` real seconds.
    pub fn advance(&mut self, dt: f64) {
        self.receive();
        if self.ended.is_some() || self.desync.is_some() {
            return;
        }
        self.snapshots.since += dt;
        if self.speed == 0 {
            // Paused: run exactly the ticks up to the next one that carries
            // a command, without the clock (Q32).
            let Some(target) = self.log.next_tick_with_a_command(self.tick) else {
                return;
            };
            let all_ready = (self.tick.saturating_add(1)..=target).all(|t| self.ready(t));
            if !all_ready {
                self.stall(dt);
                return;
            }
            while self.tick < target && self.ended.is_none() {
                self.run_tick();
            }
            self.stalled = 0.0;
            self.stall = None;
            return;
        }
        self.accumulated += dt;
        let period = 1.0 / self.speed as f64;
        // A frame far longer than a tick runs a bounded number of ticks, so
        // a stalled window catches up gracefully instead of freezing.
        let mut ran = 0u32;
        while self.accumulated >= period && ran < 64 {
            let next = self.tick.saturating_add(1);
            if !self.ready(next) {
                self.stall(dt);
                return;
            }
            self.run_tick();
            self.accumulated -= period;
            ran += 1;
            if self.ended.is_some() {
                break;
            }
        }
        self.stalled = 0.0;
        self.stall = None;
    }

    fn stall(&mut self, dt: f64) {
        self.stalled += dt;
        let threshold = if self.speed == 0 {
            self.stall_report_ticks as f64
        } else {
            self.stall_report_ticks as f64 / self.speed as f64
        };
        if self.stalled >= threshold {
            let tick = self.tick.saturating_add(1);
            self.stall = Some(Stall {
                tick,
                awaited: self.log_missing(tick),
                waited: self.stalled,
            });
        }
    }

    /// The peers whose set for `tick` is missing.
    fn log_missing(&self, tick: u64) -> Vec<TeamId> {
        self.log.missing_for(tick)
    }

    /// The match so far as a replay (Q36): the map's bytes, every command
    /// in the log, and the ticks run. The headless driver reproduces this
    /// peer's hash stream from it.
    pub fn replay(&self) -> replay::Replay {
        replay::Replay {
            map: self.map_text.clone(),
            commands: self.log.commands().cloned().collect(),
            ticks: self.replay_point().0,
        }
    }

    /// The tick the replay runs to and this peer's hash there, which name
    /// its file (Q36): the last tick run — or, on a desync over a hash,
    /// the tick the peers disagree on, though the driver may have run past
    /// it before the other's hash arrived (Q38).
    pub fn replay_point(&self) -> (u64, u64) {
        match &self.desync {
            Some(Desync {
                tick,
                own: Some(own),
                ..
            }) => (*tick, *own),
            _ => (self.tick, self.state_hash()),
        }
    }

    /// The last completed-tick snapshot.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshots.cur
    }

    /// The kind of plan `team` holds on `p`, for drawing the player's own
    /// marks: `building`, `paint` or `overlay`, the first set. A read of
    /// world state the snapshot does not carry per tile; nothing is
    /// written.
    pub fn snapshot_plan(&self, team: TeamId, p: sim::TilePos) -> Option<String> {
        let plans = self.sim.world.plans(team, p)?;
        if plans.building.is_some() {
            Some("building".into())
        } else if plans.paint.is_some() {
            Some("paint".into())
        } else if plans.overlay.is_some() {
            Some("overlay".into())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::bundle_of;

    fn driver() -> Driver {
        let map = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/maps/first.toml"
        ))
        .unwrap();
        let fool = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/opposition/fool"
        ));
        Driver::new(
            &map,
            &[&fool],
            vec![CommandKind::Deploy {
                deployment: "printer".into(),
                bundle: bundle_of("while True:\n    try:\n        print(1)\n    except ValueError:\n        wait(5)\n"),
            }],
        )
        .unwrap()
    }

    #[test]
    fn the_map_sets_the_starting_speed_and_ticks_follow_the_clock() {
        let mut d = driver();
        assert_eq!(d.speed, 10);
        d.advance(0.05);
        assert_eq!(d.tick, 0, "half a tick period: no tick yet");
        d.advance(0.05);
        assert_eq!(d.tick, 1);
        d.advance(1.0);
        assert_eq!(d.tick, 11);
        assert!(d.snapshots.prev.is_some());
        assert_eq!(d.snapshots.cur.tick, 11);
        assert!(d.stall.is_none());
    }

    #[test]
    fn a_submitted_command_is_agreed_delay_ticks_later_and_the_paused_driver_reaches_it() {
        let mut d = driver();
        d.advance(0.5);
        assert_eq!(d.tick, 5);
        let agreed = d.submit(CommandKind::SetSpeed(0)).unwrap();
        assert_eq!(agreed, 8, "tick + delay");
        d.advance(1.0);
        assert_eq!(d.speed, 0, "the SetSpeed landed on its tick");
        assert_eq!(d.tick, 15, "the clock ran the rest of the second");
        // Paused: nothing runs until a command is in the log, then exactly
        // the ticks up to it.
        d.advance(5.0);
        assert_eq!(d.tick, 15);
        let agreed = d.submit(CommandKind::SetSpeed(20)).unwrap();
        assert_eq!(agreed, 18);
        d.advance(0.001);
        assert_eq!(d.tick, 18, "ran up to the command's tick without the clock");
        assert_eq!(d.speed, 20);
        d.advance(0.5);
        assert_eq!(d.tick, 28);
    }

    #[test]
    fn a_missing_peer_set_stalls_and_is_reported_after_the_threshold() {
        let mut d = driver();
        // A third peer nobody submits for: the driver waits.
        d.log = {
            let mut log = CommandLog::new([TeamId(0), TeamId(1), TeamId(7)]);
            for c in d.log.commands() {
                log.push(c.clone());
            }
            log
        };
        d.advance(1.0);
        assert_eq!(d.tick, 0, "a tick ran without every peer's set");
        assert!(d.stall.is_none(), "reported too early");
        for _ in 0..30 {
            d.advance(0.11);
        }
        let stall = d
            .stall
            .clone()
            .expect("a stall report after stall_report_ticks");
        assert_eq!(stall.tick, 1);
        assert_eq!(stall.awaited, vec![TeamId(7)]);
        assert_eq!(stall.peers(), "team 7");
        assert!(stall.waited >= 3.0, "waited {}", stall.waited);
        // The sets arrive: the ticks run (a bounded burst per frame, so the
        // stalled seconds catch up) and the report clears.
        for t in 0..=100 {
            d.log.submit_set(TeamId(7), t, vec![]);
        }
        d.advance(0.1);
        assert!(d.tick >= 30, "tick {}", d.tick);
        assert!(d.stall.is_none());
    }

    #[test]
    fn a_resign_during_a_stall_lands_on_the_tick_it_was_told() {
        let mut d = driver();
        d.log = {
            let mut log = CommandLog::new([TeamId(0), TeamId(1), TeamId(7)]);
            for c in d.log.commands() {
                log.push(c.clone());
            }
            log
        };
        for _ in 0..40 {
            d.advance(0.11);
        }
        let stall = d.stall.clone().expect("stalled");
        let lands = d.next_open_tick();
        assert!(lands > stall.tick, "{lands}");
        assert_eq!(d.submit(CommandKind::Resign).unwrap(), lands);
        for t in 0..=100 {
            d.log.submit_set(TeamId(7), t, vec![]);
        }
        d.advance(1.0);
        assert!(d.stall.is_none());
        assert_eq!(d.tick, lands, "the match ended on the resign's tick");
        assert_eq!(d.ended, Some(Some(TeamId(1))), "the Fool stands alone");
    }

    fn map_text() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/maps/first.toml"
        ))
        .unwrap()
    }

    fn opening() -> Vec<CommandKind> {
        vec![CommandKind::Deploy {
            deployment: "printer".into(),
            bundle: bundle_of(
                "while True:\n    try:\n        print(1)\n    except ValueError:\n        wait(5)\n",
            ),
        }]
    }

    /// Two peers over a loopback, the player of each the map's two teams.
    fn peers() -> (Driver, Driver) {
        let (la, lb) = net::exchange::Loopback::pair(TeamId(0), TeamId(1));
        let map = map_text();
        let a = Driver::with_exchange(&map, &[], TeamId(0), opening(), Box::new(la)).unwrap();
        let b = Driver::with_exchange(&map, &[], TeamId(1), opening(), Box::new(lb)).unwrap();
        (a, b)
    }

    #[test]
    fn two_peers_run_in_lockstep_agree_on_every_hash_and_share_a_command() {
        let (mut a, mut b) = peers();
        assert_eq!(a.remote_peers, BTreeSet::from([TeamId(1)]));
        for _ in 0..30 {
            a.advance(0.1);
            b.advance(0.1);
        }
        assert!(a.tick >= 25 && b.tick >= 25, "{} {}", a.tick, b.tick);
        assert!(a.tick.abs_diff(b.tick) <= 1, "{} {}", a.tick, b.tick);
        assert!(a.desync.is_none() && b.desync.is_none());
        assert!(a.hashes_compared >= 20 && b.hashes_compared >= 20);
        assert!(a.stall.is_none() && b.stall.is_none());
        // A's command lands on B on the agreed tick.
        let agreed = a.submit(CommandKind::SetSpeed(2)).unwrap();
        assert!(agreed > a.tick);
        for _ in 0..20 {
            a.advance(0.1);
            b.advance(0.1);
        }
        assert!(b.tick > agreed);
        assert_eq!(b.speed, 2, "B did not take A's SetSpeed");
        assert_eq!(a.speed, 2);
        assert!(a.desync.is_none() && b.desync.is_none());
    }

    #[test]
    fn a_peer_that_stops_stalls_the_other_until_it_returns() {
        let (mut a, mut b) = peers();
        for _ in 0..10 {
            a.advance(0.1);
            b.advance(0.1);
        }
        let at = a.tick;
        for _ in 0..40 {
            a.advance(0.11);
        }
        assert!(a.tick <= at + 1, "A ran on without B's sets");
        let stall = a.stall.clone().expect("a stall report");
        assert_eq!(stall.awaited, vec![TeamId(1)]);
        // A submission during the stall lands on a tick not yet sent.
        let agreed = a.submit(CommandKind::SetSpeed(5)).unwrap();
        for _ in 0..40 {
            b.advance(0.1);
            a.advance(0.1);
        }
        assert!(a.stall.is_none());
        assert!(a.tick > agreed && b.tick > agreed);
        assert_eq!(b.speed, 5);
        assert!(a.desync.is_none() && b.desync.is_none());
    }

    /// A driver against a hand-driven end: the test plays team 1.
    fn against_the_test() -> (Driver, net::exchange::Loopback) {
        let (la, lb) = net::exchange::Loopback::pair(TeamId(0), TeamId(1));
        let a =
            Driver::with_exchange(&map_text(), &[], TeamId(0), opening(), Box::new(la)).unwrap();
        (a, lb)
    }

    /// Answer every set A sends with an empty set of team 1's, and return
    /// A's hashes.
    fn answer(end: &mut net::exchange::Loopback) -> Vec<(u64, u64)> {
        let mut hashes = Vec::new();
        for e in end.poll() {
            match e {
                Event::Message(Message::Set { tick, .. }) => end.send(&Message::Set {
                    sender: TeamId(1),
                    tick,
                    commands: vec![],
                }),
                Event::Message(Message::Hash { tick, hash, .. }) => hashes.push((tick, hash)),
                _ => {}
            }
        }
        hashes
    }

    #[test]
    fn a_peer_hashing_a_tick_differently_is_a_desync_that_stops_the_driver() {
        let (mut a, mut end) = against_the_test();
        let mut sent = false;
        for _ in 0..40 {
            a.advance(0.1);
            for (tick, hash) in answer(&mut end) {
                if tick == 3 && !sent {
                    sent = true;
                    end.send(&Message::Hash {
                        sender: TeamId(1),
                        tick,
                        hash: hash ^ 1,
                    });
                }
            }
        }
        let desync = a.desync.clone().expect("a desync report");
        assert_eq!(desync.tick, 3);
        assert!(desync.report.contains("desync at tick 3"), "{desync:?}");
        // The replay stops at the tick the peers disagree on and reproduces
        // this peer's hash there, whatever the driver ran past it (Q38).
        let own = desync.own.expect("a hash desync carries this peer's hash");
        assert_eq!(a.replay_point(), (3, own));
        let replay = a.replay();
        assert_eq!(replay.ticks, 3);
        assert_eq!(replay.run().last().copied(), Some(own));
        assert!(
            a.tick < 10,
            "the driver ran on after the desync: {}",
            a.tick
        );
        assert!(a.events.iter().any(|e| e.contains("desync")));
    }

    #[test]
    fn a_set_this_peer_refuses_is_a_desync_and_a_lost_peer_is_no_longer_awaited() {
        let (mut a, mut end) = against_the_test();
        end.send(&Message::Set {
            sender: TeamId(1),
            tick: 0,
            commands: vec![Command::new(0, TeamId(1), 0, CommandKind::SetSpeed(99))],
        });
        a.advance(0.1);
        assert!(
            a.desync
                .as_ref()
                .is_some_and(|d| d.report.contains("refuses") && d.own.is_none()),
            "{:?}",
            a.desync
        );

        let (mut a, mut end) = against_the_test();
        for _ in 0..5 {
            a.advance(0.1);
            answer(&mut end);
        }
        let at = a.tick;
        drop(end);
        for _ in 0..10 {
            a.advance(0.1);
        }
        assert!(
            a.tick > at + 5,
            "A stalled on a peer that is gone: {}",
            a.tick
        );
        assert!(a.remote_peers.is_empty());
        assert!(
            a.events.iter().any(|e| e.contains("team 1 left")),
            "{:?}",
            a.events
        );
        assert!(a.desync.is_none());
    }

    #[test]
    fn a_map_with_no_team_for_a_peer_cannot_be_hosted() {
        let map = map_text();
        let fool = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/opposition/fool"
        ));
        assert_eq!(Driver::remote_teams(&map, 0).unwrap(), vec![TeamId(1)]);
        assert!(Driver::remote_teams(&map, 1).unwrap().is_empty());
        let e = match Driver::host(&map, &[&fool], opening(), "127.0.0.1:0", |_| {}) {
            Err(e) => e,
            Ok(_) => panic!("hosted a map with no seat for a peer"),
        };
        assert!(e.contains("no team left"), "{e}");
        // A peer seated nowhere is refused before the match starts.
        let (la, _lb) = net::exchange::Loopback::pair(TeamId(0), TeamId(5));
        let e = match Driver::with_exchange(&map, &[], TeamId(0), opening(), Box::new(la)) {
            Err(e) => e,
            Ok(_) => panic!("seated team 1 nowhere and started anyway"),
        };
        assert!(e.contains("no peer for team(s) 1"), "{e}");
    }

    #[test]
    fn the_replay_of_a_match_reproduces_its_hash() {
        let mut d = driver();
        d.advance(1.0);
        let agreed = d.submit(CommandKind::SetSpeed(20)).unwrap();
        d.advance(1.0);
        assert!(d.tick > agreed, "ran past the command's tick: {}", d.tick);
        let replay = d.replay();
        assert_eq!(replay.ticks, d.tick);
        let back = replay::Replay::from_ron(&replay.to_ron()).unwrap();
        let hashes = back.run();
        assert_eq!(hashes.len() as u64, d.tick);
        assert_eq!(hashes.last().copied(), Some(d.state_hash()));
    }

    #[test]
    fn an_invalid_command_is_refused_at_submission() {
        let mut d = driver();
        assert!(d.submit(CommandKind::SetSpeed(7)).is_err());
        assert!(
            d.submit(CommandKind::Mark {
                at: sim::TilePos::new(99, 0),
                plan: sim::PlanKind::Paint,
                value: None
            })
            .is_err()
        );
        assert!(d.submitted.is_empty());
    }
}
