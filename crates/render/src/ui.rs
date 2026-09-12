//! The panels, in `docs/07`'s layout: the time bar across the top, the
//! editor on the left, the inspector and the mark tools on the right, the
//! log along the bottom. Every button becomes a command the renderer
//! submits and forgets.

use crate::app::{DriverResource, Tool, ViewState};
use crate::editor;
use crate::input::{step_speed, submit};
use crate::palette::Interface;
use crate::view;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sim::world::TileState;
use sim::{CommandKind, PlanKind};

pub fn panels(
    mut contexts: EguiContexts,
    interface: Res<Interface>,
    mut driver: NonSendMut<DriverResource>,
    mut state: ResMut<ViewState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let ctx = ctx.clone();
    // Every panel descends from one root so they lay out against each
    // other, not each against the whole viewport.
    let mut root = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

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

    egui::Panel::left("editor")
        .exact_size(460.0)
        .show(&mut root, |ui| {
            editor::panel(ui, &mut driver, &mut state);
        });

    egui::Panel::right("inspector")
        .exact_size(320.0)
        .show(&mut root, |ui| {
            let d = &mut driver.0;
            let player = d.player;
            ui.heading("Tools");
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
            if ui.button("resign (Shift+R)").clicked() {
                submit(d, &mut state, CommandKind::Resign);
            }
            ui.separator();
            ui.heading("Inspector");
            let snap = d.snapshots.cur.clone();
            if let Some(id) = state.selected {
                match view::machine(&snap, id) {
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
                    match view::tile(&snap, player, p) {
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
            ui.separator();
            ui.small("left drag / middle / Shift+right: pan");
            ui.small("right drag: orbit · wheel: zoom");
            ui.small("WASD: pan · L: log · -/=: speed");
        });

    if state.show_log {
        egui::Panel::bottom("log")
            .exact_size(180.0)
            .show(&mut root, |ui| {
                ui.heading("Log");
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
            });
    }
}

fn amounts(m: &std::collections::BTreeMap<String, lang::Num>) -> String {
    m.iter()
        .map(|(k, v)| format!("{k} {}", v.to_decimal()))
        .collect::<Vec<_>>()
        .join(", ")
}
