//! World state (`docs/02`, `docs/03`): tiles, teams, machines. Plain Rust,
//! `BTreeMap`s, no ECS (CLAUDE.md rule 1). Everything here is in the state
//! hash except the diagnostic log.

use crate::command::{Bundle, TeamId};
use crate::data::Data;
use crate::map::{Map, Terrain, TilePos, index};
use lang::{Costs, Limits, Num, Program};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct EntityId(pub u32);

/// A deposit on a tile: per resource kind, an amount, a cap and a regrowth
/// per tick (`docs/03`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deposit {
    pub amount: BTreeMap<String, Num>,
    pub cap: BTreeMap<String, Num>,
    pub regrowth: BTreeMap<String, Num>,
}

/// A plan a player placed: a value to realise, or a plan to clear the slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Set(String),
    Clear,
}

/// A team's three plans on one tile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plans {
    pub paint: Option<Plan>,
    pub overlay: Option<Plan>,
    pub building: Option<Plan>,
}

impl Plans {
    pub fn is_empty(&self) -> bool {
        self.paint.is_none() && self.overlay.is_none() && self.building.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub terrain: Terrain,
    pub deposit: Option<Deposit>,
    pub paint: Option<String>,
    pub overlay: Option<String>,
    /// Per team; absent means no plans.
    pub plans: BTreeMap<TeamId, Plans>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Model {
    Bot,
    Printer,
    Depot,
    Site,
}

impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Model::Bot => "bot",
            Model::Printer => "printer",
            Model::Depot => "depot",
            Model::Site => "site",
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            Model::Bot => "bot",
            _ => "building",
        }
    }

    pub fn from_name(s: &str) -> Option<Model> {
        match s {
            "bot" => Some(Model::Bot),
            "printer" => Some(Model::Printer),
            "depot" => Some(Model::Depot),
            "site" => Some(Model::Site),
            _ => None,
        }
    }
}

/// What an action does when it completes (`docs/02`, Actions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Move {
        to: TilePos,
    },
    Pick {
        kind: String,
        from: PickSource,
    },
    Drop {
        kind: String,
        into: EntityId,
    },
    Print {
        color: String,
    },
    /// Place a site for `model` at `at`; `remove` first if a building is to
    /// be deconstructed on the way.
    Build {
        model: Model,
        at: TilePos,
        remove: Option<EntityId>,
    },
    Paint {
        at: TilePos,
        slot: Slot,
        value: Option<String>,
    },
    Deconstruct {
        target: EntityId,
    },
    Attack {
        target: EntityId,
    },
    Convert {
        target: EntityId,
    },
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Paint,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickSource {
    Deposit(TilePos),
    Depot(EntityId),
}

/// An action in progress: its `busy` name, the ticks remaining, its effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub name: &'static str,
    pub remaining: u64,
    pub effect: Effect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub tick: u64,
    pub level: String,
    pub text: String,
}

/// A machine's body: what `docs/02` calls its attributes, its action, and
/// its program. A site has no program.
pub struct Machine {
    pub id: EntityId,
    pub name: String,
    pub model: Model,
    pub team: TeamId,
    /// The color or model name whose bundle it runs; `None` for a site.
    pub deployment: Option<String>,
    pub pos: TilePos,
    pub health: Num,
    /// In the `dying` handler: health frozen, most actions refused.
    pub dying: bool,
    pub action: Option<Action>,
    /// A bot's carried resources, every kind present.
    pub load: BTreeMap<String, Num>,
    /// A building's held resources, every kind present.
    pub store: BTreeMap<String, Num>,
    pub store_capacity: BTreeMap<String, Num>,
    /// For a site: what it becomes, and the construction ticks remaining
    /// once its store is full.
    pub becomes: Option<Model>,
    pub construction: Option<u64>,
    pub program: Option<lang::Machine>,
    /// The diagnostic log, this tick's entries; not world state.
    pub log: Vec<LogEntry>,
}

impl Machine {
    pub fn is_bot(&self) -> bool {
        self.model == Model::Bot
    }

    pub fn is_building(&self) -> bool {
        self.model != Model::Bot
    }

    pub fn busy(&self) -> Option<&'static str> {
        match &self.action {
            Some(a) => Some(a.name),
            None if self.construction.is_some() => Some("build"),
            None => None,
        }
    }

    pub fn progress(&self) -> u64 {
        match &self.action {
            Some(a) => a.remaining,
            None => self.construction.unwrap_or(0),
        }
    }

    /// The tiles it occupies: its own, and a move's target while moving.
    pub fn occupies(&self, p: TilePos) -> bool {
        if self.pos == p {
            return true;
        }
        matches!(&self.action, Some(Action { effect: Effect::Move { to }, .. }) if *to == p)
    }
}

/// A bundle written into a deployment slot, loaded.
#[derive(Clone)]
pub struct LoadedBundle {
    pub files: Bundle,
    pub program: Program,
    pub version: u64,
}

#[derive(Clone, Default)]
pub struct Deployment {
    pub bundle: Option<LoadedBundle>,
    /// A deploy that arrived while the slot was locked; applied when it
    /// unlocks (`docs/02`, Locking).
    pub pending: Option<LoadedBundle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileState {
    Unknown,
    Visible,
    Remembered,
}

/// A machine's attributes as a sighting or a remembered tile carries them
/// (`docs/02`, A machine's attributes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineRecord {
    pub id: EntityId,
    pub name: String,
    pub kind: &'static str,
    pub model: Model,
    pub team: TeamId,
    pub deployment: Option<String>,
    pub pos: TilePos,
    pub health: Num,
    pub busy: Option<&'static str>,
    pub load: Option<BTreeMap<String, Num>>,
    pub store: Option<BTreeMap<String, Num>>,
    pub progress: u64,
}

impl MachineRecord {
    pub fn of(m: &Machine) -> MachineRecord {
        MachineRecord {
            id: m.id,
            name: m.name.clone(),
            kind: m.model.kind(),
            model: m.model,
            team: m.team,
            deployment: m.deployment.clone(),
            pos: m.pos,
            health: m.health,
            busy: m.busy(),
            load: m.is_bot().then(|| m.load.clone()),
            store: m.is_building().then(|| m.store.clone()),
            progress: m.progress(),
        }
    }
}

/// A team's knowledge of one tile (Q21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileMemory {
    pub state: TileState,
    pub seen_at: u64,
    pub terrain: Terrain,
    pub deposit: Option<BTreeMap<String, Num>>,
    pub paint: Option<String>,
    pub overlay: Option<String>,
    pub building: Option<MachineRecord>,
}

pub struct Team {
    pub id: TeamId,
    pub deployments: BTreeMap<String, Deployment>,
    pub out: bool,
    pub resigned: bool,
    /// Per tile index.
    pub memory: Vec<TileMemory>,
    /// The starting tiles, for scripts.
    pub starts: Vec<TilePos>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    pub cause: String,
    pub pos: TilePos,
    pub loudness: i64,
    pub tick: u64,
}

pub struct World {
    pub tick: u64,
    pub map: Map,
    pub data: Rc<Data>,
    pub costs: Rc<Costs>,
    pub limits: Rc<Limits>,
    pub tiles: Vec<Tile>,
    pub machines: BTreeMap<EntityId, Machine>,
    next_id: u32,
    pub teams: BTreeMap<TeamId, Team>,
    /// Sounds of the previous tick, what `hear()` returns; and this tick's.
    pub sounds_prev: Vec<Sound>,
    pub sounds_now: Vec<Sound>,
    /// Attack hits landed this tick, summed per target in step 4.
    pub pending_damage: BTreeMap<EntityId, Num>,
    /// Printers converted this tick: a second conversion is a no-op (Q30).
    pub converted_this_tick: BTreeSet<EntityId>,
    /// Machines whose `death` epilogue ran this tick, removed in step 4.
    pub dead: BTreeSet<EntityId>,
    /// The driver's speed, carried for the snapshot only.
    pub speed: u64,
    /// Ids for the dicts and lists the sim hands programs.
    pub next_host_object: u64,
    /// The program a deployment with no bundle runs.
    pub empty_program: Program,
}

impl World {
    pub fn new(map: Map, data: Rc<Data>, costs: Rc<Costs>, limits: Rc<Limits>) -> World {
        let tiles: Vec<Tile> = map
            .tiles
            .iter()
            .map(|t| Tile {
                terrain: t.terrain,
                deposit: t.deposit.as_ref().map(|d| Deposit {
                    amount: d.amount.clone(),
                    cap: d.cap.clone(),
                    regrowth: d.regrowth.clone(),
                }),
                paint: None,
                overlay: None,
                plans: BTreeMap::new(),
            })
            .collect();
        let empty_program =
            Program::load(&[("main.py", "")], &limits).expect("the empty program loads");
        let speed = map.speed;
        let mut world = World {
            tick: 0,
            map,
            data,
            costs,
            limits,
            tiles,
            machines: BTreeMap::new(),
            next_id: 1,
            teams: BTreeMap::new(),
            sounds_prev: Vec::new(),
            sounds_now: Vec::new(),
            pending_damage: BTreeMap::new(),
            converted_this_tick: BTreeSet::new(),
            dead: BTreeSet::new(),
            speed,
            next_host_object: 0x8000_0000_0000_0000,
            empty_program,
        };
        let team_starts: Vec<Vec<TilePos>> = world.map.teams.clone();
        for (i, starts) in team_starts.iter().enumerate() {
            let id = TeamId(i as u32);
            let mut deployments = BTreeMap::new();
            for c in &world.data.machines.colors {
                deployments.insert(c.clone(), Deployment::default());
            }
            deployments.insert("printer".to_string(), Deployment::default());
            deployments.insert("depot".to_string(), Deployment::default());
            let memory = world
                .tiles
                .iter()
                .map(|t| TileMemory {
                    state: TileState::Unknown,
                    seen_at: 0,
                    terrain: t.terrain,
                    deposit: None,
                    paint: None,
                    overlay: None,
                    building: None,
                })
                .collect();
            world.teams.insert(
                id,
                Team {
                    id,
                    deployments,
                    out: false,
                    resigned: false,
                    memory,
                    starts: starts.clone(),
                },
            );
            for &s in starts {
                world.place(Model::Printer, id, s, Some("printer".to_string()));
            }
        }
        world
    }

    pub fn alloc_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("entity id space exhausted");
        id
    }

    pub fn next_id_value(&self) -> u32 {
        self.next_id
    }

    pub fn host_object_id(&mut self) -> u64 {
        let id = self.next_host_object;
        self.next_host_object = self.next_host_object.wrapping_add(1);
        id
    }

    pub fn tile_index(&self, p: TilePos) -> Option<usize> {
        index(p, self.map.width, self.map.height)
    }

    pub fn tile(&self, p: TilePos) -> Option<&Tile> {
        self.tile_index(p).map(|i| &self.tiles[i])
    }

    pub fn tile_mut(&mut self, p: TilePos) -> Option<&mut Tile> {
        self.tile_index(p).map(move |i| &mut self.tiles[i])
    }

    pub fn passable(&self, p: TilePos) -> bool {
        self.tile(p)
            .is_some_and(|t| self.data.world.terrain[t.terrain.name()].passable)
    }

    pub fn blocks_sight(&self, p: TilePos) -> bool {
        self.tile(p)
            .is_some_and(|t| self.data.world.terrain[t.terrain.name()].blocks_sight)
    }

    /// The machine on a tile, if any (`docs/02`, Occupancy).
    pub fn machine_at(&self, p: TilePos) -> Option<EntityId> {
        self.machines.values().find(|m| m.pos == p).map(|m| m.id)
    }

    /// Whether any machine occupies the tile, a move's target included.
    pub fn occupied(&self, p: TilePos) -> bool {
        self.machines.values().any(|m| m.occupies(p))
    }

    /// Buildable (`docs/03`): ground, no deposit, no building.
    pub fn buildable(&self, p: TilePos) -> bool {
        match self.tile(p) {
            Some(t) => {
                t.terrain == Terrain::Ground
                    && t.deposit.is_none()
                    && !self
                        .machines
                        .values()
                        .any(|m| m.pos == p && m.is_building())
            }
            None => false,
        }
    }

    pub fn printers_of(&self, team: TeamId) -> usize {
        self.machines
            .values()
            .filter(|m| m.team == team && m.model == Model::Printer)
            .count()
    }

    pub fn bots_of(&self, team: TeamId) -> usize {
        self.machines
            .values()
            .filter(|m| m.team == team && m.model == Model::Bot)
            .count()
    }

    pub fn bot_cap(&self, team: TeamId) -> usize {
        (self.printers_of(team) as u64).saturating_mul(self.data.machines.bots_per_printer) as usize
    }

    /// The colors a team may print and deploy to: one per printer, in
    /// order (Q24).
    pub fn unlocked_colors(&self, team: TeamId) -> Vec<String> {
        let n = self.printers_of(team).min(self.data.machines.colors.len());
        self.data.machines.colors[..n].to_vec()
    }

    pub fn is_unlocked(&self, team: TeamId, deployment: &str) -> bool {
        deployment == "printer"
            || deployment == "depot"
            || self.unlocked_colors(team).iter().any(|c| c == deployment)
    }

    /// The program a deployment currently runs.
    pub fn program_of(&self, team: TeamId, deployment: &str) -> Program {
        self.teams
            .get(&team)
            .and_then(|t| t.deployments.get(deployment))
            .and_then(|d| d.bundle.as_ref())
            .map(|b| b.program.clone())
            .unwrap_or_else(|| self.empty_program.clone())
    }

    /// Create a machine of `model` for `team` on `pos`, running its
    /// deployment's bundle (or none, for a site). Returns its id.
    pub fn place(
        &mut self,
        model: Model,
        team: TeamId,
        pos: TilePos,
        deployment: Option<String>,
    ) -> EntityId {
        let id = self.alloc_id();
        let md = self.data.model(model.name()).clone();
        let kinds = self.data.machines.kinds.clone();
        let zero = |_: &String| Num::ZERO;
        let load: BTreeMap<String, Num> = kinds.iter().map(|k| (k.clone(), zero(k))).collect();
        let store = load.clone();
        let store_capacity: BTreeMap<String, Num> = kinds
            .iter()
            .map(|k| (k.clone(), md.store.get(k).copied().unwrap_or(Num::ZERO)))
            .collect();
        let name = format!("{}{}", deployment.as_deref().unwrap_or(model.name()), id.0);
        let program = deployment.as_ref().map(|d| {
            let p = self.program_of(team, d);
            let mut m = lang::Machine::new(&p, self.costs.clone(), self.limits.clone());
            m.tick = self.tick;
            m
        });
        self.machines.insert(
            id,
            Machine {
                id,
                name,
                model,
                team,
                deployment,
                pos,
                health: md.health,
                dying: false,
                action: None,
                load,
                store,
                store_capacity,
                becomes: None,
                construction: None,
                program,
                log: Vec::new(),
            },
        );
        id
    }

    /// Emit a sound at `pos` (`docs/02`, Sounds).
    pub fn sound(&mut self, cause: &str, pos: TilePos) {
        let loudness = self.data.machines.loudness.get(cause).copied().unwrap_or(0);
        self.sounds_now.push(Sound {
            cause: cause.to_string(),
            pos,
            loudness,
            tick: self.tick,
        });
    }

    /// Plans of `team` on `p`, if any.
    pub fn plans(&self, team: TeamId, p: TilePos) -> Option<&Plans> {
        self.tile(p).and_then(|t| t.plans.get(&team))
    }

    pub fn plans_mut(&mut self, team: TeamId, p: TilePos) -> Option<&mut Plans> {
        self.tile_mut(p).map(|t| t.plans.entry(team).or_default())
    }

    /// Drop empty plan entries so the hash and the snapshot see none.
    pub fn tidy_plans(&mut self, p: TilePos) {
        if let Some(t) = self.tile_mut(p) {
            t.plans.retain(|_, v| !v.is_empty());
        }
    }
}
