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
use crate::view;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sim::world::TileState;
use sim::{CommandKind, PlanKind};

/// Seconds between saves of a changed layout.
const SAVE_EVERY: f32 = 1.0;

/// Open or close a window by name; the dock and the `L` key use it.
pub fn toggle(state: &mut ViewState, name: &str) {
    if let Some(p) = state.layout.windows.get_mut(name) {
        p.open = !p.open;
        state.layout_dirty = true;
    }
}

/// The windows in dock order: the deployments as the editor orders them,
/// then the inspector, the tools and the log.
fn window_names(state: &ViewState) -> Vec<String> {
    let mut names = editor::deployment_order(state.editor.docs.keys().cloned());
    names.extend(["inspector", "tools", "log"].map(String::from));
    names
}

pub fn panels(
    mut contexts: EguiContexts,
    time: Res<Time>,
    interface: Res<Interface>,
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
    let names = window_names(&state);
    // The first color deployment opens by default: it is what the player
    // edits most.
    let first_color = names
        .iter()
        .find(|n| {
            n.as_str() != "printer" && n.as_str() != "depot" && state.editor.docs.contains_key(*n)
        })
        .cloned();
    // Where a window may go: the viewport under the bar. The bar's height
    // is known only after it is drawn; the defaults, seeded once, use a
    // guess, and the constraint below uses the measured rect.
    let guess = {
        let r = ctx.viewport_rect();
        egui::Rect::from_min_max(egui::pos2(r.min.x, r.min.y + 52.0), r.max)
    };

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
        // The dock: a toggle per window.
        ui.horizontal_wrapped(|ui| {
            let snap = d.snapshot().clone();
            let team = snap.teams.iter().find(|t| t.id == d.player);
            let screen = guess;
            for (i, name) in names.iter().enumerate() {
                let open = state
                    .layout
                    .placement(name, || {
                        default_placement(name, i, screen, Some(name) == first_color.as_ref())
                    })
                    .open;
                let label = if editor::deployment_order([name.clone()].into_iter()).len() == 1
                    && state.editor.docs.contains_key(name)
                {
                    let running = team.and_then(|t| t.deployments.get(name).copied().flatten());
                    editor::title(&state, name, running)
                } else {
                    name.clone()
                };
                if ui.selectable_label(open, label).clicked() {
                    toggle(&mut state, name);
                }
            }
            if !state.status.is_empty() {
                ui.separator();
                ui.small(&state.status);
            }
        });
    });

    // The windows, constrained to what the bar leaves.
    let screen = root.available_rect_before_wrap();
    for (i, name) in names.iter().enumerate() {
        let placement = *state.layout.placement(name, || {
            default_placement(name, i, screen, Some(name) == first_color.as_ref())
        });
        if !placement.open {
            continue;
        }
        let mut open = true;
        let is_deployment = state.editor.docs.contains_key(name);
        let title = if is_deployment {
            let snap = driver.0.snapshot();
            let running = snap
                .teams
                .iter()
                .find(|t| t.id == driver.0.player)
                .and_then(|t| t.deployments.get(name).copied().flatten());
            editor::title(&state, name, running)
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
                if is_deployment {
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
