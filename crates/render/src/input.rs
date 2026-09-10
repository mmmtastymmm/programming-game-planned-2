//! Input: the camera, the keys, and clicks that become commands (Q10). The
//! renderer submits a command and forgets it; nothing here changes what a
//! machine does.

use crate::app::{DriverResource, Tool, ViewState};
use crate::view::{self, TILE};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use sim::{CommandKind, PlanKind};

pub fn camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut cams: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(mut t) = cams.single_mut() else { return };
    let dt = time.delta_secs();
    let speed = 600.0 * t.scale.x;
    let mut d = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        d.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        d.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        d.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) && !keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ArrowRight)
    {
        d.x += 1.0;
    }
    if d != Vec2::ZERO {
        t.translation += (d.normalize() * speed * dt).extend(0.0);
    }
    for w in wheel.read() {
        let factor = if w.y > 0.0 { 0.9 } else { 1.1 };
        let s = (t.scale.x * factor).clamp(0.25, 4.0);
        t.scale = Vec3::new(s, s, 1.0);
    }
}

/// Keys: tools 1–5, speed with `-`/`=`, `Shift+D` deploys, `L` toggles the
/// log, `Escape` drops the tool, `Shift+R` resigns.
pub fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::Escape) {
        state.tool = Tool::Select;
        state.selected = None;
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
    let steps = sim::Data::load()
        .map(|d| d.world.speed_steps)
        .unwrap_or_default();
    let current = driver.0.speed;
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::Equal) {
        let idx = steps.iter().position(|s| *s == current).unwrap_or(0);
        let next = if keys.just_pressed(KeyCode::Minus) {
            idx.saturating_sub(1)
        } else {
            (idx + 1).min(steps.len().saturating_sub(1))
        };
        if let Some(s) = steps.get(next) {
            submit(&mut driver.0, &mut state, CommandKind::SetSpeed(*s));
        }
    }
    if shift && keys.just_pressed(KeyCode::KeyD) {
        deploy(&mut driver.0, &mut state);
    }
    if shift && keys.just_pressed(KeyCode::KeyR) {
        submit(&mut driver.0, &mut state, CommandKind::Resign);
    }
}

/// Re-read the programs directory and deploy every bundle that changed.
pub fn deploy(driver: &mut crate::driver::Driver, state: &mut ViewState) {
    let Some(root) = state.programs.clone() else {
        state.status = "no programs directory (--programs DIR)".into();
        return;
    };
    match crate::programs::read_all(&root) {
        Ok(bundles) => {
            let deploys = crate::programs::deploys_for(&bundles, &state.deployed);
            if deploys.is_empty() {
                state.status = "programs unchanged".into();
            }
            for kind in deploys {
                let name = match &kind {
                    CommandKind::Deploy { deployment, .. } => deployment.clone(),
                    _ => String::new(),
                };
                if submit(driver, state, kind)
                    && let Some(b) = bundles.get(&name)
                {
                    state.deployed.insert(name, b.clone());
                }
            }
        }
        Err(e) => state.status = e,
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

/// A click on a tile with a tool is a `Mark` or `Unmark`; with no tool, it
/// selects the machine there.
pub fn click(
    mut contexts: EguiContexts,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    let over_ui = contexts
        .ctx_mut()
        .map(|c| c.is_pointer_over_egui() || c.egui_wants_pointer_input())
        .unwrap_or(false);
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = cams.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        state.hover = None;
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let tile = view::tile_of(world);
    state.hover = Some(tile);
    if over_ui || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let tool = state.tool.clone();
    match tool {
        Tool::Select => {
            let snap = driver.0.snapshot();
            let hit = snap
                .machines
                .iter()
                .filter(|m| m.record.pos == tile)
                .map(|m| m.record.id)
                .next();
            state.selected = hit;
            let _ = TILE;
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
