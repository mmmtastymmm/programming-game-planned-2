//! The tuning tables behind `docs/02` and `docs/03`: `data/machines.toml`,
//! `data/world.toml` and `data/net.toml`, compiled in. Every number is a
//! `num`, read as an integer or as a string holding a `num` literal — a
//! TOML float is refused, since a float never enters `sim` (CLAUDE.md
//! rule 2). Every key is hash-affecting.

use lang::Num;
use std::collections::BTreeMap;

pub const MACHINES_TOML: &str = include_str!("../../../data/machines.toml");
pub const WORLD_TOML: &str = include_str!("../../../data/world.toml");
pub const NET_TOML: &str = include_str!("../../../data/net.toml");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    pub file: &'static str,
    pub message: String,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for DataError {}

type D<T> = Result<T, DataError>;

/// One model's row in `machines.toml`. A field a model lacks is zero or
/// empty; `docs/02`'s tables say which apply to which.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelData {
    pub vision: i64,
    pub hearing: i64,
    pub health: Num,
    pub death_threshold: Num,
    /// A bot's load capacity per kind.
    pub capacity: BTreeMap<String, Num>,
    /// A building's store capacity per kind.
    pub store: BTreeMap<String, Num>,
    /// What building it costs, per kind (a site's store capacity).
    pub cost: BTreeMap<String, Num>,
    pub pick_rate: BTreeMap<String, Num>,
    pub move_ticks: u64,
    pub pick_ticks: u64,
    pub drop_ticks: u64,
    pub attack_ticks: u64,
    pub convert_ticks: u64,
    pub print_ticks: u64,
    pub build_ticks: u64,
    pub deconstruct_ticks: u64,
    pub paint_ticks: u64,
    pub overlay_ticks: u64,
    pub unpaint_ticks: u64,
    pub unoverlay_ticks: u64,
    pub attack_range: i64,
    pub attack_damage: Num,
    pub defence_damage: Num,
    pub build_reach: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachinesData {
    pub kinds: Vec<String>,
    pub colors: Vec<String>,
    pub bots_per_printer: u64,
    pub name_max: usize,
    pub fault_damage: Num,
    pub models: BTreeMap<String, ModelData>,
    /// Loudness per sound cause.
    pub loudness: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainData {
    pub passable: bool,
    pub blocks_sight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldData {
    pub terrain: BTreeMap<String, TerrainData>,
    /// Per resource kind: the largest cap and regrowth a map may set.
    pub deposit_max: BTreeMap<String, (Num, Num)>,
    pub overlays: Vec<String>,
    pub speed_steps: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetData {
    pub delay: u64,
    pub stall_report_ticks: u64,
}

/// Everything the tables hold, loaded once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    pub machines: MachinesData,
    pub world: WorldData,
    pub net: NetData,
}

impl Data {
    pub fn load() -> D<Data> {
        Ok(Data {
            machines: MachinesData::parse(MACHINES_TOML)?,
            world: WorldData::parse(WORLD_TOML)?,
            net: NetData::parse(NET_TOML)?,
        })
    }

    pub fn model(&self, name: &str) -> &ModelData {
        self.machines
            .models
            .get(name)
            .unwrap_or_else(|| panic!("model {name} is not in machines.toml"))
    }
}

fn err(file: &'static str, message: impl Into<String>) -> DataError {
    DataError {
        file,
        message: message.into(),
    }
}

fn table(file: &'static str, text: &str) -> D<toml::Table> {
    text.parse::<toml::Table>()
        .map_err(|e| err(file, e.to_string()))
}

fn get<'a>(file: &'static str, t: &'a toml::Table, key: &str) -> D<&'a toml::Value> {
    t.get(key)
        .ok_or_else(|| err(file, format!("missing key `{key}`")))
}

/// A `num` from a TOML value: an integer, or a string holding a literal.
fn num(file: &'static str, key: &str, v: &toml::Value) -> D<Num> {
    match v {
        toml::Value::Integer(i) => {
            Num::from_int(i128::from(*i)).map_err(|_| err(file, format!("`{key}` is out of range")))
        }
        toml::Value::String(s) => {
            Num::parse_literal(s).ok_or_else(|| err(file, format!("`{key}` is not a num literal")))
        }
        toml::Value::Float(_) => Err(err(
            file,
            format!("`{key}` is a float; write a decimal as a string, e.g. \"2.5\""),
        )),
        _ => Err(err(file, format!("`{key}` is not a number"))),
    }
}

fn int(file: &'static str, key: &str, v: &toml::Value) -> D<i64> {
    match v {
        toml::Value::Integer(i) => Ok(*i),
        _ => Err(err(file, format!("`{key}` is not an integer"))),
    }
}

fn uint(file: &'static str, key: &str, v: &toml::Value) -> D<u64> {
    u64::try_from(int(file, key, v)?).map_err(|_| err(file, format!("`{key}` is negative")))
}

fn num_map(file: &'static str, key: &str, v: &toml::Value) -> D<BTreeMap<String, Num>> {
    let t = v
        .as_table()
        .ok_or_else(|| err(file, format!("`{key}` is not a table")))?;
    let mut out = BTreeMap::new();
    for (k, x) in t {
        out.insert(k.clone(), num(file, &format!("{key}.{k}"), x)?);
    }
    Ok(out)
}

fn strings(file: &'static str, key: &str, v: &toml::Value) -> D<Vec<String>> {
    let a = v
        .as_array()
        .ok_or_else(|| err(file, format!("`{key}` is not a list")))?;
    a.iter()
        .map(|x| {
            x.as_str()
                .map(str::to_string)
                .ok_or_else(|| err(file, format!("`{key}` holds a non-string")))
        })
        .collect()
}

impl MachinesData {
    pub fn parse(text: &str) -> D<MachinesData> {
        const FILE: &str = "data/machines.toml";
        let t = table(FILE, text)?;
        let resource = get(FILE, &t, "resource")?
            .as_table()
            .ok_or_else(|| err(FILE, "`resource` is not a table"))?;
        let kinds = strings(FILE, "resource.kinds", get(FILE, resource, "kinds")?)?;
        let deployment = get(FILE, &t, "deployment")?
            .as_table()
            .ok_or_else(|| err(FILE, "`deployment` is not a table"))?;
        let colors = strings(FILE, "deployment.colors", get(FILE, deployment, "colors")?)?;
        let bots_per_printer = uint(
            FILE,
            "deployment.bots_per_printer",
            get(FILE, deployment, "bots_per_printer")?,
        )?;
        let name_max = usize::try_from(uint(
            FILE,
            "deployment.name_max",
            get(FILE, deployment, "name_max")?,
        )?)
        .map_err(|_| err(FILE, "name_max"))?;
        let fault_damage = num(
            FILE,
            "deployment.fault_damage",
            get(FILE, deployment, "fault_damage")?,
        )?;
        let models_t = get(FILE, &t, "model")?
            .as_table()
            .ok_or_else(|| err(FILE, "`model` is not a table"))?;
        let mut models = BTreeMap::new();
        for (name, mv) in models_t {
            let m = mv
                .as_table()
                .ok_or_else(|| err(FILE, format!("`model.{name}` is not a table")))?;
            let key = |k: &str| format!("model.{name}.{k}");
            let mut d = ModelData::default();
            for (k, v) in m {
                match k.as_str() {
                    "vision" => d.vision = int(FILE, &key(k), v)?,
                    "hearing" => d.hearing = int(FILE, &key(k), v)?,
                    "health" => d.health = num(FILE, &key(k), v)?,
                    "death_threshold" => d.death_threshold = num(FILE, &key(k), v)?,
                    "capacity" => d.capacity = num_map(FILE, &key(k), v)?,
                    "store" => d.store = num_map(FILE, &key(k), v)?,
                    "cost" => d.cost = num_map(FILE, &key(k), v)?,
                    "pick_rate" => d.pick_rate = num_map(FILE, &key(k), v)?,
                    "move_ticks" => d.move_ticks = uint(FILE, &key(k), v)?,
                    "pick_ticks" => d.pick_ticks = uint(FILE, &key(k), v)?,
                    "drop_ticks" => d.drop_ticks = uint(FILE, &key(k), v)?,
                    "attack_ticks" => d.attack_ticks = uint(FILE, &key(k), v)?,
                    "convert_ticks" => d.convert_ticks = uint(FILE, &key(k), v)?,
                    "print_ticks" => d.print_ticks = uint(FILE, &key(k), v)?,
                    "build_ticks" => d.build_ticks = uint(FILE, &key(k), v)?,
                    "deconstruct_ticks" => d.deconstruct_ticks = uint(FILE, &key(k), v)?,
                    "paint_ticks" => d.paint_ticks = uint(FILE, &key(k), v)?,
                    "overlay_ticks" => d.overlay_ticks = uint(FILE, &key(k), v)?,
                    "unpaint_ticks" => d.unpaint_ticks = uint(FILE, &key(k), v)?,
                    "unoverlay_ticks" => d.unoverlay_ticks = uint(FILE, &key(k), v)?,
                    "attack_range" => d.attack_range = int(FILE, &key(k), v)?,
                    "attack_damage" => d.attack_damage = num(FILE, &key(k), v)?,
                    "defence_damage" => d.defence_damage = num(FILE, &key(k), v)?,
                    "build_reach" => d.build_reach = int(FILE, &key(k), v)?,
                    other => {
                        return Err(err(
                            FILE,
                            format!("`model.{name}.{other}` is not a field docs/02 names"),
                        ));
                    }
                }
            }
            models.insert(name.clone(), d);
        }
        for required in ["bot", "printer", "depot", "site"] {
            if !models.contains_key(required) {
                return Err(err(FILE, format!("no `model.{required}`")));
            }
        }
        let loud_t = get(FILE, &t, "loudness")?
            .as_table()
            .ok_or_else(|| err(FILE, "`loudness` is not a table"))?;
        let mut loudness = BTreeMap::new();
        for (k, v) in loud_t {
            loudness.insert(k.clone(), int(FILE, &format!("loudness.{k}"), v)?);
        }
        Ok(MachinesData {
            kinds,
            colors,
            bots_per_printer,
            name_max,
            fault_damage,
            models,
            loudness,
        })
    }
}

impl WorldData {
    pub fn parse(text: &str) -> D<WorldData> {
        const FILE: &str = "data/world.toml";
        let t = table(FILE, text)?;
        let terrain_t = get(FILE, &t, "terrain")?
            .as_table()
            .ok_or_else(|| err(FILE, "`terrain` is not a table"))?;
        let mut terrain = BTreeMap::new();
        for (name, v) in terrain_t {
            let row = v
                .as_table()
                .ok_or_else(|| err(FILE, format!("`terrain.{name}` is not a table")))?;
            let flag = |k: &str| -> D<bool> {
                get(FILE, row, k)?
                    .as_bool()
                    .ok_or_else(|| err(FILE, format!("`terrain.{name}.{k}` is not a bool")))
            };
            terrain.insert(
                name.clone(),
                TerrainData {
                    passable: flag("passable")?,
                    blocks_sight: flag("blocks_sight")?,
                },
            );
        }
        for required in ["ground", "rock", "water"] {
            if !terrain.contains_key(required) {
                return Err(err(FILE, format!("no `terrain.{required}`")));
            }
        }
        let dep_t = get(FILE, &t, "deposit")?
            .as_table()
            .ok_or_else(|| err(FILE, "`deposit` is not a table"))?;
        let mut deposit_max = BTreeMap::new();
        for (kind, v) in dep_t {
            let row = v
                .as_table()
                .ok_or_else(|| err(FILE, format!("`deposit.{kind}` is not a table")))?;
            let cap = num(FILE, "max_cap", get(FILE, row, "max_cap")?)?;
            let regrowth = num(FILE, "max_regrowth", get(FILE, row, "max_regrowth")?)?;
            deposit_max.insert(kind.clone(), (cap, regrowth));
        }
        let mark = get(FILE, &t, "mark")?
            .as_table()
            .ok_or_else(|| err(FILE, "`mark` is not a table"))?;
        let overlays = strings(FILE, "mark.overlays", get(FILE, mark, "overlays")?)?;
        let speed = get(FILE, &t, "speed")?
            .as_table()
            .ok_or_else(|| err(FILE, "`speed` is not a table"))?;
        let steps_v = get(FILE, speed, "steps")?
            .as_array()
            .ok_or_else(|| err(FILE, "`speed.steps` is not a list"))?;
        let mut speed_steps = Vec::new();
        for s in steps_v {
            speed_steps.push(uint(FILE, "speed.steps", s)?);
        }
        Ok(WorldData {
            terrain,
            deposit_max,
            overlays,
            speed_steps,
        })
    }
}

impl NetData {
    pub fn parse(text: &str) -> D<NetData> {
        const FILE: &str = "data/net.toml";
        let t = table(FILE, text)?;
        Ok(NetData {
            delay: uint(FILE, "delay", get(FILE, &t, "delay")?)?,
            stall_report_ticks: uint(
                FILE,
                "stall_report_ticks",
                get(FILE, &t, "stall_report_ticks")?,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_tables_load() {
        let d = Data::load().expect("data");
        assert_eq!(d.machines.kinds, ["ore"]);
        assert_eq!(d.machines.colors[0], "red");
        assert_eq!(d.model("bot").move_ticks, 2);
        assert_eq!(d.model("printer").defence_damage, Num::ONE);
        assert!(d.world.terrain["rock"].blocks_sight);
        assert_eq!(d.net.delay, 3);
        assert_eq!(d.world.speed_steps[0], 0);
    }

    #[test]
    fn a_float_is_refused_with_the_fix_named() {
        let text = MACHINES_TOML.replace("fault_damage = 1", "fault_damage = 1.5");
        let e = MachinesData::parse(&text).expect_err("float");
        assert!(e.message.contains("string"), "{e}");
        let text = MACHINES_TOML.replace("fault_damage = 1", "fault_damage = \"1.5\"");
        let d = MachinesData::parse(&text).expect("string literal");
        assert_eq!(d.fault_damage, Num::parse_literal("1.5").unwrap());
    }

    #[test]
    fn an_unknown_model_field_is_refused() {
        let text = MACHINES_TOML.replace("build_reach = 3", "build_reach = 3\nspeed = 4");
        let e = MachinesData::parse(&text).expect_err("unknown field");
        assert!(e.message.contains("speed"), "{e}");
    }
}
