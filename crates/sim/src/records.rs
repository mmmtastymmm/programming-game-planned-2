//! The records a program reads (`docs/02`, A machine's attributes; Senses):
//! machine records, tile records and sounds as language values.

use crate::command::TeamId;
use crate::map::TilePos;
use crate::world::{MachineRecord, Sound, TileMemory, TileState, World};
use lang::Num;
use lang::value::{DictObj, ListObj, Record, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub fn int(i: i128) -> Value {
    Value::Num(Num::from_int(i).unwrap_or(Num::ZERO))
}

pub fn pos_value(p: TilePos) -> Value {
    Value::tuple(vec![int(i128::from(p.x)), int(i128::from(p.y))])
}

pub fn opt_str(s: Option<&str>) -> Value {
    match s {
        Some(s) => Value::str(s),
        None => Value::None,
    }
}

/// A `dict` of resource kind to amount, as a host object.
pub fn amounts(world: &mut World, m: &BTreeMap<String, Num>) -> Value {
    let items: Vec<(Value, Value)> = m
        .iter()
        .map(|(k, v)| (Value::str(k), Value::Num(*v)))
        .collect();
    Value::Dict(Rc::new(DictObj {
        id: world.host_object_id(),
        items: RefCell::new(items),
    }))
}

pub fn list(world: &mut World, items: Vec<Value>) -> Value {
    Value::List(Rc::new(ListObj {
        id: world.host_object_id(),
        items: RefCell::new(items),
    }))
}

fn record(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Rc::new(Record {
        type_name: type_name.to_string(),
        fields: fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }))
}

pub fn machine_record(world: &mut World, r: &MachineRecord) -> Value {
    let load = r.load.as_ref().map(|l| amounts(world, l));
    let store = r.store.as_ref().map(|s| amounts(world, s));
    let mut fields = vec![
        ("id", int(i128::from(r.id.0))),
        ("name", Value::str(&r.name)),
        ("kind", Value::str(r.kind)),
        ("model", Value::str(r.model.name())),
        ("team", int(i128::from(r.team.0))),
        ("deployment", opt_str(r.deployment.as_deref())),
        ("pos", pos_value(r.pos)),
        ("health", Value::Num(r.health)),
        ("busy", opt_str(r.busy)),
    ];
    if let Some(l) = load {
        fields.push(("load", l));
    }
    if let Some(s) = store {
        fields.push(("store", s));
    }
    fields.push(("progress", int(i128::from(r.progress))));
    record("machine", fields)
}

pub fn sound_record(s: &Sound) -> Value {
    record(
        "sound",
        vec![
            ("cause", Value::str(&s.cause)),
            ("pos", pos_value(s.pos)),
            ("loudness", int(i128::from(s.loudness))),
            ("tick", int(i128::from(s.tick))),
        ],
    )
}

/// A tile's record for `team`, or `None` for an unknown tile.
pub fn tile_record(world: &mut World, team: TeamId, p: TilePos) -> Value {
    let Some(idx) = world.tile_index(p) else {
        return Value::None;
    };
    let mem: TileMemory = world.teams[&team].memory[idx].clone();
    if mem.state == TileState::Unknown {
        return Value::None;
    }
    let (terrain, deposit, paint, overlay, building, seen_at) = if mem.state == TileState::Visible {
        // The world now.
        let tile = world.tiles[idx].clone();
        let building = world
            .machines
            .values()
            .find(|m| m.pos == p && m.is_building())
            .map(MachineRecord::of);
        (
            tile.terrain,
            tile.deposit.as_ref().map(|d| d.amount.clone()),
            tile.paint.clone(),
            tile.overlay.clone(),
            building,
            world.tick,
        )
    } else {
        (
            mem.terrain,
            mem.deposit.clone(),
            mem.paint.clone(),
            mem.overlay.clone(),
            mem.building.clone(),
            mem.seen_at,
        )
    };
    let state = match mem.state {
        TileState::Unknown => "unknown",
        TileState::Visible => "visible",
        TileState::Remembered => "remembered",
    };
    let deposit_v = match &deposit {
        Some(d) => amounts(world, d),
        None => Value::None,
    };
    let building_v = match &building {
        Some(b) => machine_record(world, b),
        None => Value::None,
    };
    let plans = world.plans(team, p).cloned().unwrap_or_default();
    let plan_value = |pl: &Option<crate::world::Plan>| match pl {
        Some(crate::world::Plan::Set(v)) => Value::str(v),
        Some(crate::world::Plan::Clear) | None => Value::None,
    };
    let plans_v = {
        let items = vec![
            (Value::str("paint"), plan_value(&plans.paint)),
            (Value::str("overlay"), plan_value(&plans.overlay)),
            (Value::str("building"), plan_value(&plans.building)),
        ];
        Value::Dict(Rc::new(DictObj {
            id: world.host_object_id(),
            items: RefCell::new(items),
        }))
    };
    record(
        "tile",
        vec![
            ("x", int(i128::from(p.x))),
            ("y", int(i128::from(p.y))),
            ("state", Value::str(state)),
            ("terrain", Value::str(terrain.name())),
            ("deposit", deposit_v),
            ("building", building_v),
            ("seen_at", int(i128::from(seen_at))),
            ("paint", opt_str(paint.as_deref())),
            ("overlay", opt_str(overlay.as_deref())),
            ("plans", plans_v),
        ],
    )
}
