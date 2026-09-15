//! The screen, as `docs/07` rules it (Q39): the map fills the window, the
//! time bar is the one fixed strip and doubles as the dock, and everything
//! else — one window per deployment's program, the inspector, the tools,
//! the log — floats over the map, dragged, resized, collapsed and closed by
//! the player, its layout remembered beside the programs. Every button
//! becomes a command the renderer submits and forgets.

use crate::app::{DriverResource, Tool, ViewState};
use crate::editor;
use crate::input::{step_speed, submit};
use crate::layout::default_placement;
use crate::palette::Interface;
use crate::view::{self, Entities, MachineBody};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sim::world::TileState;
use sim::{CommandKind, PlanKind};
use std::collections::BTreeSet;

/// Seconds between saves of a changed layout.
const SAVE_EVERY: f32 = 1.0;

/// Open or close a window by name; the dock and the `L` key use it.
pub fn toggle(state: &mut ViewState, name: &str) {
    if let Some(p) = state.layout.windows.get_mut(name) {
        p.open = !p.open;
        state.layout_dirty = true;
    }
}

/// The three windows that are not files.
const FIXED_WINDOWS: [&str; 3] = ["inspector", "tools", "log"];

pub fn panels(
    mut contexts: EguiContexts,
    time: Res<Time>,
    interface: Res<Interface>,
    ents: Res<Entities>,
    cams: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    bodies: Query<&GlobalTransform, With<MachineBody>>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let ctx = ctx.clone();
    let mut root = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    editor::sync(&driver, &mut state);

    egui::Panel::top("time").show(&mut root, |ui| {
        let d = &mut driver.0;
        ui.horizontal(|ui| {
            ui.strong(&d.map_name);
            ui.separator();
            ui.label(format!("tick {}", d.tick));
            ui.separator();
            if ui.small_button("−").clicked() {
                step_speed(d, &mut state, false);
            }
            ui.label(if d.speed == 0 {
                "paused".to_string()
            } else {
                format!("{} ticks/s", d.speed)
            });
            if ui.small_button("+").clicked() {
                step_speed(d, &mut state, true);
            }
            ui.separator();
            ui.label(format!("delay {}", d.delay));
            ui.separator();
            ui.monospace(format!("{:016x}", d.state_hash()));
            if d.is_networked() {
                ui.separator();
                let peers: Vec<String> = d
                    .remote_peers
                    .iter()
                    .map(|t| format!("team {}", t.0))
                    .collect();
                ui.label(format!(
                    "you are team {} · with {}",
                    d.player.0,
                    if peers.is_empty() {
                        "none left".to_string()
                    } else {
                        peers.join(", ")
                    }
                ));
            }
            if let Some(r) = &d.desync {
                ui.separator();
                ui.colored_label(egui::Color32::LIGHT_RED, r);
            }
            if let Some(r) = &d.stall_report {
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, format!("stalled: {r}"));
            }
            if let Some(ended) = d.ended {
                ui.separator();
                ui.colored_label(
                    egui::Color32::LIGHT_GREEN,
                    match ended {
                        Some(t) => format!("match over — team {} wins", t.0),
                        None => "match over — a draw".to_string(),
                    },
                );
            }
        });
        if !state.status.is_empty() {
            ui.small(&state.status);
        }
    });

    // The tree (Q41), fixed down the left; its foot opens the other
    // windows.
    let snap = driver.0.snapshot().clone();
    let player = driver.0.player;
    let open_now: BTreeSet<String> = state
        .layout
        .windows
        .iter()
        .filter(|(_, p)| p.open)
        .map(|(k, _)| k.clone())
        .collect();
    let mut to_open: Vec<String> = Vec::new();
    egui::Panel::left("tree")
        .exact_size(280.0)
        .show(&mut root, |ui| {
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 40.0)
                .show(ui, |ui| {
                    for a in editor::tree_panel(ui, &mut state, &snap, player, &open_now) {
                        match a {
                            editor::TreeAction::Open(p) => to_open.push(p),
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                for name in FIXED_WINDOWS {
                    let open = open_now.contains(name);
                    if ui.selectable_label(open, name).clicked() {
                        to_open.push(name.to_string());
                    }
                }
            });
        });
    // The first run opens the first robot file and the fixed windows.
    if state.layout.windows.is_empty() {
        let first = state
            .editor
            .files
            .keys()
            .filter(|p| {
                editor::robot_of(p)
                    .and_then(sim::world::deployment_number)
                    .is_some()
            })
            .min_by_key(|p| editor::robot_of(p).and_then(sim::world::deployment_number))
            .cloned();
        to_open.extend(first);
        to_open.extend(FIXED_WINDOWS.iter().map(|s| s.to_string()));
    }
    // A file clicked in the tree opens; a fixed window's label toggles.
    let screen = root.available_rect_before_wrap();
    let count = state.layout.windows.len();
    for (i, name) in to_open.iter().enumerate() {
        let fixed = FIXED_WINDOWS.contains(&name.as_str());
        let existed = state.layout.windows.contains_key(name);
        let p = state
            .layout
            .placement(name, || default_placement(name, count + i, screen, true));
        p.open = if fixed && existed { !p.open } else { true };
        state.layout_dirty = true;
    }

    // The windows, constrained to what the bars leave: every open file,
    // and the three fixed ones.
    let names: Vec<String> = state
        .layout
        .windows
        .iter()
        .filter(|(k, p)| {
            p.open && (state.editor.files.contains_key(*k) || FIXED_WINDOWS.contains(&k.as_str()))
        })
        .map(|(k, _)| k.clone())
        .collect();
    for name in &names {
        let placement = state.layout.windows[name];
        let mut open = true;
        let is_file = state.editor.files.contains_key(name);
        let title = if is_file {
            editor::title(&state, name, &snap, player)
        } else {
            name.clone()
        };
        let shown = egui::Window::new(title)
            .id(egui::Id::new(("window", name)))
            .open(&mut open)
            .default_rect(placement.rect())
            .constrain_to(screen)
            .resizable(true)
            .collapsible(true)
            .show(&ctx, |ui| {
                if is_file {
                    editor::window_body(ui, &mut driver, &mut state, name);
                } else {
                    match name.as_str() {
                        "inspector" => inspector(ui, &interface, &driver, &mut state),
                        "tools" => tools(ui, &mut driver, &mut state),
                        _ => log(ui, &state),
                    }
                }
            });
        let p = state.layout.windows.get_mut(name).expect("placed above");
        if let Some(r) = shown
            && p.differs_from(r.response.rect)
        {
            p.set_rect(r.response.rect);
            state.layout_dirty = true;
        }
        if !open {
            state
                .layout
                .windows
                .get_mut(name)
                .expect("placed above")
                .open = false;
            state.layout_dirty = true;
        }
    }

    // Every bot wears its deployment's number (Q40): drawn on the
    // background layer at the body's screen position, under the windows.
    if let Ok((camera, cam_tf)) = cams.single() {
        let painter = ctx.layer_painter(egui::LayerId::background());
        let font = egui::FontId::proportional(15.0);
        let snap = driver.0.snapshot();
        for m in &snap.machines {
            let Some(n) = m
                .record
                .deployment
                .as_deref()
                .and_then(sim::world::deployment_number)
            else {
                continue;
            };
            let Some(view) = ents.machines.get(&m.record.id) else {
                continue;
            };
            if !view.shown {
                continue;
            }
            let Ok(tf) = bodies.get(view.body) else {
                continue;
            };
            let Ok(at) = camera.world_to_viewport(cam_tf, tf.translation() + Vec3::Y * 0.05) else {
                continue;
            };
            let pos = egui::pos2(at.x, at.y);
            // Under the tree or the bar, not over them.
            if !screen.contains(pos) {
                continue;
            }
            let text = n.to_string();
            for d in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                painter.text(
                    pos + egui::vec2(d.0, d.1),
                    egui::Align2::CENTER_CENTER,
                    &text,
                    font.clone(),
                    egui::Color32::BLACK,
                );
            }
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                &text,
                font.clone(),
                egui::Color32::WHITE,
            );
        }
    }

    // A changed layout is saved once a second, never on every drag frame.
    let now = time.elapsed_secs();
    if state.layout_dirty
        && now - state.layout_saved_at >= SAVE_EVERY
        && let Some(path) = state.layout_path.clone()
    {
        if let Err(e) = state.layout.save(&path) {
            state.status = e;
        }
        state.layout_dirty = false;
        state.layout_saved_at = now;
    }
}

fn tools(ui: &mut egui::Ui, driver: &mut DriverResource, state: &mut ViewState) {
    let d = &mut driver.0;
    let mut tool = state.tool.clone();
    ui.radio_value(&mut tool, Tool::Select, "select (Esc)");
    ui.radio_value(
        &mut tool,
        Tool::Building("depot".into()),
        "plan a depot (1)",
    );
    ui.radio_value(&mut tool, Tool::Paint("red".into()), "plan red paint (2)");
    ui.radio_value(&mut tool, Tool::Overlay("a".into()), "plan overlay a (3)");
    ui.radio_value(
        &mut tool,
        Tool::Clear(PlanKind::Paint),
        "plan to clear paint (4)",
    );
    ui.radio_value(
        &mut tool,
        Tool::Unmark(PlanKind::Building),
        "withdraw building plan (5)",
    );
    state.tool = tool;
    ui.separator();
    if ui.button("resign (Shift+R)").clicked() {
        submit(d, state, CommandKind::Resign);
    }
    ui.separator();
    ui.small("left drag / middle / Shift+right: pan");
    ui.small("right drag: orbit · wheel: zoom");
    ui.small("WASD: pan · L: log · -/=: speed");
}

fn inspector(
    ui: &mut egui::Ui,
    interface: &Interface,
    driver: &DriverResource,
    state: &mut ViewState,
) {
    let d = &driver.0;
    let player = d.player;
    let snap = d.snapshot();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(id) = state.selected {
            match view::machine(snap, id) {
                Some(m) => {
                    let r = &m.record;
                    ui.colored_label(
                        interface.team_color32(player, r.team),
                        format!(
                            "{} — {} #{} of team {}",
                            r.name,
                            r.model.name(),
                            r.id.0,
                            r.team.0
                        ),
                    );
                    ui.label(format!("at ({}, {})", r.pos.x, r.pos.y));
                    ui.label(format!("health {}", r.health.to_decimal()));
                    match r.busy {
                        Some(b) => ui.label(format!("busy: {b} ({})", r.progress)),
                        None => ui.label("idle"),
                    };
                    let running = r.deployment.as_ref().and_then(|dep| {
                        snap.teams
                            .iter()
                            .find(|t| t.id == r.team)
                            .and_then(|t| t.deployments.get(dep).copied().flatten())
                    });
                    if let Some(dep) = &r.deployment {
                        match running {
                            Some(v) => ui.label(format!("runs {dep} ({v:016x})")),
                            None => ui.label(format!("runs {dep}")),
                        };
                    }
                    if let Some(l) = &r.load {
                        ui.label(format!("load: {}", amounts(l)));
                    }
                    if let Some(s) = &r.store {
                        ui.label(format!("store: {}", amounts(s)));
                    }
                    if let Some(f) = &m.fault {
                        let version = match running {
                            Some(v) if v == f.version => "the running version".to_string(),
                            _ => format!("version {:016x}, since redeployed", f.version),
                        };
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            format!(
                                "fault: {}\n  at {}:{} tick {}\n  {version}",
                                view::fault_what(f),
                                f.file.as_deref().unwrap_or("?"),
                                f.line,
                                f.tick
                            ),
                        );
                    }
                    if !m.log.is_empty() {
                        ui.separator();
                        for l in m.log.iter().rev().take(6) {
                            ui.small(format!("[{}] {}", l.level, l.text));
                        }
                    }
                }
                None => {
                    ui.label("gone");
                }
            }
            ui.separator();
        }
        match state.hover {
            Some(p) => {
                ui.label(format!("tile ({}, {})", p.x, p.y));
                match view::tile(snap, player, p) {
                    Some(mem) if mem.state != TileState::Unknown => {
                        let seen = match mem.state {
                            TileState::Visible => "in sight".to_string(),
                            _ => format!("remembered from tick {}", mem.seen_at),
                        };
                        ui.small(format!("{} — {seen}", mem.terrain.name()));
                        if let Some(dep) = &mem.deposit {
                            ui.small(format!("deposit: {}", amounts(dep)));
                        }
                        if let Some(paint) = &mem.paint {
                            ui.small(format!("paint: {paint}"));
                        }
                        if let Some(o) = &mem.overlay {
                            ui.small(format!("overlay: {o}"));
                        }
                        if let Some(b) = &mem.building {
                            ui.small(format!("{} of team {}", b.name, b.team.0));
                        }
                        if let Some(plan) = d.snapshot_plan(player, p) {
                            ui.small(format!("your plan: {plan}"));
                        }
                    }
                    _ => {
                        ui.small("unknown");
                    }
                }
            }
            None => {
                ui.small("hover a tile");
            }
        }
        ui.separator();
        ui.heading("Teams");
        for t in &snap.teams {
            let mine = if t.id == player { " (you)" } else { "" };
            let out = if t.out { " — out" } else { "" };
            ui.colored_label(
                interface.team_color32(player, t.id),
                format!(
                    "team {}{mine}: {} bots / cap {}{out}",
                    t.id.0, t.bots, t.cap
                ),
            );
            for (name, v) in &t.deployments {
                if let Some(v) = v {
                    ui.small(format!("  {name}: {v:016x}"));
                }
            }
        }
    });
}

fn log(ui: &mut egui::Ui, state: &ViewState) {
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for line in state
                .log_lines
                .iter()
                .rev()
                .take(200)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                ui.monospace(line);
            }
        });
}

fn amounts(m: &std::collections::BTreeMap<String, lang::Num>) -> String {
    m.iter()
        .map(|(k, v)| format!("{k} {}", v.to_decimal()))
        .collect::<Vec<_>>()
        .join(", ")
}
