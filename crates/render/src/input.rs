//! Input: the keys, and clicks that become commands (Q10). The renderer
//! submits a command and forgets it; nothing here changes what a machine
//! does. The camera is `camera.rs`.

use crate::app::{DriverResource, Tool, ViewState};
use crate::camera::{LmbGesture, cursor_tile};
use crate::palette::Frame;
use crate::view::Entities;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use sim::{CommandKind, PlanKind};

/// Keys: tools 1–5, speed with `-`/`=`, `Shift+D` deploys, `L` toggles the
/// log, `Escape` drops the tool, `Shift+R` resigns.
pub fn keys(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
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
    if keys.just_pressed(KeyCode::Digit1) {
        state.tool = Tool::Building("depot".into());
    }
    if keys.just_pressed(KeyCode::Digit2) {
        state.tool = Tool::Paint("red".into());
    }
    if keys.just_pressed(KeyCode::Digit3) {
        state.tool = Tool::Overlay("a".into());
    }
    if keys.just_pressed(KeyCode::Digit4) {
        state.tool = Tool::Clear(PlanKind::Paint);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        state.tool = Tool::Unmark(PlanKind::Building);
    }
    if keys.just_pressed(KeyCode::KeyL) {
        state.show_log = !state.show_log;
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

/// Deploy every working copy that is ahead of its running version
/// (`docs/07`, Q33).
pub fn deploy(driver: &mut crate::driver::Driver, state: &mut ViewState) {
    let names = state.editor.changed(driver.snapshot(), driver.player);
    if names.is_empty() {
        state.status = "every working copy is what its deployment runs".into();
    }
    for name in names {
        crate::editor::deploy_doc(driver, state, &name);
    }
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

/// The tile under the cursor every frame; a click (a left press that did
/// not become a drag) on it with a tool is a `Mark` or `Unmark`, and with
/// no tool selects the machine there.
pub fn click(
    mut contexts: EguiContexts,
    gesture: Res<LmbGesture>,
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
    let Some(tile) = tile else { return };
    if !gesture.clicked {
        return;
    }
    let tool = state.tool.clone();
    match tool {
        Tool::Select => {
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
        Tool::Building(model) => {
            submit(
                &mut driver.0,
                &mut state,
                CommandKind::Mark {
                    at: tile,
                    plan: PlanKind::Building,
                    value: Some(model),
                },
            );
        }
        Tool::Paint(color) => {
            submit(
                &mut driver.0,
                &mut state,
                CommandKind::Mark {
                    at: tile,
                    plan: PlanKind::Paint,
                    value: Some(color),
                },
            );
        }
        Tool::Overlay(label) => {
            submit(
                &mut driver.0,
                &mut state,
                CommandKind::Mark {
                    at: tile,
                    plan: PlanKind::Overlay,
                    value: Some(label),
                },
            );
        }
        Tool::Clear(plan) => {
            submit(
                &mut driver.0,
                &mut state,
                CommandKind::Mark {
                    at: tile,
                    plan,
                    value: None,
                },
            );
        }
        Tool::Unmark(plan) => {
            submit(
                &mut driver.0,
                &mut state,
                CommandKind::Unmark { at: tile, plan },
            );
        }
    }
}
