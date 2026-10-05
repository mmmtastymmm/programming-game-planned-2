//! Before the first tick and after the last (`docs/07`, Q36): the start
//! screen that picks the map, the cards, the programs directory and play,
//! host or join, and the end screen over the frozen map that names the
//! result and where the replay went. [`crate::launch`] does the work; this
//! is only what the player sees of it.

use crate::app::{DriverResource, Screen, ViewState};
use crate::launch::{self, Match, Mode, Pending, Setup, Started, stem};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sim::TeamId;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModeKind {
    Play,
    Host,
    Join,
}

/// The start screen's form and the match it hands over. Non-send: a
/// started match holds the driver, and the driver holds the sim's `Rc`s.
pub struct StartScreen {
    setup: Setup,
    maps: Vec<PathBuf>,
    cards: Vec<PathBuf>,
    programs: String,
    kind: ModeKind,
    host_addr: String,
    join_addr: String,
    /// Start as soon as the screen is up: a flag was given (Q36).
    autostart: bool,
    error: Option<String>,
    pending: Option<Pending>,
    /// The started match, taken by [`install`] on entering [`Screen::Match`].
    launched: Option<Match>,
}

impl StartScreen {
    pub fn new(setup: Setup, autostart: bool) -> StartScreen {
        let root = launch::data_root();
        let (kind, host_addr, join_addr) = match &setup.mode {
            Mode::Play => (ModeKind::Play, None, None),
            Mode::Host(a) => (ModeKind::Host, Some(a.clone()), None),
            Mode::Join(a) => (ModeKind::Join, None, Some(a.clone())),
        };
        StartScreen {
            programs: setup
                .programs
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            maps: launch::list(&root.join("maps"), true),
            cards: launch::list(&root.join("opposition"), false),
            setup,
            kind,
            host_addr: host_addr.unwrap_or_else(|| "0.0.0.0:7777".into()),
            join_addr: join_addr.unwrap_or_else(|| "127.0.0.1:7777".into()),
            autostart,
            error: None,
            pending: None,
            launched: None,
        }
    }

    /// The setup the form now says.
    fn chosen(&self) -> Setup {
        let mut s = self.setup.clone();
        s.programs = match self.programs.trim() {
            "" => None,
            p => Some(PathBuf::from(p)),
        };
        s.mode = match self.kind {
            ModeKind::Play => Mode::Play,
            ModeKind::Host => Mode::Host(self.host_addr.trim().to_string()),
            ModeKind::Join => Mode::Join(self.join_addr.trim().to_string()),
        };
        if self.kind == ModeKind::Join {
            s.opposition.clear();
        }
        s
    }

    fn begin(&mut self, next: &mut NextState<Screen>) {
        self.error = None;
        match launch::start(&self.chosen()) {
            Ok(Started::Ready(m)) => self.launch(*m, next),
            Ok(Started::Waiting(p)) => self.pending = Some(*p),
            Err(e) => self.fail(e),
        }
    }

    fn launch(&mut self, m: Match, next: &mut NextState<Screen>) {
        self.launched = Some(m);
        next.set(Screen::Match);
    }

    fn fail(&mut self, e: String) {
        eprintln!("{e}");
        self.error = Some(e);
    }
}

pub fn start_screen(
    mut contexts: EguiContexts,
    mut start: NonSendMut<StartScreen>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
) {
    let start = &mut *start;
    if let Some(p) = &mut start.pending {
        match p.poll() {
            Some(Ok(m)) => {
                start.pending = None;
                start.launch(m, &mut next);
                return;
            }
            Some(Err(e)) => {
                start.pending = None;
                start.fail(e);
            }
            None => {}
        }
    } else if std::mem::take(&mut start.autostart) {
        start.begin(&mut next);
        if start.launched.is_some() {
            return;
        }
    }
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let waiting = start.pending.is_some();
    egui::Window::new("programming game")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_enabled_ui(!waiting, |ui| form(ui, start));
            ui.separator();
            if let Some(p) = &start.pending {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(&p.progress);
                });
            }
            if let Some(e) = &start.error {
                ui.colored_label(egui::Color32::LIGHT_RED, e);
            }
            ui.horizontal(|ui| {
                let label = match start.kind {
                    ModeKind::Play => "play",
                    ModeKind::Host => "host",
                    ModeKind::Join => "join",
                };
                if ui.add_enabled(!waiting, egui::Button::new(label)).clicked() {
                    start.begin(&mut next);
                }
                if ui.button("quit").clicked() {
                    exit.write(AppExit::Success);
                }
            });
        });
}

fn form(ui: &mut egui::Ui, start: &mut StartScreen) {
    egui::Grid::new("start")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("map");
            egui::ComboBox::from_id_salt("map")
                .selected_text(stem(&start.setup.map))
                .show_ui(ui, |ui| {
                    for m in &start.maps {
                        ui.selectable_value(&mut start.setup.map, m.clone(), stem(m));
                    }
                });
            ui.end_row();

            ui.label("programs");
            ui.add(
                egui::TextEdit::singleline(&mut start.programs)
                    .hint_text("none")
                    .desired_width(320.0),
            );
            ui.end_row();

            ui.label("match");
            ui.horizontal(|ui| {
                ui.radio_value(&mut start.kind, ModeKind::Play, "play alone");
                ui.radio_value(&mut start.kind, ModeKind::Host, "host");
                ui.radio_value(&mut start.kind, ModeKind::Join, "join");
            });
            ui.end_row();

            match start.kind {
                ModeKind::Play => {}
                ModeKind::Host => {
                    ui.label("listen on");
                    ui.text_edit_singleline(&mut start.host_addr);
                    ui.end_row();
                }
                ModeKind::Join => {
                    ui.label("host");
                    ui.text_edit_singleline(&mut start.join_addr);
                    ui.end_row();
                }
            }

            if start.kind != ModeKind::Join {
                ui.label("opposition");
                ui.vertical(|ui| {
                    let mut remove = None;
                    for (i, card) in start.setup.opposition.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("team {}: {}", i + 1, stem(card)));
                            if ui.small_button("remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        start.setup.opposition.remove(i);
                    }
                    if start.setup.opposition.is_empty() {
                        ui.weak(if start.kind == ModeKind::Host {
                            "none — peers take every other team"
                        } else {
                            "none — the other teams sit idle"
                        });
                    }
                    egui::ComboBox::from_id_salt("add card")
                        .selected_text("add a card")
                        .show_ui(ui, |ui| {
                            for c in &start.cards {
                                if ui.selectable_label(false, stem(c)).clicked() {
                                    start.setup.opposition.push(c.clone());
                                }
                            }
                        });
                });
                ui.end_row();
            }
        });
}

/// What the end screen reports, fixed when the match stopped.
#[derive(Resource)]
pub struct EndReport {
    /// The winner, or `None` for a draw; `None` too on a desync.
    pub ended: Option<Option<TeamId>>,
    pub desync: Option<String>,
    pub player: TeamId,
    pub tick: u64,
    pub hash: u64,
    pub replay: Result<PathBuf, String>,
}

/// The map file's stem for the match on screen, which names its replay.
#[derive(Resource)]
pub struct MatchInfo {
    pub map_stem: String,
}

/// Take the started match into the world on entering [`Screen::Match`]:
/// the driver, and fresh renderer state beside it.
pub fn install(world: &mut World) {
    let Some(m) = world.non_send_mut::<StartScreen>().launched.take() else {
        return;
    };
    let title = format!("programming game — {}", m.driver.map_name);
    let mut windows = world.query::<&mut Window>();
    for mut w in windows.iter_mut(world) {
        w.title = title.clone();
    }
    let layout_path = crate::layout::Layout::path(m.programs.as_deref());
    let mut state = ViewState {
        layout: crate::layout::Layout::load(layout_path.as_deref()),
        layout_path,
        editor: crate::editor::Editor::from_tree(&m.tree),
        programs: m.programs,
        deployed: m.deployed,
        ..Default::default()
    };
    if let Some(w) = m.warning {
        state.status = format!("started without programs: {w}");
    }
    world.insert_resource(crate::palette::Frame {
        min: m.driver.bounds.0,
        max: m.driver.bounds.1,
    });
    world.insert_resource(state);
    world.insert_resource(crate::view::Entities::default());
    world.insert_resource(MatchInfo {
        map_stem: m.map_stem,
    });
    world.insert_non_send(DriverResource(m.driver));
}

/// Drop the match on returning to [`Screen::Start`]; its entities go by
/// `DespawnOnEnter`.
pub fn teardown(world: &mut World) {
    world.remove_non_send::<DriverResource>();
    world.remove_resource::<EndReport>();
    world.remove_resource::<MatchInfo>();
}

/// The match is over — it ended, or two peers disagreed: save the replay
/// (Q36) and go to the end screen.
pub fn check_end(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    state: Res<ViewState>,
    info: Res<MatchInfo>,
    mut next: ResMut<NextState<Screen>>,
) {
    let d = &driver.0;
    if d.ended.is_none() && d.desync.is_none() {
        return;
    }
    let replay = launch::save_replay(d, state.programs.as_deref(), &info.map_stem);
    match &replay {
        Ok(p) => eprintln!("replay saved to {}", p.display()),
        Err(e) => eprintln!("replay not saved: {e}"),
    }
    commands.insert_resource(EndReport {
        ended: d.ended,
        desync: d.desync.clone(),
        player: d.player,
        tick: d.tick,
        hash: d.state_hash(),
        replay,
    });
    next.set(Screen::End);
}

pub fn end_screen(
    mut contexts: EguiContexts,
    report: Option<Res<EndReport>>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(r) = report else { return };
    let Ok(ctx) = contexts.ctx_mut() else { return };
    egui::Modal::new(egui::Id::new("end")).show(ctx, |ui| {
        ui.set_min_width(360.0);
        let headline = match (r.ended, &r.desync) {
            (_, Some(_)) => "Desync — no winner".to_string(),
            (Some(Some(t)), _) if t == r.player => format!("Team {} wins — you", t.0),
            (Some(Some(t)), _) => format!("Team {} wins", t.0),
            _ => "A draw".to_string(),
        };
        ui.heading(headline);
        if let Some(d) = &r.desync {
            ui.colored_label(egui::Color32::LIGHT_RED, d);
        }
        ui.label(format!("ended on tick {}", r.tick));
        ui.monospace(format!("hash {:016x}", r.hash));
        match &r.replay {
            Ok(p) => ui.label(format!("replay saved to {}", p.display())),
            Err(e) => ui.colored_label(egui::Color32::LIGHT_RED, format!("replay not saved: {e}")),
        };
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("again").clicked() {
                next.set(Screen::Start);
            }
            if ui.button("quit").clicked() {
                exit.write(AppExit::Success);
            }
        });
    });
}
