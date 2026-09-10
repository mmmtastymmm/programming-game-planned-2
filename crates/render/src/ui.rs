//! The panels: the tick and speed, the stall report, the tools, the deploy
//! and resign buttons, the selected machine's record, and the log view.
//! Every button becomes a command the renderer submits and forgets.

use crate::app::{DriverResource, Tool, ViewState};
use crate::input::{deploy, submit};
use crate::view;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sim::{CommandKind, PlanKind};

pub fn panels(
    mut contexts: EguiContexts,
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
    let d = &mut driver.0;

    egui::Panel::top("top").show(&mut root, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("tick {}", d.tick));
            ui.separator();
            ui.label(format!("speed {} /s", d.speed));
            if ui.small_button("−").clicked() {
                step_speed(d, &mut state, false);
            }
            if ui.small_button("+").clicked() {
                step_speed(d, &mut state, true);
            }
            ui.separator();
            ui.label(format!("delay {} ticks", d.delay));
            ui.separator();
            ui.label(format!("hash {:016x}", d.state_hash()));
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
            ui.label(&state.status);
        }
    });

    egui::Panel::left("tools")
        .exact_size(240.0)
        .show(&mut root, |ui| {
            ui.heading("tools");
            let mut tool = state.tool.clone();
            ui.radio_value(&mut tool, Tool::Select, "select (Esc)");
            ui.radio_value(
                &mut tool,
                Tool::Building("depot".into()),
                "building plan: depot (1)",
            );
            ui.radio_value(&mut tool, Tool::Paint("red".into()), "paint plan: red (2)");
            ui.radio_value(&mut tool, Tool::Overlay("a".into()), "overlay plan: a (3)");
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
            if ui.button("deploy programs (Shift+D)").clicked() {
                deploy(d, &mut state);
            }
            if let Some(p) = &state.programs {
                ui.small(format!("{}", p.display()));
            }
            ui.separator();
            if ui.button("resign (Shift+R)").clicked() {
                submit(d, &mut state, CommandKind::Resign);
            }
            ui.separator();
            ui.heading("teams");
            for t in &d.snapshots.cur.teams {
                let mine = if t.id == d.player { " (you)" } else { "" };
                let out = if t.out { " — out" } else { "" };
                ui.label(format!(
                    "team {}{mine}: {} bots / cap {}{out}",
                    t.id.0, t.bots, t.cap
                ));
                for (name, v) in &t.deployments {
                    if let Some(v) = v {
                        ui.small(format!("  {name}: {v:016x}"));
                    }
                }
            }
            if let Some(h) = state.hover {
                ui.separator();
                ui.label(format!("tile ({}, {})", h.x, h.y));
            }
            if let Some(id) = state.selected {
                ui.separator();
                ui.heading("selected");
                match view::machine(&d.snapshots.cur, id) {
                    Some(m) => {
                        let r = &m.record;
                        ui.label(format!(
                            "{} #{} — {} of team {}",
                            r.name,
                            r.id.0,
                            r.model.name(),
                            r.team.0
                        ));
                        ui.label(format!(
                            "pos ({}, {}) health {}",
                            r.pos.x,
                            r.pos.y,
                            r.health.to_decimal()
                        ));
                        ui.label(format!("busy {:?} progress {}", r.busy, r.progress));
                        if let Some(l) = &r.load {
                            ui.label(format!("load {}", amounts(l)));
                        }
                        if let Some(s) = &r.store {
                            ui.label(format!("store {}", amounts(s)));
                        }
                        if let Some(f) = &m.fault {
                            let what =
                                f.exception
                                    .as_ref()
                                    .map(|e| e.display())
                                    .unwrap_or_else(|| {
                                        format!("hook budget exhausted ({:?})", f.exhausted)
                                    });
                            ui.colored_label(
                                egui::Color32::LIGHT_RED,
                                format!(
                                    "fault: {what} at {}:{} tick {}",
                                    f.file.as_deref().unwrap_or("?"),
                                    f.line,
                                    f.tick
                                ),
                            );
                        }
                    }
                    None => {
                        ui.label("gone");
                    }
                }
            }
            ui.separator();
            ui.small("WASD/arrows pan · scroll zoom · L toggles the log");
        });

    if state.show_log {
        egui::Panel::bottom("log")
            .exact_size(180.0)
            .show(&mut root, |ui| {
                ui.heading("log");
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

fn step_speed(d: &mut crate::driver::Driver, state: &mut ViewState, up: bool) {
    let steps = sim::Data::load()
        .map(|x| x.world.speed_steps)
        .unwrap_or_default();
    let idx = steps.iter().position(|s| *s == d.speed).unwrap_or(0);
    let next = if up {
        (idx + 1).min(steps.len().saturating_sub(1))
    } else {
        idx.saturating_sub(1)
    };
    if let Some(s) = steps.get(next) {
        submit(d, state, CommandKind::SetSpeed(*s));
    }
}
