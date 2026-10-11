//! Input: the keys, and clicks that become commands (Q10). The renderer
//! submits a command and forgets it; nothing here changes what a machine
//! does. The camera is `camera.rs`.

use crate::app::{Drag, DriverResource, Tool, ViewState};
use crate::camera::{LmbGesture, cursor_tile};
use crate::marks::{self, MarkRows};
use crate::palette::Frame;
use crate::view::Entities;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use sim::world::Plans;
use sim::{CommandKind, PlanKind};
use std::collections::BTreeMap;

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Keys: `B`/`P`/`O` pick a row of the mark strip and `1`–`9` arm an entry
/// in it (Q37), `Escape` disarms, speed with `-`/`=`, `Shift+D` deploys,
/// `L` toggles the log, `Shift+R` resigns.
pub fn keys(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    rows: Res<MarkRows>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    if contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.egui_wants_keyboard_input())
    {
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::Escape) {
        if state.tool != Tool::Select {
            state.tool = Tool::Select;
        } else {
            state.selected = None;
        }
    }
    for (key, plan) in [
        (KeyCode::KeyB, PlanKind::Building),
        (KeyCode::KeyP, PlanKind::Paint),
        (KeyCode::KeyO, PlanKind::Overlay),
    ] {
        if keys.just_pressed(key) {
            state.row = Some(plan);
        }
    }
    if let Some(row) = state.row {
        for (i, key) in DIGITS.iter().enumerate() {
            if keys.just_pressed(*key)
                && let Some(tool) = rows.pick(row, i + 1, &state.tool)
            {
                state.tool = tool;
            }
        }
    }
    if keys.just_pressed(KeyCode::KeyL) {
        crate::ui::toggle(&mut state, "log");
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::Equal) {
        step_speed(&mut driver.0, &mut state, keys.just_pressed(KeyCode::Equal));
    }
    if shift && keys.just_pressed(KeyCode::KeyD) {
        deploy(&mut driver.0, &mut state);
    }
    if shift && keys.just_pressed(KeyCode::KeyR) {
        submit(&mut driver.0, &mut state, CommandKind::Resign);
    }
}

/// One step along `data/world.toml`'s speed ladder.
pub fn step_speed(driver: &mut crate::driver::Driver, state: &mut ViewState, up: bool) {
    let steps = sim::Data::load()
        .map(|d| d.world.speed_steps)
        .unwrap_or_default();
    let idx = steps.iter().position(|s| *s == driver.speed).unwrap_or(0);
    let next = if up {
        (idx + 1).min(steps.len().saturating_sub(1))
    } else {
        idx.saturating_sub(1)
    };
    if let Some(s) = steps.get(next) {
        submit(driver, state, CommandKind::SetSpeed(*s));
    }
}

/// Deploy every deployment the tree composes ahead of what runs
/// (`docs/07`, Q41).
pub fn deploy(driver: &mut crate::driver::Driver, state: &mut ViewState) {
    crate::editor::deploy_changed(driver, state);
}

/// Submit and report; `true` if the log took it.
pub fn submit(
    driver: &mut crate::driver::Driver,
    state: &mut ViewState,
    kind: CommandKind,
) -> bool {
    let label = match &kind {
        CommandKind::Deploy { deployment, .. } => format!("deploy {deployment}"),
        CommandKind::Mark { at, plan, value } => {
            format!("mark {} ({}, {}) {:?}", plan.name(), at.x, at.y, value)
        }
        CommandKind::Unmark { at, plan } => format!("unmark {} ({}, {})", plan.name(), at.x, at.y),
        CommandKind::SetSpeed(s) => format!("speed {s}"),
        CommandKind::Resign => "resign".to_string(),
    };
    match driver.submit(kind) {
        Ok(tick) => {
            state.status = format!("{label}: agreed for tick {tick}");
            state.log_lines.push(format!("> {label} (tick {tick})"));
            true
        }
        Err(e) => {
            state.status = format!("{label}: refused — {e}");
            false
        }
    }
}

/// The tile under the cursor every frame. Unarmed, a click (a left press
/// that did not become a drag) selects the machine there. Armed, the press
/// marks the tile and a drag every tile the cursor crosses, each once
/// (Q37); a press with `Shift` held instead anchors a rectangle, which
/// fills on release — or on nothing, if `Esc` disarms first (Q44).
pub fn click(
    mut contexts: EguiContexts,
    gesture: Res<LmbGesture>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    frame: Res<Frame>,
    ents: Res<Entities>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    let over_ui = contexts
        .ctx_mut()
        .is_ok_and(|c| c.is_pointer_over_egui() || c.egui_wants_pointer_input());
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = cams.single() else {
        return;
    };
    let tile = if over_ui {
        None
    } else {
        cursor_tile(window, camera, cam_tf, &frame, |p| ents.top(p))
    };
    state.hover = tile;
    let tool = state.tool.clone();
    // A drag lives while the left button is held from a press on the map:
    // the camera's gesture only starts on one.
    if !buttons.pressed(MouseButton::Left) || gesture.travel.is_none() {
        if let Some(Drag {
            anchor: Some(a),
            last: Some(b),
            ..
        }) = state.drag.take()
        {
            mark_tiles(&mut driver.0, &mut state, &tool, marks::rect(a, b));
        }
        if tool == Tool::Select {
            select(&gesture, tile, &driver, &ents, &mut state);
        }
        return;
    }
    if tool == Tool::Select {
        return;
    }
    let Some(tile) = tile else { return };
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let corner = clamp(tile, driver.0.bounds);
    let mut drag = state.drag.take().unwrap_or_else(|| Drag {
        anchor: shift.then_some(corner),
        ..Drag::default()
    });
    let crossed = match (drag.anchor, drag.last) {
        (Some(_), _) => vec![],
        (None, Some(last)) => marks::walk(last, tile),
        (None, None) => vec![tile],
    };
    drag.last = Some(if drag.anchor.is_some() { corner } else { tile });
    let fresh: Vec<_> = crossed
        .into_iter()
        .filter(|p| drag.done.insert(*p))
        .collect();
    mark_tiles(&mut driver.0, &mut state, &tool, fresh);
    state.drag = Some(drag);
}

/// `p` moved onto the map: a rectangle's corner stops at the edge.
fn clamp(p: sim::TilePos, (min, max): (sim::TilePos, sim::TilePos)) -> sim::TilePos {
    sim::TilePos::new(p.x.clamp(min.x, max.x), p.y.clamp(min.y, max.y))
}

/// Submit what marking each of `tiles` on the map with `tool` becomes,
/// given the team's plans there as the sim last reported them.
fn mark_tiles(
    driver: &mut crate::driver::Driver,
    state: &mut ViewState,
    tool: &Tool,
    tiles: Vec<sim::TilePos>,
) {
    if tiles.is_empty() {
        return;
    }
    let (min, max) = driver.bounds;
    let player = driver.player;
    let held: BTreeMap<sim::TilePos, Plans> = driver
        .snapshot()
        .teams
        .iter()
        .find(|t| t.id == player)
        .map(|t| t.plans.iter().cloned().collect())
        .unwrap_or_default();
    for p in tiles {
        let on_map = (min.x..=max.x).contains(&p.x) && (min.y..=max.y).contains(&p.y);
        if on_map && let Some(kind) = marks::command(tool, p, held.get(&p)) {
            submit(driver, state, kind);
        }
    }
}

/// Unarmed, a click selects the machine on the tile.
fn select(
    gesture: &LmbGesture,
    tile: Option<sim::TilePos>,
    driver: &DriverResource,
    ents: &Entities,
    state: &mut ViewState,
) {
    let Some(tile) = tile else { return };
    if !gesture.clicked {
        return;
    }
    let snap = driver.0.snapshot();
    state.selected = snap
        .machines
        .iter()
        .filter(|m| {
            m.record.pos == tile && ents.machines.get(&m.record.id).is_some_and(|v| v.shown)
        })
        .map(|m| m.record.id)
        .next();
}
