//! World state (`docs/02`, `docs/03`): tiles, teams, machines. Plain Rust,
//! `BTreeMap`s, no ECS (CLAUDE.md rule 1). Everything here is in the state
//! hash except the diagnostic log.

use crate::command::{Bundle, PlanKind, TeamId};
use crate::data::Data;
use crate::map::{Height, Map, Slope, Terrain, TilePos, index};
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

/// The value a wire plan to lay one carries (`docs/03`): a wire has no
/// kinds, so there is one.
pub const WIRE: &str = "wire";

/// A team's four plans on one tile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plans {
    pub paint: Option<Plan>,
    pub overlay: Option<Plan>,
    pub building: Option<Plan>,
    /// To lay the team's wire ([`WIRE`]) or to clear it (Q62).
    pub wire: Option<Plan>,
}

impl Plans {
    pub fn is_empty(&self) -> bool {
        self.paint.is_none()
            && self.overlay.is_none()
            && self.building.is_none()
            && self.wire.is_none()
    }

    pub fn of(&self, kind: PlanKind) -> &Option<Plan> {
        match kind {
            PlanKind::Paint => &self.paint,
            PlanKind::Overlay => &self.overlay,
            PlanKind::Building => &self.building,
            PlanKind::Wire => &self.wire,
        }
    }

    pub fn of_mut(&mut self, kind: PlanKind) -> &mut Option<Plan> {
        match kind {
            PlanKind::Paint => &mut self.paint,
            PlanKind::Overlay => &mut self.overlay,
            PlanKind::Building => &mut self.building,
            PlanKind::Wire => &mut self.wire,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub terrain: Terrain,
    /// Its height (Q65), from the map.
    pub height: Height,
    /// The parts of water standing on it (`docs/09`), which only the
    /// water step changes; and a spring's and a sink's rate, from the map.
    pub water: u32,
    pub spring: Option<u32>,
    pub sink: Option<u32>,
    /// A spring's head, and a sink's sill, where they have one
    /// (`docs/09`): each a surface, in parts.
    pub head: Option<u32>,
    pub sill: Option<u32>,
    pub deposit: Option<Deposit>,
    pub paint: Option<String>,
    pub overlay: Option<String>,
    /// The teams with a wire on the tile (Q62): underground, each its
    /// team's own. Changed only by `World::lay_wire`, `remove_wire` and
    /// `clear_wires`, which keep each team's index of its own in step.
    pub wires: BTreeSet<TeamId>,
    /// Per team; absent means no plans.
    pub plans: BTreeMap<TeamId, Plans>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Model {
    Bot,
    Printer,
    Depot,
    Solar,
    Charger,
    Pylon,
    Site,
}

impl Model {
    pub const ALL: [Model; 7] = [
        Model::Bot,
        Model::Printer,
        Model::Depot,
        Model::Solar,
        Model::Charger,
        Model::Pylon,
        Model::Site,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Model::Bot => "bot",
            Model::Printer => "printer",
            Model::Depot => "depot",
            Model::Solar => "solar",
            Model::Charger => "charger",
            Model::Pylon => "pylon",
            Model::Site => "site",
        }
    }

    /// The deployment a machine of the model runs: its name, or none for
    /// a pylon and a site, which run no program (Q42, Q62).
    pub fn deployment(self) -> Option<&'static str> {
        match self {
            Model::Pylon | Model::Site => None,
            m => Some(m.name()),
        }
    }

    /// A model a bot may `build` and a building plan may name: every
    /// building but a site.
    pub fn buildable(self) -> bool {
        !matches!(self, Model::Bot | Model::Site)
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
            "solar" => Some(Model::Solar),
            "charger" => Some(Model::Charger),
            "pylon" => Some(Model::Pylon),
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
    /// Take charge from an adjacent building's battery (Q62).
    Recharge {
        from: EntityId,
    },
    /// Lay or remove the team's wire on a tile (Q62).
    Wire {
        at: TilePos,
    },
    Unwire {
        at: TilePos,
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
    /// The model name whose bundle it runs (Q42); `None` for a site.
    pub deployment: Option<String>,
    pub pos: TilePos,
    pub health: Num,
    /// In the `dying` handler: health frozen, most actions refused.
    pub dying: bool,
    pub action: Option<Action>,
    /// A bot's carried resources, every kind present.
    pub load: BTreeMap<String, Num>,
    /// A bot's color, set by `set_color` (Q43): a paint color, or `None`
    /// as printed. Kept across a redeploy and a fault; `None` on a building.
    pub color: Option<String>,
    /// A building's held resources, every kind present.
    pub store: BTreeMap<String, Num>,
    pub store_capacity: BTreeMap<String, Num>,
    /// For a site: what it becomes, and the construction ticks remaining
    /// once its store is full.
    pub becomes: Option<Model>,
    pub construction: Option<u64>,
    /// A printer's ticks remaining on its print in progress, `0` when not
    /// printing (`docs/02`, Printing). Printing is not an action.
    pub printing: u64,
    /// The charge its battery holds (`docs/08`); zero on a model with none.
    pub charge: Num,
    /// A bot out of power: from a call it could not pay for until its
    /// solar has brought it to the wake level (Q63).
    pub flat: bool,
    /// A bot whose charge is above the low-power level since it last took
    /// `low_power` for falling to it (Q62).
    pub armed: bool,
    /// A building the last power step powered, or left dark; a building
    /// is placed powered, and a site never is.
    pub powered: bool,
    /// The label of the grid the last power step found a building on.
    pub grid: Option<EntityId>,
    /// `low_power` fell due during the machine's own slice, where its
    /// program is out of its body: the interpreter collects it after the
    /// call that caused it (`Host::take_raised`). Never set between calls.
    pub low_power_due: bool,
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

    /// A building the last power step left dark (`docs/08`): its rules,
    /// its program and its senses stop. Never a bot, nor a site, which
    /// has none of the three.
    pub fn is_dark(&self) -> bool {
        self.is_building() && self.model != Model::Site && !self.powered
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
    /// A bot's `color` (Q43): `Some` of its color or `None` on a bot,
    /// absent on a building, as `load` is.
    pub color: Option<Option<String>>,
    pub store: Option<BTreeMap<String, Num>>,
    pub progress: u64,
    /// A printer's `printing`; absent on every other model.
    pub printing: Option<u64>,
    /// The charge its battery holds (`docs/08`).
    pub charge: Num,
    /// A bot's `flat`; absent on a building.
    pub flat: Option<bool>,
    /// A building's `powered` and `grid`; absent on a bot.
    pub powered: Option<bool>,
    pub grid: Option<Option<EntityId>>,
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
            color: m.is_bot().then(|| m.color.clone()),
            store: m.is_building().then(|| m.store.clone()),
            progress: m.progress(),
            printing: (m.model == Model::Printer).then_some(m.printing),
            charge: m.charge,
            flat: m.is_bot().then_some(m.flat),
            powered: m.is_building().then_some(m.powered),
            grid: m.is_building().then_some(m.grid),
        }
    }
}

/// A team's knowledge of one tile (Q21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileMemory {
    pub state: TileState,
    pub seen_at: u64,
    pub terrain: Terrain,
    /// The tile's height as last seen (Q65).
    pub height: Height,
    /// Its water as last seen, and the spring and the sink on it, with
    /// the one's head and the other's sill, which never change once seen
    /// (`docs/09`).
    pub water: u32,
    pub spring: Option<u32>,
    pub sink: Option<u32>,
    pub head: Option<u32>,
    pub sill: Option<u32>,
    pub deposit: Option<BTreeMap<String, Num>>,
    pub paint: Option<String>,
    pub overlay: Option<String>,
    /// The teams with a wire on it, as last seen (Q62).
    pub wires: BTreeSet<TeamId>,
    pub building: Option<MachineRecord>,
}

/// A team's deployments: one per model that runs a program (Q42, Q62).
pub const DEPLOYMENTS: [&str; 5] = ["bot", "printer", "depot", "solar", "charger"];

/// One grid's figures as a power step kept them (`docs/08`, The power
/// step): its label, and four sums over its buildings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridFigures {
    pub grid: EntityId,
    pub generation: Num,
    /// Every need, powered or dark.
    pub drain: Num,
    /// The charge held after the step.
    pub stored: Num,
    pub capacity: Num,
}

/// One team's machines as one pass over every machine found them
/// ([`World::rosters`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    /// Every machine of the team, in id order.
    pub machines: Vec<EntityId>,
    pub bots: usize,
    pub printers: usize,
    /// Its printers with a print in progress.
    pub printing: usize,
}

/// Whether `name` is a deployment a team holds.
pub fn is_deployment_name(name: &str) -> bool {
    DEPLOYMENTS.contains(&name)
}

pub struct Team {
    pub id: TeamId,
    pub deployments: BTreeMap<String, Deployment>,
    pub out: bool,
    pub resigned: bool,
    /// Per tile index.
    pub memory: Vec<TileMemory>,
    /// The tile indices the team saw at its last vision pass: exactly the
    /// tiles its memory holds as `Visible`, since that pass is the only
    /// writer of a tile's state. Kept so the pass demotes them without
    /// scanning the map for them (I9); derived from the memory, and no
    /// part of the hash beyond it.
    pub seen: Vec<usize>,
    /// Per tile index, whether the team knows the tile: exactly whether
    /// its memory's state is other than `Unknown`. Set where the vision
    /// pass first sees a tile, and never cleared, since nothing forgets
    /// one. Kept so the state hash writes an unknown tile's byte without
    /// reading its memory, which is a wide record (I12); derived from
    /// the memory, and no part of the hash beyond it.
    pub known: Vec<bool>,
    /// The starting tiles, for scripts.
    pub starts: Vec<TilePos>,
    /// The printer that began the team's last print; `0`, which no entity
    /// is, before the first (`docs/02`, Printing).
    pub print_turn: EntityId,
    /// The team's grids as its last power step kept them, ascending label.
    pub grids: Vec<GridFigures>,
    /// The tiles carrying the team's wire: an index of `Tile::wires`,
    /// kept by the three methods that change it, and no part of the hash
    /// beyond the tiles.
    pub wire_tiles: BTreeSet<TilePos>,
    /// Those of them the last power step found on a grid, for the
    /// snapshot; derived, never hashed.
    pub live_wires: BTreeSet<TilePos>,
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
    /// Machines whose `death` epilogue ran this tick, removed in step 4.
    pub dead: BTreeSet<EntityId>,
    /// The driver's speed, carried for the snapshot only.
    pub speed: u64,
    /// Ids for the dicts and lists the sim hands programs.
    pub next_host_object: u64,
    /// The program a deployment with no bundle runs.
    pub empty_program: Program,
    /// The tiles each team sees now, kept between queries
    /// (`sense::SightCache`); derived, never hashed.
    pub sight: crate::sense::SightCache,
    /// Counts the changes a team's sight depends on: every machine
    /// placed, removed or moved. A site becoming its building is a
    /// removal and a placement, and a team going out is its machines'
    /// removal. The sight cache holds the count it was built at, so
    /// asking whether it still stands is one comparison (I9). Bumped by
    /// `place`, `remove`, `move_machine` and `remove_team`, which are
    /// the only code that may do those things; derived, never hashed.
    pub(crate) generation: u64,
    /// The water under the whole map, in parts (`docs/09`): what springs
    /// draw on and sinks and drying fill. World state, in the hash.
    pub ground_water: u64,
    /// The tiles holding a spring, and those holding a sink, in
    /// row-then-column order; from the map, and fixed.
    pub(crate) springs: Vec<usize>,
    pub(crate) sinks: Vec<usize>,
    /// The tiles whose water the next water step must look at
    /// (`World::stir`); derived, never hashed.
    pub(crate) stirred: BTreeSet<usize>,
}

impl World {
    pub fn new(map: Map, data: Rc<Data>, costs: Rc<Costs>, limits: Rc<Limits>) -> World {
        let tiles: Vec<Tile> = map
            .tiles
            .iter()
            .map(|t| Tile {
                terrain: t.terrain,
                height: t.height,
                water: t.water,
                spring: t.spring,
                sink: t.sink,
                head: t.head,
                sill: t.sill,
                deposit: t.deposit.as_ref().map(|d| Deposit {
                    amount: d.amount.clone(),
                    cap: d.cap.clone(),
                    regrowth: d.regrowth.clone(),
                }),
                paint: None,
                overlay: None,
                wires: BTreeSet::new(),
                plans: BTreeMap::new(),
            })
            .collect();
        let empty_program =
            Program::load(&[("main.py", "")], &limits).expect("the empty program loads");
        let in_order = |has: fn(&Tile) -> bool| -> Vec<usize> {
            map.walk()
                .map(|(_, idx)| idx)
                .filter(|idx| has(&tiles[*idx]))
                .collect()
        };
        let springs = in_order(|t| t.spring.is_some());
        let sinks = in_order(|t| t.sink.is_some());
        // Every tile that starts with water is looked at in the first
        // water step, with the tiles beside it.
        let wet_at_start: Vec<usize> = (0..tiles.len()).filter(|i| tiles[*i].water > 0).collect();
        let ground_water = map.ground_water;
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
            dead: BTreeSet::new(),
            speed,
            next_host_object: 0x8000_0000_0000_0000,
            empty_program,
            sight: crate::sense::SightCache::default(),
            generation: 0,
            ground_water,
            springs,
            sinks,
            stirred: BTreeSet::new(),
        };
        for idx in wet_at_start {
            world.stir(idx);
        }
        let team_starts: Vec<Vec<TilePos>> = world.map.teams.clone();
        for (i, starts) in team_starts.iter().enumerate() {
            let id = TeamId(i as u32);
            // One deployment per model, from the start (Q42).
            let deployments: BTreeMap<String, Deployment> = DEPLOYMENTS
                .iter()
                .map(|d| (d.to_string(), Deployment::default()))
                .collect();
            let memory: Vec<TileMemory> = world
                .tiles
                .iter()
                .map(|t| TileMemory {
                    state: TileState::Unknown,
                    seen_at: 0,
                    terrain: t.terrain,
                    height: Height::default(),
                    water: 0,
                    spring: None,
                    sink: None,
                    head: None,
                    sill: None,
                    deposit: None,
                    paint: None,
                    overlay: None,
                    wires: BTreeSet::new(),
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
                    known: vec![false; memory.len()],
                    memory,
                    seen: Vec::new(),
                    starts: starts.clone(),
                    print_turn: EntityId(0),
                    grids: Vec::new(),
                    wire_tiles: BTreeSet::new(),
                    live_wires: BTreeSet::new(),
                },
            );
            for &s in starts {
                let printer = world.place(Model::Printer, id, s, Some("printer".to_string()));
                // A printer the map places starts full (`docs/08`).
                let full = world.data.model("printer").battery;
                world.machines.get_mut(&printer).expect("printer").charge = full;
            }
        }
        // The match is built with its grids traced, every building
        // powered and the figures kept, no charge having moved.
        let teams: Vec<TeamId> = world.teams.keys().copied().collect();
        let rosters = world.rosters();
        for team in teams {
            let own = rosters.get(&team).map_or(&[][..], |r| &r.machines);
            world.trace_and_keep(team, own);
        }
        world
    }

    /// A copy that shares nothing this world will change, for a saved
    /// state and a fork (`docs/06`, Forks): every machine's program copied
    /// as deeply as `lang` copies one, every team and tile its own. What
    /// cannot change is shared — the tables and each loaded bundle's code.
    /// Written out field by field so that a field added to the world, a
    /// machine or a team fails to compile here until it is copied too.
    pub fn copy(&self) -> World {
        let machines = self
            .machines
            .iter()
            .map(|(id, m)| {
                let copy = Machine {
                    id: m.id,
                    name: m.name.clone(),
                    model: m.model,
                    team: m.team,
                    deployment: m.deployment.clone(),
                    pos: m.pos,
                    health: m.health,
                    dying: m.dying,
                    action: m.action.clone(),
                    load: m.load.clone(),
                    color: m.color.clone(),
                    store: m.store.clone(),
                    store_capacity: m.store_capacity.clone(),
                    becomes: m.becomes,
                    construction: m.construction,
                    printing: m.printing,
                    charge: m.charge,
                    flat: m.flat,
                    armed: m.armed,
                    powered: m.powered,
                    grid: m.grid,
                    low_power_due: m.low_power_due,
                    program: m.program.as_ref().map(|p| p.deep_copy()),
                    log: m.log.clone(),
                };
                (*id, copy)
            })
            .collect();
        let teams = self
            .teams
            .iter()
            .map(|(id, t)| {
                let copy = Team {
                    id: t.id,
                    deployments: t.deployments.clone(),
                    out: t.out,
                    resigned: t.resigned,
                    memory: t.memory.clone(),
                    seen: t.seen.clone(),
                    known: t.known.clone(),
                    starts: t.starts.clone(),
                    print_turn: t.print_turn,
                    grids: t.grids.clone(),
                    wire_tiles: t.wire_tiles.clone(),
                    live_wires: t.live_wires.clone(),
                };
                (*id, copy)
            })
            .collect();
        World {
            tick: self.tick,
            map: self.map.clone(),
            data: self.data.clone(),
            costs: self.costs.clone(),
            limits: self.limits.clone(),
            tiles: self.tiles.clone(),
            machines,
            next_id: self.next_id,
            teams,
            sounds_prev: self.sounds_prev.clone(),
            sounds_now: self.sounds_now.clone(),
            pending_damage: self.pending_damage.clone(),
            dead: self.dead.clone(),
            speed: self.speed,
            next_host_object: self.next_host_object,
            empty_program: self.empty_program.clone(),
            sight: self.sight.clone(),
            generation: self.generation,
            ground_water: self.ground_water,
            springs: self.springs.clone(),
            sinks: self.sinks.clone(),
            stirred: self.stirred.clone(),
        }
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

    /// How the ground lies from `from` to `to` (`docs/03`), or `None` if
    /// either is off the map.
    pub fn slope(&self, from: TilePos, to: TilePos) -> Option<Slope> {
        let (a, b) = (self.tile(from)?, self.tile(to)?);
        Some(a.height.slope_to(b.height, self.data.world.height.step))
    }

    /// Whether a bot on `from` may step onto the adjacent `to` (`docs/03`,
    /// A step): passable terrain, ground it can step across, and water
    /// no deeper than it wades.
    pub fn steps(&self, from: TilePos, to: TilePos) -> bool {
        self.passable(to)
            && self.slope(from, to).is_some_and(Slope::is_step)
            && self
                .tile(to)
                .is_some_and(|t| t.water <= self.data.world.water.wade)
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
                    && t.spring.is_none()
                    && t.sink.is_none()
                    && t.water < self.data.world.water.wet
                    && !self
                        .machines
                        .values()
                        .any(|m| m.pos == p && m.is_building())
            }
            None => false,
        }
    }

    /// Every team's machines from one pass over them all (T41): what the
    /// steps that go team by team, and the state hash, would otherwise
    /// each scan every machine for. A team with no machine has no entry.
    pub fn rosters(&self) -> BTreeMap<TeamId, Roster> {
        let mut out: BTreeMap<TeamId, Roster> = BTreeMap::new();
        for m in self.machines.values() {
            let r = out.entry(m.team).or_default();
            r.machines.push(m.id);
            match m.model {
                Model::Bot => r.bots = r.bots.saturating_add(1),
                Model::Printer => {
                    r.printers = r.printers.saturating_add(1);
                    if m.printing > 0 {
                        r.printing = r.printing.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The bots `printers` printers support (`docs/02`, Printing).
    pub fn cap_of(&self, printers: usize) -> usize {
        (printers as u64).saturating_mul(self.data.machines.bots_per_printer) as usize
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
        self.cap_of(self.printers_of(team))
    }

    /// The team's standing buildings of `model` and its sites becoming
    /// one: the `n` of a new site's price (Q42).
    pub fn count_toward_price(&self, team: TeamId, model: Model) -> u64 {
        self.machines
            .values()
            .filter(|m| {
                m.team == team
                    && (m.model == model || (m.model == Model::Site && m.becomes == Some(model)))
            })
            .count() as u64
    }

    /// A new site's store capacity for `model`: `cost + n × cost_step` per
    /// kind (Q42).
    pub fn price(&self, team: TeamId, model: Model) -> BTreeMap<String, Num> {
        let md = self.data.model(model.name());
        let n = Num::from_int(i128::from(self.count_toward_price(team, model))).unwrap_or(Num::MAX);
        self.data
            .machines
            .kinds
            .iter()
            .map(|k| {
                let cost = md.cost.get(k).copied().unwrap_or(Num::ZERO);
                let step = md.cost_step.get(k).copied().unwrap_or(Num::ZERO);
                let extra = step.checked_mul(n).unwrap_or(Num::MAX);
                (k.clone(), cost.checked_add(extra).unwrap_or(Num::MAX))
            })
            .collect()
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
        // Every machine is `<model><id>`, bots included (Q42).
        let name = format!("{}{}", model.name(), id.0);
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
                color: None,
                store,
                store_capacity,
                becomes: None,
                construction: None,
                printing: 0,
                // A bot is printed full and armed; a building is placed
                // empty, and powered until a power step finds otherwise.
                charge: if model == Model::Bot {
                    md.battery
                } else {
                    Num::ZERO
                },
                flat: false,
                armed: model == Model::Bot,
                powered: model != Model::Bot && model != Model::Site,
                grid: None,
                low_power_due: false,
                program,
                log: Vec::new(),
            },
        );
        self.sight_changed();
        id
    }

    /// Remove a machine from the world: the one way one leaves it, so
    /// the sight cache hears of it.
    pub fn remove(&mut self, id: EntityId) -> Option<Machine> {
        let gone = self.machines.remove(&id);
        if gone.is_some() {
            self.sight_changed();
        }
        gone
    }

    /// Stand a machine on `to`: the one place a position changes, so the
    /// sight cache hears of it.
    pub fn move_machine(&mut self, id: EntityId, to: TilePos) {
        if let Some(m) = self.machines.get_mut(&id) {
            m.pos = to;
            self.sight_changed();
        }
    }

    /// Remove every machine of `team`, as a team that goes out loses
    /// them (`docs/06`, step 5).
    pub fn remove_team(&mut self, team: TeamId) {
        self.machines.retain(|_, m| m.team != team);
        self.sight_changed();
    }

    /// What some team sees may have changed (`generation`).
    pub(crate) fn sight_changed(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Set a machine's health: the one place health changes, so a printer
    /// that falls to zero or below loses its print there (`docs/02`,
    /// Printing), whatever dealt the damage.
    pub fn set_health(&mut self, id: EntityId, health: Num) {
        if let Some(m) = self.machines.get_mut(&id) {
            m.health = health;
            if m.model == Model::Printer && health <= Num::ZERO {
                m.printing = 0;
            }
        }
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

    /// Whether `team` may place a `plan` on `p` now (Q49): a paint or
    /// overlay plan, a value or a clear, only on ground the team has seen —
    /// never on rock or water, and never on a tile still unknown to it,
    /// which is checked first, so a refusal tells it nothing it could not
    /// see. A building plan may go anywhere on the map, as before. It
    /// depends on what the team has seen, so `Sim::validate` does not call
    /// it; a `Mark` that fails it on its tick is dropped, and a renderer
    /// asks it before submitting.
    pub fn may_plan(&self, team: TeamId, p: TilePos, plan: PlanKind) -> Result<(), String> {
        let what = match plan {
            PlanKind::Building => return Ok(()),
            PlanKind::Paint => "paint",
            PlanKind::Overlay => "an overlay",
            PlanKind::Wire => "a wire",
        };
        let idx = self
            .tile_index(p)
            .ok_or_else(|| format!("({}, {}) is off the map", p.x, p.y))?;
        let seen = self
            .teams
            .get(&team)
            .is_some_and(|t| t.memory[idx].state != TileState::Unknown);
        if !seen {
            return Err(format!(
                "({}, {}) is unknown to your team: only ground it has seen takes {what}",
                p.x, p.y
            ));
        }
        let terrain = self.tiles[idx].terrain;
        if terrain != Terrain::Ground {
            return Err(format!(
                "({}, {}) is {}: only ground takes {what}",
                p.x,
                p.y,
                terrain.name()
            ));
        }
        Ok(())
    }

    /// Plans of `team` on `p`, if any.
    pub fn plans(&self, team: TeamId, p: TilePos) -> Option<&Plans> {
        self.tile(p).and_then(|t| t.plans.get(&team))
    }

    pub fn plans_mut(&mut self, team: TeamId, p: TilePos) -> Option<&mut Plans> {
        self.tile_mut(p).map(|t| t.plans.entry(team).or_default())
    }

    /// Whether `team`'s wire lies on `p`.
    pub fn has_wire(&self, team: TeamId, p: TilePos) -> bool {
        self.tile(p).is_some_and(|t| t.wires.contains(&team))
    }

    /// Lay `team`'s wire on `p`; `false` if it was there or `p` is no tile.
    pub fn lay_wire(&mut self, team: TeamId, p: TilePos) -> bool {
        let laid = self.tile_mut(p).is_some_and(|t| t.wires.insert(team));
        if laid && let Some(t) = self.teams.get_mut(&team) {
            t.wire_tiles.insert(p);
        }
        laid
    }

    /// Remove `team`'s wire from `p`; `false` if there was none.
    pub fn remove_wire(&mut self, team: TeamId, p: TilePos) -> bool {
        let gone = self.tile_mut(p).is_some_and(|t| t.wires.remove(&team));
        if gone && let Some(t) = self.teams.get_mut(&team) {
            t.wire_tiles.remove(&p);
            t.live_wires.remove(&p);
        }
        gone
    }

    /// Clear every wire of `team`, as a team that goes out loses them
    /// (`docs/06`, step 5).
    pub fn clear_wires(&mut self, team: TeamId) {
        let Some(t) = self.teams.get_mut(&team) else {
            return;
        };
        let tiles = std::mem::take(&mut t.wire_tiles);
        t.live_wires.clear();
        for p in tiles {
            if let Some(tile) = self.tile_mut(p) {
                tile.wires.remove(&team);
            }
        }
    }

    /// A bot's charge changed (`docs/08`, Low power): at or below the
    /// level an armed bot takes `low_power` and is no longer armed; above
    /// it, a bot that was not is armed again. Called after every change.
    pub fn charge_changed(&mut self, id: EntityId) {
        let level = self.data.machines.low_power_charge();
        let Some(m) = self.machines.get_mut(&id) else {
            return;
        };
        if m.model != Model::Bot {
            return;
        }
        if m.armed && m.charge <= level {
            m.armed = false;
            match m.program.as_mut() {
                Some(p) => p.raise(lang::Interrupt::LowPower),
                // Inside its own slice the program is out of the body.
                None => m.low_power_due = true,
            }
        } else if !m.armed && m.charge > level {
            m.armed = true;
        }
    }

    /// Drop empty plan entries so the hash and the snapshot see none.
    pub fn tidy_plans(&mut self, p: TilePos) {
        if let Some(t) = self.tile_mut(p) {
            t.plans.retain(|_, v| !v.is_empty());
        }
    }
}
