//! The driver (`docs/06`, The driver): the loop that decides when a tick
//! runs. Plain Rust — no Bevy type — so it is tested headless; `app.rs`
//! calls [`Driver::advance`] once per frame with the wall-clock delta.

use net::CommandLog;
use sim::snapshot::Snapshot;
use sim::{Command, CommandKind, Data, Map, Sim, StepReport, TeamId};
use std::collections::BTreeSet;
use std::path::Path;

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
    pub stall_report: Option<String>,
    pub snapshots: Snapshots,
    pub last_report: StepReport,
    pub ended: Option<Option<TeamId>>,
    /// Commands the player submitted, for the log view.
    pub submitted: Vec<Command>,
    pub map_name: String,
}

impl Driver {
    /// A match on `map`, the player as team 0, and scripted teams from
    /// `opposition` directories in team order for the remaining teams.
    pub fn new(
        map_text: &str,
        opposition: &[&Path],
        player_opening: Vec<CommandKind>,
    ) -> Result<Driver, String> {
        let data = Data::load().map_err(|e| e.to_string())?;
        let map = Map::parse(map_text, &data).map_err(|e| e.to_string())?;
        let teams: Vec<TeamId> = (0..map.teams.len() as u32).map(TeamId).collect();
        let player = TeamId(0);
        let mut log = CommandLog::new(teams.iter().copied());
        let mut local_peers: BTreeSet<TeamId> = BTreeSet::new();
        local_peers.insert(player);
        let mut scripted = Vec::new();
        for (i, dir) in opposition.iter().enumerate() {
            let team = TeamId(i as u32 + 1);
            if !teams.contains(&team) {
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
        let map_name = map.name.clone();
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
            stall_report: None,
            snapshots: Snapshots {
                prev: None,
                cur,
                since: 0.0,
            },
            last_report: StepReport::default(),
            ended: None,
            submitted: Vec::new(),
            map_name,
        })
    }

    /// Submit a command from the player: validated as a peer would, agreed
    /// for `tick + delay`, and forgotten — what the player sees is the
    /// sim's answer, `delay` ticks later. Returns the agreed tick.
    pub fn submit(&mut self, kind: CommandKind) -> Result<u64, String> {
        let tick = self.tick.saturating_add(self.delay);
        let c = Command::new(tick, self.player, 0, kind);
        self.sim.validate(&c)?;
        self.submitted.push(c.clone());
        self.log.push(c);
        Ok(tick)
    }

    /// The player's own state hash, for the display.
    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// Every local peer's set for `tick` is in hand by construction.
    fn ensure_local_sets(&mut self, tick: u64) {
        for peer in self.local_peers.clone() {
            if !self.log.have_all_sets(tick) {
                self.log.submit_set(peer, tick, vec![]);
            }
        }
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
        let cur = self.sim.snapshot();
        let prev = std::mem::replace(&mut self.snapshots.cur, cur);
        self.snapshots.prev = Some(prev);
        self.snapshots.since = 0.0;
    }

    /// Whether every peer's set for `tick` is in hand, submitting the
    /// local peers' first.
    fn ready(&mut self, tick: u64) -> bool {
        self.ensure_local_sets(tick);
        self.log.have_all_sets(tick)
    }

    /// One frame of the driver loop `docs/06` specifies, `dt` real seconds.
    pub fn advance(&mut self, dt: f64) {
        if self.ended.is_some() {
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
            self.stall_report = None;
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
        self.stall_report = None;
    }

    fn stall(&mut self, dt: f64) {
        self.stalled += dt;
        let threshold = if self.speed == 0 {
            self.stall_report_ticks as f64
        } else {
            self.stall_report_ticks as f64 / self.speed as f64
        };
        if self.stalled >= threshold {
            let waiting: Vec<String> = self
                .log_missing(self.tick.saturating_add(1))
                .into_iter()
                .map(|t| format!("team {}", t.0))
                .collect();
            self.stall_report = Some(format!(
                "waiting {:.0}s for tick {}'s set from {}",
                self.stalled,
                self.tick.saturating_add(1),
                waiting.join(", ")
            ));
        }
    }

    /// The peers whose set for `tick` is missing.
    fn log_missing(&self, tick: u64) -> Vec<TeamId> {
        self.log.missing_for(tick)
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
                bundle: bundle_of("while True:\n    try:\n        print('red')\n    except ValueError:\n        wait(5)\n"),
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
        assert!(d.stall_report.is_none());
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
        assert!(d.stall_report.is_none(), "reported too early");
        for _ in 0..30 {
            d.advance(0.11);
        }
        let report = d
            .stall_report
            .clone()
            .expect("a stall report after stall_report_ticks");
        assert!(report.contains("team 7"), "{report}");
        // The sets arrive: the ticks run (a bounded burst per frame, so the
        // stalled seconds catch up) and the report clears.
        for t in 1..=100 {
            d.log.submit_set(TeamId(7), t, vec![]);
        }
        d.advance(0.1);
        assert!(d.tick >= 30, "tick {}", d.tick);
        assert!(d.stall_report.is_none());
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
