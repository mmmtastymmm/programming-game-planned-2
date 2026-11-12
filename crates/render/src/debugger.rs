//! The debugger's windows (`docs/07`, Debugging): the breakpoint column a
//! file window and a version window share, the read-only version windows,
//! the debugger window with the fork's steps and what it holds of the
//! selected machine, and the banner over a map that draws a fork.
//!
//! Everything here reads the driver's debugger half ([`crate::debug`]) and
//! asks it for steps; the fork it drives is a private copy that never
//! reaches the match (`CLAUDE.md`, rule 1). The one thing these windows
//! give the match's sim is the team's watches, which change nothing.

use crate::app::{Tool, ViewState};
use crate::breakpoints::{self, Breakpoints, Catalog, Text};
use crate::debug::{flags_text, position_text, stop_text};
use crate::driver::Driver;
use crate::editor;
use crate::whatif::Tried;
use bevy::prelude::KeyCode;
use bevy_egui::egui;
use lang::EditTarget;
use sim::{EntityId, Hit, Inspected, Step};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The debugger window's name in the layout and the dock.
pub const WINDOW: &str = "debugger";

/// The most characters of one value the debugger window shows.
const VALUE_CHARS: usize = 200;

/// What the interface holds for debugging, beside the driver's half.
#[derive(Debug, Default)]
pub struct DebugUi {
    /// The team's own versions, from its deploys.
    pub catalog: Catalog,
    pub breakpoints: Breakpoints,
    /// The version windows open, in the order they were opened.
    pub windows: Vec<Text>,
    /// A line a version window brings into view on its next frame.
    pub reveal: Option<(Text, u32)>,
    /// The variable being edited, with the literal typed so far.
    pub editing: Option<(EditTarget, String)>,
    /// A listed hit whose reopening waits on the player's word, the open
    /// fork being diverged.
    pub confirm: Option<Hit>,
    /// The last refusal, shown in the debugger window.
    pub note: String,
    /// The fork's selected machine as the fork holds it, read once a
    /// frame.
    pub inspected: Option<Inspected>,
    /// What each file's last *try* sent into the open fork, by tree path
    /// (`docs/07`, What-if): what *take it* deploys. It goes with the
    /// fork.
    pub tried: BTreeMap<String, Tried>,
    /// The forks opened when the windows last looked, to notice a new one.
    opened_seen: u64,
    /// The speed ladder a run steps along, read once from `data/`.
    speeds: Vec<u64>,
}

impl DebugUi {
    /// Open the version window on `text`, once.
    pub fn open(&mut self, text: Text) {
        if !self.windows.contains(&text) {
            self.windows.push(text);
        }
    }
}

/// A step the debugger window offers (`docs/07`, Stepping), with the key
/// the build chose for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Back,
    Continue,
    /// *Run*, or its pause while it is going.
    Run,
    Line,
    Over,
    Tick,
}

impl Action {
    pub const ALL: [Action; 6] = [
        Action::Continue,
        Action::Run,
        Action::Line,
        Action::Over,
        Action::Tick,
        Action::Back,
    ];

    /// The key the build chose: function keys, which no text box takes.
    pub fn code(self) -> KeyCode {
        match self {
            Action::Back => KeyCode::F4,
            Action::Continue => KeyCode::F5,
            Action::Run => KeyCode::F6,
            Action::Line => KeyCode::F7,
            Action::Over => KeyCode::F8,
            Action::Tick => KeyCode::F9,
        }
    }

    /// The key as a button names it.
    pub fn key(self) -> String {
        format!("{:?}", self.code())
    }

    pub fn name(self, running: bool) -> &'static str {
        match self {
            Action::Back => "back",
            Action::Continue => "continue",
            Action::Run if running => "pause",
            Action::Run => "run",
            Action::Line => "line",
            Action::Over => "over",
            Action::Tick => "tick",
        }
    }
}

/// Whether the step is offered where the fork stands, or why not: a run
/// that is going takes only its pause, *line* and *over* step the
/// selected machine, and *back* is not offered at a fork's first stop.
pub fn offered(d: &Driver, action: Action) -> Result<(), String> {
    let s = d.debug.session.as_ref().ok_or("no fork is open")?;
    if s.running {
        return match action {
            Action::Run => Ok(()),
            _ => Err("pause the run first".into()),
        };
    }
    match action {
        Action::Line | Action::Over if s.selected.is_none() => {
            Err("select a machine of yours: line and over step one machine".into())
        }
        Action::Back if !s.can_back() => {
            Err("the fork's first stop: there is nowhere to go back to".into())
        }
        _ => Ok(()),
    }
}

/// Take the step, if it is offered.
pub fn act(d: &mut Driver, action: Action) -> Result<(), String> {
    offered(d, action)?;
    let running = d.debug.session.as_ref().is_some_and(|s| s.running);
    match action {
        Action::Back => d.fork_back().map(drop),
        Action::Continue => d.fork_step(Step::Continue).map(drop),
        Action::Run if running => d.fork_pause().map(drop),
        Action::Run => d.fork_run(),
        Action::Line => d.fork_step(Step::Line).map(drop),
        Action::Over => d.fork_step(Step::Over).map(drop),
        Action::Tick => d.fork_step(Step::Tick).map(drop),
    }
}

/// The speed one step along the ladder from `now`: the match's speeds
/// without its pause, since a run has a pause of its own. A speed off the
/// ladder steps to the nearest one on it in that direction.
pub fn step_run_speed(steps: &[u64], now: u64, up: bool) -> Option<u64> {
    let mut speeds = steps.iter().copied().filter(|s| *s > 0);
    if up {
        speeds.find(|s| *s > now)
    } else {
        speeds.rfind(|s| *s < now)
    }
}

/// Once a frame, before the windows draw: take in the team's deploys, let
/// the breakpoints follow the tree, give the driver the watches they
/// make, and read the fork's selected machine. While the map draws a fork
/// the mark tools rest and the selection is the fork's. Returns whether a
/// fork opened since the last frame, for the debugger window to open on.
pub fn sync(d: &mut Driver, state: &mut ViewState) -> bool {
    let Ok(team) = d.debug_team() else {
        return false;
    };
    state.debug.catalog.read(&d.deploys(team));
    let moved = std::mem::take(&mut state.editor.moved);
    state.debug.breakpoints.follow(&moved, &state.editor);
    let watches = state.debug.breakpoints.watches(team, &state.editor);
    if let Err(e) = d.set_watches(watches) {
        state.debug.note = e;
    }
    let kept = state.debug.breakpoints.kept();
    if kept != state.layout.breakpoints {
        state.layout.breakpoints = kept;
        state.layout_dirty = true;
    }
    if state.debug.speeds.is_empty() {
        state.debug.speeds = sim::Data::load()
            .map(|d| d.world.speed_steps)
            .unwrap_or_default();
    }
    let opened = d.debug.opened();
    let fresh = opened != state.debug.opened_seen;
    state.debug.opened_seen = opened;
    if fresh || d.debug.session.is_none() {
        state.debug.editing = None;
        state.debug.confirm = None;
        state.debug.tried.clear();
    }
    if fresh {
        state.debug.note.clear();
        // A fork opened by hand follows the machine the player had
        // selected, if it is one the fork lets them select.
        if d.debug
            .session
            .as_ref()
            .is_some_and(|s| s.selected.is_none())
        {
            let _ = d.fork_select(state.selected);
        }
    }
    if let Some(s) = d.debug.session.as_ref().filter(|_| d.draws_fork()) {
        state.selected = s.selected;
        state.tool = Tool::Select;
        state.drag = None;
    }
    state.debug.inspected = d.inspect(VALUE_CHARS);
    fresh && d.debug.session.is_some()
}

/// A click in the gutter of the file window on `path` (`docs/07`,
/// Breakpoints): set or clear a breakpoint on the tree's text.
pub fn click_file(d: &Driver, state: &mut ViewState, path: &str, line: u32) {
    let team = match d.debug_team() {
        Ok(team) => team,
        Err(e) => {
            state.status = e;
            return;
        }
    };
    let Some(doc) = state.editor.files.get(path) else {
        return;
    };
    let text = Text::of_tree(path, &doc.text);
    let scope = breakpoints::scope(&state.debug.catalog, d.drawn(), team, state.selected, &text);
    let lines = state
        .debug
        .breakpoints
        .files
        .entry(path.to_string())
        .or_default();
    if breakpoints::toggle(lines, line, state.editor.begins.get(path), scope).is_none() {
        state.status = format!("no statement begins on line {line} or after it");
    }
}

/// One breakpoint as the column draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    pub line: u32,
    /// A machine it is scoped to runs the text; greyed otherwise.
    pub live: bool,
    /// Scoped to one machine rather than to the team.
    pub scoped: bool,
}

/// The dots of a text's breakpoints, against the machines the map draws.
/// With `begins`, a file window's: each is drawn where its watch is, on
/// the statement it settles on, and greyed past the last one.
pub fn dots(
    d: &Driver,
    catalog: &Catalog,
    lines: Option<&breakpoints::Lines>,
    begins: Option<&BTreeSet<u32>>,
    text: &Text,
) -> Vec<Dot> {
    let (Some(lines), Ok(team)) = (lines, d.debug_team()) else {
        return Vec::new();
    };
    lines
        .iter()
        .map(|(line, scope)| {
            let settled = match begins {
                Some(b) => breakpoints::settle(*line, b),
                None => Some(*line),
            };
            let runs = || !breakpoints::runners(catalog, d.drawn(), team, text, *scope).is_empty();
            Dot {
                line: settled.unwrap_or(*line),
                live: settled.is_some() && runs(),
                scoped: scope.is_some(),
            }
        })
        .collect()
}

/// The lines of `text` the fork's selected machine has a frame on, the
/// top frame's last: where a window marks the stack.
pub fn frame_lines(d: &Driver, ui: &DebugUi, text: &Text) -> Vec<u32> {
    let (Some(inspected), Some(s)) = (&ui.inspected, &d.debug.session) else {
        return Vec::new();
    };
    let Some(program) = &inspected.program else {
        return Vec::new();
    };
    let runs = s
        .snapshots
        .cur
        .machines
        .iter()
        .find(|m| m.record.id == inspected.record.id)
        .is_some_and(|m| ui.catalog.runs(m, text));
    if !runs {
        return Vec::new();
    }
    program
        .frames
        .iter()
        .filter(|f| f.file == text.file)
        .map(|f| f.line)
        .collect()
}

/// The 1-based line whose rows `y` falls in, given each line's first
/// row's top: a wrapped line's later rows are its own.
pub fn line_at(tops: &[f32], y: f32) -> Option<u32> {
    let below = tops.iter().take_while(|top| **top <= y).count();
    (below > 0).then_some(below as u32)
}

/// The breakpoint column over `column`, beside a text box laid out as
/// `galley`: a dot per breakpoint, an arrow per stack frame, and the line
/// a click landed on.
pub fn marks_column(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    column: egui::Rect,
    id: egui::Id,
    dots: &[Dot],
    arrows: &[u32],
) -> Option<u32> {
    let mut tops = Vec::new();
    let mut heights = Vec::new();
    let mut starts = true;
    for row in &galley.rows {
        if starts {
            let r = row.rect().translate(galley_pos.to_vec2());
            tops.push(r.top());
            heights.push(r.height());
        }
        starts = row.ends_with_newline;
    }
    let response = ui
        .interact(column, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let clicked = response
        .clicked()
        .then(|| response.interact_pointer_pos())
        .flatten()
        .and_then(|pos| line_at(&tops, pos.y));
    let clip = ui.clip_rect();
    let place = |line: u32| {
        let i = (line as usize).checked_sub(1)?;
        let (top, h) = (*tops.get(i)?, *heights.get(i)?);
        (top + h >= clip.top() && top <= clip.bottom()).then_some((top + h / 2.0, h))
    };
    let x = column.left() + 7.0;
    let painter = ui.painter();
    for dot in dots {
        let Some((y, h)) = place(dot.line) else {
            continue;
        };
        let color = if dot.live {
            egui::Color32::from_rgb(214, 62, 62)
        } else {
            egui::Color32::from_gray(110)
        };
        let r = (h * 0.3).min(5.0);
        painter.circle_filled(egui::pos2(x, y), r, color);
        if dot.scoped {
            let ring = egui::Stroke::new(1.0, egui::Color32::WHITE);
            painter.circle_stroke(egui::pos2(x, y), r + 1.5, ring);
        }
    }
    for (i, line) in arrows.iter().enumerate() {
        let Some((y, h)) = place(*line) else {
            continue;
        };
        // The top frame is the last and the brightest.
        let color = if i + 1 == arrows.len() {
            egui::Color32::from_rgb(255, 214, 90)
        } else {
            egui::Color32::from_rgb(150, 130, 70)
        };
        let r = (h * 0.3).min(5.0);
        let points = vec![
            egui::pos2(x - r * 0.6, y - r),
            egui::pos2(x + r, y),
            egui::pos2(x - r * 0.6, y + r),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
    }
    clicked
}

/// The width the breakpoint column adds to a gutter.
pub const MARKS_WIDTH: f32 = 14.0;

/// One version window (`docs/07`, Version windows): a version's text,
/// read-only, with its breakpoints and the stack's lines. Returns whether
/// it stays open.
fn version_window(ctx: &egui::Context, d: &Driver, state: &mut ViewState, text: &Text) -> bool {
    let mut open = true;
    let marks = dots(
        d,
        &state.debug.catalog,
        state.debug.breakpoints.versions.get(text),
        None,
        text,
    );
    let arrows = frame_lines(d, &state.debug, text);
    let reveal = state
        .debug
        .reveal
        .take_if(|(t, _)| t == text)
        .map(|(_, line)| line);
    let mut clicked = None;
    egui::Window::new(format!("{} — read-only", text.label()))
        .id(egui::Id::new(("version", text)))
        .open(&mut open)
        .default_size([520.0, 420.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            let Some(listing) = state.debug.catalog.listing(text) else {
                ui.label("this version is not among the team's deploys");
                return;
            };
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            let layout_font = font.clone();
            let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
                let mut job = editor::highlight(buf.as_str(), layout_font.clone(), &[]);
                job.wrap.max_width = wrap;
                let galley: Arc<egui::Galley> = ui.fonts_mut(|f| f.layout_job(job));
                galley
            };
            let lines = listing.text.split('\n').count();
            let digits = lines.to_string().len().max(2);
            let digit_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
            let gutter_w = MARKS_WIDTH + digit_w * digits as f32 + 4.0;
            let mut shown: &str = &listing.text;
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let gutter = ui.allocate_space(egui::vec2(gutter_w, 0.0)).1;
                    let output = egui::TextEdit::multiline(&mut shown)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .show(ui);
                    editor::line_numbers(ui, &output, gutter.right(), font.clone(), None);
                    let across = output.response.response.rect;
                    let column = egui::Rect::from_x_y_ranges(gutter.x_range(), across.y_range());
                    clicked = marks_column(
                        ui,
                        &output.galley,
                        output.galley_pos,
                        column,
                        egui::Id::new(("version marks", text)),
                        &marks,
                        &arrows,
                    );
                    if let Some(line) = reveal {
                        let rect = editor::line_rect(
                            &output.galley,
                            output.galley_pos,
                            across,
                            &listing.text,
                            line,
                        );
                        ui.scroll_to_rect(rect, Some(egui::Align::Center));
                    }
                });
            });
        });
    if let Some(line) = clicked {
        click_version(d, state, text, line);
    }
    open
}

/// A click in a version window's gutter: set or clear a breakpoint on
/// the version's text.
fn click_version(d: &Driver, state: &mut ViewState, text: &Text, line: u32) {
    let Ok(team) = d.debug_team() else { return };
    let Some(listing) = state.debug.catalog.listing(text) else {
        return;
    };
    let scope = breakpoints::scope(&state.debug.catalog, d.drawn(), team, state.selected, text);
    let lines = state
        .debug
        .breakpoints
        .versions
        .entry(text.clone())
        .or_default();
    if breakpoints::toggle(lines, line, Some(&listing.begins), scope).is_none() {
        state.status = format!("no statement begins on line {line} or after it");
    }
}

/// The version windows and the fork's banner, over everything the layout
/// holds.
pub fn overlays(ctx: &egui::Context, d: &mut Driver, state: &mut ViewState) {
    let texts = state.debug.windows.clone();
    for text in &texts {
        if !version_window(ctx, d, state, text) {
            state.debug.windows.retain(|t| t != text);
        }
    }
    banner(ctx, d);
}

/// A machine as the debugger names it.
fn machine_name(d: &Driver, id: EntityId) -> String {
    let named = d.debug.session.as_ref().and_then(|s| {
        let m = s.snapshots.cur.machines.iter().find(|m| m.record.id == id);
        m.map(|m| m.record.name.clone())
    });
    match named {
        Some(name) => format!("{name} #{}", id.0),
        None => format!("machine #{}", id.0),
    }
}

/// The banner over a map that draws a fork (`docs/07`, A hit): its tick,
/// the machine, its state, and *show the match*. While the map shows the
/// match with a fork open, it says so and offers the way back.
fn banner(ctx: &egui::Context, d: &mut Driver) {
    let Some(s) = &d.debug.session else { return };
    let showing_match = d.debug.show_match;
    let at = position_text(s.fork.position());
    let who = match s.selected {
        Some(id) => machine_name(d, id),
        None => "no machine selected".to_string(),
    };
    let state = flags_text(s.flags);
    // The stall and desync banners sit just under the time bar.
    let y = if d.desync.is_some() || d.stall.is_some() {
        130.0
    } else {
        44.0
    };
    let mut flip = false;
    egui::Area::new(egui::Id::new(("banner", "fork")))
        .anchor(egui::Align2::CENTER_TOP, [0.0, y])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(28, 52, 96))
                .inner_margin(10)
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if showing_match {
                            ui.strong("The match");
                            ui.label(format!("a fork is open at {at}"));
                            flip = ui.button("show the fork").clicked();
                        } else {
                            ui.strong("A fork");
                            ui.label(format!("{at} · {who} · {state}"));
                            flip = ui.button("show the match").clicked();
                        }
                    });
                });
        });
    if flip {
        d.debug.show_match = !showing_match;
    }
}

/// The debugger window (`docs/07`): the fork's steps, why it stopped,
/// further hits, and the selected machine as the fork holds it.
pub fn window(ui: &mut egui::Ui, d: &mut Driver, state: &mut ViewState) {
    if let Err(e) = d.debug_team() {
        ui.label(e);
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if d.debug.session.is_none() {
                ui.label("No fork is open. A breakpoint that hits opens one.");
                if ui.button("open a fork").clicked()
                    && let Err(e) = d.open_fork()
                {
                    state.debug.note = e;
                }
                ui.small("click a window's gutter to set a breakpoint");
                breakpoint_list(ui, state);
            } else {
                controls(ui, d, state);
                further_hits(ui, d, state);
                breakpoint_list(ui, state);
                ui.separator();
                machine(ui, d, state);
            }
            if !state.debug.note.is_empty() {
                ui.colored_label(egui::Color32::LIGHT_RED, &state.debug.note);
            }
        });
}

/// Every breakpoint set, file windows' first, each with a way to clear
/// it: the one place a breakpoint on a version nothing runs any more can
/// be found again.
fn breakpoint_list(ui: &mut egui::Ui, state: &mut ViewState) {
    let b = &mut state.debug.breakpoints;
    let count: usize = b
        .files
        .values()
        .chain(b.versions.values())
        .map(|l| l.len())
        .sum();
    if count == 0 {
        return;
    }
    let mut clear_file = None;
    let mut clear_version = None;
    egui::CollapsingHeader::new(format!("breakpoints ({count})"))
        .default_open(false)
        .show(ui, |ui| {
            let row = |ui: &mut egui::Ui, name: &str, line: u32, scope: Option<EntityId>| {
                ui.horizontal(|ui| {
                    let scope = scope.map_or(String::new(), |id| format!(" · machine #{}", id.0));
                    ui.monospace(format!("{name}:{line}{scope}"));
                    ui.small_button("clear").clicked()
                })
                .inner
            };
            for (path, lines) in &b.files {
                for (line, scope) in lines {
                    if row(ui, path, *line, *scope) {
                        clear_file = Some((path.clone(), *line));
                    }
                }
            }
            for (text, lines) in &b.versions {
                for (line, scope) in lines {
                    if row(ui, &text.label(), *line, *scope) {
                        clear_version = Some((text.clone(), *line));
                    }
                }
            }
        });
    if let Some((path, line)) = clear_file
        && let Some(lines) = b.files.get_mut(&path)
    {
        lines.remove(&line);
    }
    if let Some((text, line)) = clear_version
        && let Some(lines) = b.versions.get_mut(&text)
    {
        lines.remove(&line);
    }
}

/// Why the fork stopped, its state, and the steps.
fn controls(ui: &mut egui::Ui, d: &mut Driver, state: &mut ViewState) {
    let Some(s) = &d.debug.session else { return };
    let running = s.running;
    let speed = s.run_speed;
    match s.stop().filter(|_| !running) {
        Some(stop) => {
            ui.strong(stop_text(stop));
            ui.label(format!("at {}", position_text(stop.position)));
        }
        None => {
            ui.strong(format!("running at {speed} ticks/s"));
            ui.label(format!("at {}", position_text(s.fork.position())));
        }
    }
    ui.label(format!("the fork is {}", flags_text(s.flags)));
    let mut wanted = None;
    ui.horizontal_wrapped(|ui| {
        for action in Action::ALL {
            let label = format!("{} ({})", action.name(running), action.key());
            match offered(d, action) {
                Ok(()) => {
                    if ui.button(label).clicked() {
                        wanted = Some(action);
                    }
                }
                Err(why) => {
                    ui.add_enabled(false, egui::Button::new(label))
                        .on_disabled_hover_text(why);
                }
            }
        }
    });
    if let Some(action) = wanted {
        state.debug.note = act(d, action).err().unwrap_or_default();
    }
    ui.horizontal(|ui| {
        ui.label("run at");
        let slower = step_run_speed(&state.debug.speeds, speed, false);
        let faster = step_run_speed(&state.debug.speeds, speed, true);
        let step = |ui: &mut egui::Ui, text: &str, to: Option<u64>| {
            let button = egui::Button::new(text).small();
            ui.add_enabled(to.is_some(), button).clicked().then_some(to)
        };
        let down = step(ui, "−", slower);
        ui.label(format!("{speed} ticks/s"));
        let up = step(ui, "+", faster);
        if let Some(to) = down.or(up).flatten() {
            let _ = d.set_run_speed(to);
        }
        ui.separator();
        let flip = if d.debug.show_match {
            "show the fork"
        } else {
            "show the match"
        };
        if ui.button(flip).clicked() {
            d.debug.show_match = !d.debug.show_match;
        }
        if ui.button("close the fork").clicked() {
            d.close_fork();
        }
    });
}

/// The hits listed while the fork is open, the latest first, each with
/// how many more its machine made that tick and a way to reopen there.
/// Reopening a diverged fork asks first: its changes go with it.
fn further_hits(ui: &mut egui::Ui, d: &mut Driver, state: &mut ViewState) {
    let Some(s) = &d.debug.session else { return };
    let diverged = s.flags.diverged;
    let hits: Vec<Hit> = d.debug.hits.iter().map(|l| l.hit.clone()).collect();
    if hits.is_empty() {
        state.debug.confirm = None;
        return;
    }
    let mut reopen = None;
    egui::CollapsingHeader::new(format!("further hits ({})", hits.len()))
        .default_open(true)
        .show(ui, |ui| {
            for hit in &hits {
                ui.horizontal(|ui| {
                    let more = match hit.more {
                        0 => String::new(),
                        n => format!(" (+{n} more)"),
                    };
                    ui.label(format!(
                        "tick {} · {} · {}:{}{more}",
                        hit.position.tick(),
                        machine_name(d, hit.machine),
                        hit.file,
                        hit.line
                    ));
                    if ui.small_button("reopen").clicked() {
                        if diverged {
                            state.debug.confirm = Some(hit.clone());
                        } else {
                            reopen = Some(hit.clone());
                        }
                    }
                });
            }
        });
    if let Some(hit) = state.debug.confirm.clone() {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!(
                "The fork is diverged: reopening at tick {} discards what you changed in it.",
                hit.position.tick()
            ),
        );
        ui.horizontal(|ui| {
            if ui.button("reopen").clicked() {
                reopen = Some(hit);
                state.debug.confirm = None;
            }
            if ui.button("keep this fork").clicked() {
                state.debug.confirm = None;
            }
        });
    }
    if let Some(hit) = reopen {
        let listed = d.debug.hits.iter().position(|l| l.hit == hit);
        let done = match listed {
            Some(i) => d.reopen(i),
            None => Err("that hit is no longer listed".into()),
        };
        state.debug.note = done.err().unwrap_or_default();
    }
}

/// The selected machine as the fork holds it: its record, its budget,
/// interrupts and handler, its stack with each frame's version, and the
/// globals of each module; a local or a global is edited in place where
/// the fork allows it.
fn machine(ui: &mut egui::Ui, d: &mut Driver, state: &mut ViewState) {
    let Some(inspected) = state.debug.inspected.clone() else {
        ui.label("Select a machine of yours on the fork's map to inspect it.");
        return;
    };
    let r = &inspected.record;
    ui.strong(format!("{} — {} #{}", r.name, r.model.name(), r.id.0));
    ui.label(format!(
        "at ({}, {}) · health {}",
        r.pos.x,
        r.pos.y,
        r.health.to_decimal()
    ));
    match r.busy {
        Some(b) => ui.label(format!("busy: {b} ({})", r.progress)),
        None => ui.label("idle"),
    };
    let Some(p) = &inspected.program else {
        ui.label("it runs no program");
        return;
    };
    ui.label(format!(
        "budget {} · spent {} · deficit {}",
        p.budget, p.spent, p.deficit
    ));
    let pending: Vec<&str> = p.pending.iter().map(|i| i.name()).collect();
    ui.label(if pending.is_empty() {
        "no interrupt pending".to_string()
    } else {
        format!("pending: {}", pending.join(", "))
    });
    match &p.handler {
        Some(h) => ui.label(format!(
            "in the {} handler · hook budget {} of {} spent",
            h.kind.name(),
            h.hook_spent,
            h.hook_budget
        )),
        None => ui.label("in main flow"),
    };
    if p.waiting {
        ui.small("waiting on an action");
    }
    if p.dead {
        ui.small("dead");
    }
    // Editing is not offered where the fork would refuse it: at a stop in
    // step 4, of a machine at its death, in a live PvP match.
    let editable = d.can_edit();
    if let Err(why) = &editable {
        ui.small(format!("values are read-only here: {why}"));
    }
    let editable = editable.is_ok();
    let texts = d
        .drawn()
        .machines
        .iter()
        .find(|m| m.record.id == r.id)
        .map(|m| state.debug.catalog.texts_of(m))
        .unwrap_or_default();
    let mut edit = None;
    ui.separator();
    ui.strong("stack");
    for (depth, frame) in p.frames.iter().enumerate().rev() {
        ui.horizontal(|ui| {
            ui.monospace(format!(
                "{} — {}:{}",
                frame.function, frame.file, frame.line
            ));
            if let Some(text) = texts.iter().find(|t| t.file == frame.file)
                && ui.small_button("view").clicked()
            {
                state.debug.reveal = Some((text.clone(), frame.line));
                state.debug.open(text.clone());
            }
        });
        for (name, value) in &frame.locals {
            let target = EditTarget::Local {
                depth,
                name: name.clone(),
            };
            let shown = value.as_deref().unwrap_or("(not bound yet)");
            variable(ui, state, editable, target, name, shown, &mut edit);
        }
    }
    for module in &p.modules {
        ui.separator();
        ui.strong(format!("globals of {}", module.name));
        for (name, value) in &module.globals {
            let target = EditTarget::Global {
                module: module.name.clone(),
                name: name.clone(),
            };
            variable(ui, state, editable, target, name, value, &mut edit);
        }
    }
    if let Some((target, literal)) = edit {
        match d.fork_edit(target, &literal) {
            Ok(()) => {
                state.debug.editing = None;
                state.debug.note.clear();
            }
            Err(e) => state.debug.note = e,
        }
    }
}

/// One variable's row: its value, and a field to set it while it is the
/// one being edited. A confirmed edit is left in `edit`.
fn variable(
    ui: &mut egui::Ui,
    state: &mut ViewState,
    editable: bool,
    target: EditTarget,
    name: &str,
    value: &str,
    edit: &mut Option<(EditTarget, String)>,
) {
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        match &mut state.debug.editing {
            Some((editing, literal)) if *editing == target && editable => {
                ui.monospace(format!("{name} ="));
                let field = ui.add(
                    egui::TextEdit::singleline(literal)
                        .code_editor()
                        .desired_width(160.0),
                );
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.small_button("set").clicked() || entered {
                    *edit = Some((target, literal.clone()));
                } else if ui.small_button("cancel").clicked() {
                    state.debug.editing = None;
                }
            }
            _ => {
                ui.monospace(format!("{name} = {value}"));
                if editable && ui.small_button("edit").clicked() {
                    state.debug.editing = Some((target, value.to_string()));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Drag;
    use crate::breakpoints::Lines;
    use crate::debug::tests::{
        BOT, COUNTER, PRINTER, driver, driver_running, recorded, run_to, session, tuning, watch,
        x_of_main,
    };
    use crate::editor::Editor;
    use sim::script::Tree;
    use sim::{Cause, PlanKind, Position, Watch};

    /// The rival's printer.
    const RIVAL: EntityId = EntityId(2);

    /// The interface over a tree whose robots are `main`.
    fn state_running(main: &str) -> ViewState {
        let mut t = Tree::new();
        t.insert("robots/printer.py".into(), main.into());
        t.insert("robots/bot.py".into(), main.into());
        ViewState {
            editor: Editor::from_tree(&t),
            ..Default::default()
        }
    }

    /// The interface over the tree the tests' match was opened from.
    fn state() -> ViewState {
        state_running(COUNTER)
    }

    fn printer_text() -> Text {
        Text::of_tree("robots/printer.py", COUNTER)
    }

    fn bot_text() -> Text {
        Text::of_tree("robots/bot.py", COUNTER)
    }

    fn on(deployment: &str, line: u32, machine: Option<EntityId>) -> Watch {
        Watch {
            deployment: Some(deployment.into()),
            machine,
            ..watch(line)
        }
    }

    fn lines(of: &[(u32, Option<EntityId>)]) -> Lines {
        of.iter().copied().collect()
    }

    fn paint() -> Tool {
        Tool::Mark(PlanKind::Paint, None)
    }

    #[test]
    fn each_step_has_a_key_and_a_name_of_its_own() {
        let keys: Vec<String> = Action::ALL.iter().map(|a| a.key()).collect();
        assert_eq!(keys, ["F5", "F6", "F7", "F8", "F9", "F4"]);
        let codes: BTreeSet<String> = keys.into_iter().collect();
        assert_eq!(codes.len(), 6, "no two steps share a key");
        let names: BTreeSet<&str> = Action::ALL.iter().map(|a| a.name(false)).collect();
        assert_eq!(names.len(), 6);
        assert_eq!(
            (Action::Run.name(false), Action::Run.name(true)),
            ("run", "pause")
        );
        for a in Action::ALL.into_iter().filter(|a| *a != Action::Run) {
            assert_eq!(a.name(true), a.name(false));
        }
        assert_eq!(Action::Line.name(false), "line");
        assert_eq!(Action::Over.name(false), "over");
        assert_eq!(Action::Tick.name(false), "tick");
        assert_eq!(Action::Continue.name(false), "continue");
        assert_eq!(Action::Back.name(false), "back");
    }

    #[test]
    fn a_step_is_offered_and_taken_only_where_the_fork_can_take_it() {
        let mut d = driver();
        run_to(&mut d, 38);
        for a in Action::ALL {
            let refused = offered(&d, a).unwrap_err();
            assert!(refused.contains("no fork"), "{refused}");
            assert_eq!(act(&mut d, a), Err(refused));
        }
        d.set_watches(vec![watch(3)]).unwrap();
        d.advance(0.1);
        let hit = session(&d).fork.position();
        for a in Action::ALL.into_iter().filter(|a| *a != Action::Back) {
            assert_eq!(offered(&d, a), Ok(()), "{a:?}");
        }
        let refused = offered(&d, Action::Back).unwrap_err();
        assert!(refused.contains("first stop"), "{refused}");
        assert_eq!(act(&mut d, Action::Back), Err(refused));
        assert_eq!(session(&d).fork.position(), hit);

        let cause = |d: &Driver| session(d).stop().unwrap().cause;
        act(&mut d, Action::Line).unwrap();
        assert_eq!(cause(&d), Cause::Statement);
        let line = session(&d).fork.position();
        assert!(line > hit);
        assert_eq!(offered(&d, Action::Back), Ok(()));
        act(&mut d, Action::Back).unwrap();
        assert_eq!(session(&d).fork.position(), hit);
        act(&mut d, Action::Over).unwrap();
        assert_eq!(
            (cause(&d), session(&d).fork.position()),
            (Cause::Statement, line)
        );
        act(&mut d, Action::Tick).unwrap();
        let end = Position::End { tick: 39 };
        assert_eq!(
            (cause(&d), session(&d).fork.position()),
            (Cause::TickEnd, end)
        );
        act(&mut d, Action::Continue).unwrap();
        assert_eq!(cause(&d), Cause::Watch);
        assert_eq!(session(&d).fork.position().tick(), 40);

        // *Line* and *over* step one machine: with none selected they are
        // not offered, and the rest are.
        d.fork_select(None).unwrap();
        let at = session(&d).fork.position();
        for a in [Action::Line, Action::Over] {
            let refused = offered(&d, a).unwrap_err();
            assert!(refused.contains("select a machine"), "{refused}");
            assert_eq!(act(&mut d, a), Err(refused));
        }
        assert_eq!(session(&d).fork.position(), at);
        for a in [Action::Tick, Action::Continue, Action::Run, Action::Back] {
            assert_eq!(offered(&d, a), Ok(()), "{a:?}");
        }

        // A run that is going takes only its pause.
        act(&mut d, Action::Run).unwrap();
        assert!(session(&d).running);
        for a in Action::ALL.into_iter().filter(|a| *a != Action::Run) {
            let refused = offered(&d, a).unwrap_err();
            assert!(refused.contains("pause the run"), "{refused}");
            assert_eq!(act(&mut d, a), Err(refused));
        }
        assert!(session(&d).running);
        assert_eq!(offered(&d, Action::Run), Ok(()));
        act(&mut d, Action::Run).unwrap();
        assert!(!session(&d).running);
        assert_eq!(cause(&d), Cause::Paused);
    }

    #[test]
    fn a_gutter_click_becomes_the_teams_watch_and_is_kept_with_the_tree() {
        let mut d = driver();
        run_to(&mut d, 38);
        let mut st = state();
        assert!(!sync(&mut d, &mut st));
        assert!(d.debug.watches().is_empty());
        assert!(!st.layout_dirty, "nothing to keep");

        click_file(&d, &mut st, "robots/printer.py", 3);
        assert!(!sync(&mut d, &mut st));
        assert_eq!(d.debug.watches(), [on("printer", 3, None)]);
        let kept: BTreeMap<String, Vec<u32>> = [("robots/printer.py".to_string(), vec![3])].into();
        assert_eq!(st.layout.breakpoints, kept);
        assert!(st.layout_dirty);
        st.layout_dirty = false;
        assert!(!sync(&mut d, &mut st));
        assert!(!st.layout_dirty, "the same breakpoints are not saved again");

        // The watch is on the tree's text as it stands: an edit is
        // another text, which nothing runs until it is deployed.
        let edited = format!("{COUNTER}y = 1\n");
        st.editor.files.get_mut("robots/printer.py").unwrap().text = edited.clone();
        st.editor.recompose();
        sync(&mut d, &mut st);
        let content = lang::content_hash(edited.as_bytes());
        let moved = Watch {
            content,
            ..on("printer", 3, None)
        };
        assert_eq!(d.debug.watches(), [moved]);
        assert!(!st.layout_dirty, "the line did not move");

        // A second click clears it, there and in what is kept.
        click_file(&d, &mut st, "robots/printer.py", 3);
        sync(&mut d, &mut st);
        assert!(d.debug.watches().is_empty());
        assert!(st.layout.breakpoints.is_empty() && st.layout_dirty);

        // A renamed file keeps its breakpoints under its new path, and
        // its watch goes by its new name.
        st.editor.add_file("lib/geo.py").unwrap();
        st.editor.files.get_mut("lib/geo.py").unwrap().text = "def one(n):\n    return n\n".into();
        st.editor.recompose();
        click_file(&d, &mut st, "lib/geo.py", 2);
        sync(&mut d, &mut st);
        let files = |d: &Driver| -> Vec<String> {
            d.debug.watches().iter().map(|w| w.file.clone()).collect()
        };
        assert_eq!(files(&d), ["geo.py"]);
        st.layout_dirty = false;
        st.editor.rename("lib/geo.py", "lib/nav.py").unwrap();
        sync(&mut d, &mut st);
        assert!(st.editor.moved.is_empty(), "taken");
        assert_eq!(files(&d), ["nav.py"]);
        let kept: Vec<&str> = st.layout.breakpoints.keys().map(String::as_str).collect();
        assert_eq!(kept, ["lib/nav.py"]);
        assert!(st.layout_dirty);
        st.editor.delete("lib/nav.py").unwrap();
        sync(&mut d, &mut st);
        assert!(d.debug.watches().is_empty() && st.layout.breakpoints.is_empty());
    }

    #[test]
    fn a_fork_opened_and_closed_between_two_frames_opens_no_window() {
        let mut d = driver();
        run_to(&mut d, 10);
        let mut st = state();
        sync(&mut d, &mut st);
        d.open_fork().unwrap();
        d.close_fork();
        assert!(!sync(&mut d, &mut st));
        d.open_fork().unwrap();
        assert!(sync(&mut d, &mut st));
        assert!(!sync(&mut d, &mut st));
    }

    #[test]
    fn a_match_begins_with_the_breakpoints_kept_beside_its_tree() {
        let dir = std::env::temp_dir().join(format!("render-kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = driver();
        run_to(&mut d, 10);
        let mut st = state();
        st.layout_path = crate::layout::Layout::path(Some(&dir));
        st.selected = Some(PRINTER);
        sync(&mut d, &mut st);
        click_file(&d, &mut st, "robots/printer.py", 3);
        sync(&mut d, &mut st);
        assert_eq!(d.debug.watches(), [on("printer", 3, Some(PRINTER))]);
        assert!(st.layout_dirty);
        st.layout.save(st.layout_path.as_ref().unwrap()).unwrap();

        let mut t = Tree::new();
        t.insert("robots/printer.py".into(), COUNTER.into());
        t.insert("robots/bot.py".into(), COUNTER.into());
        let mut next = ViewState::started(&t, Some(dir.clone()), BTreeMap::new());
        assert_eq!(next.layout_path, st.layout_path);
        assert_eq!(next.debug.breakpoints.files, {
            let mut m = BTreeMap::new();
            m.insert("robots/printer.py".to_string(), lines(&[(3, None)]));
            m
        });
        // The scope is not kept: the machine was that match's.
        let mut d = driver();
        run_to(&mut d, 10);
        sync(&mut d, &mut next);
        assert_eq!(d.debug.watches(), [on("printer", 3, None)]);
        assert!(!next.layout_dirty, "nothing to write back");
        // With no programs directory there is nothing kept.
        let bare = ViewState::started(&t, None, BTreeMap::new());
        assert!(bare.debug.breakpoints.files.is_empty() && bare.layout_path.is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_window_opens_once_on_a_hit_and_the_map_takes_the_forks_selection() {
        let mut d = driver();
        run_to(&mut d, 38);
        let mut st = state();
        click_file(&d, &mut st, "robots/printer.py", 3);
        st.selected = Some(BOT);
        st.tool = paint();
        st.drag = Some(Drag::default());
        assert!(!sync(&mut d, &mut st));
        assert_eq!((st.selected, &st.tool), (Some(BOT), &paint()));
        assert!(st.drag.is_some(), "the match's map takes marks");
        assert!(st.debug.inspected.is_none());

        st.debug.note = "an old refusal".into();
        d.advance(0.1);
        assert!(sync(&mut d, &mut st), "the hit opened a fork");
        assert_eq!(st.selected, Some(PRINTER), "the hit's machine");
        assert_eq!(st.tool, Tool::Select, "the mark tools rest");
        assert!(st.drag.is_none());
        assert_eq!(st.debug.note, "");
        let seen = st.debug.inspected.as_ref().expect("the fork's machine");
        assert_eq!(seen.record.id, PRINTER);
        assert!(!sync(&mut d, &mut st), "it opened once");

        // While the map draws the fork the selection is the fork's, and
        // no mark tool arms.
        st.selected = Some(BOT);
        st.tool = paint();
        sync(&mut d, &mut st);
        assert_eq!((st.selected, &st.tool), (Some(PRINTER), &Tool::Select));
        d.fork_select(Some(BOT)).unwrap();
        sync(&mut d, &mut st);
        assert_eq!(st.selected, Some(BOT));
        assert_eq!(st.debug.inspected.as_ref().map(|i| i.record.id), Some(BOT));

        // *Show the match*: the map is the match's again, and the window
        // still reads the fork.
        d.debug.show_match = true;
        st.selected = Some(PRINTER);
        st.tool = paint();
        assert!(!sync(&mut d, &mut st));
        assert_eq!((st.selected, &st.tool), (Some(PRINTER), &paint()));
        assert_eq!(st.debug.inspected.as_ref().map(|i| i.record.id), Some(BOT));

        // An edit half typed and a reopening half asked go with the fork.
        // The match runs on to its pause, and its further hits are listed.
        for _ in 0..3 {
            d.advance(0.1);
            assert!(!sync(&mut d, &mut st), "a listed hit opens nothing");
        }
        st.debug.editing = Some((x_of_main(), "5".into()));
        st.debug.confirm = Some(d.debug.hits[0].hit.clone());
        sync(&mut d, &mut st);
        assert!(st.debug.editing.is_some() && st.debug.confirm.is_some());
        d.reopen(0).unwrap();
        assert!(sync(&mut d, &mut st), "a reopening is a fork opened");
        assert!(st.debug.editing.is_none() && st.debug.confirm.is_none());
        st.debug.editing = Some((x_of_main(), "5".into()));
        d.close_fork();
        assert!(!sync(&mut d, &mut st));
        assert!(st.debug.editing.is_none() && st.debug.inspected.is_none());
    }

    #[test]
    fn a_fork_opened_by_hand_follows_the_selected_machine_if_it_is_the_teams() {
        let mut d = driver();
        run_to(&mut d, 40);
        let mut st = state();
        st.selected = Some(BOT);
        assert!(!sync(&mut d, &mut st));
        d.open_fork().unwrap();
        assert_eq!(session(&d).selected, None);
        assert!(sync(&mut d, &mut st));
        assert_eq!(session(&d).selected, Some(BOT));
        assert_eq!(st.selected, Some(BOT));
        assert_eq!(st.debug.inspected.as_ref().map(|i| i.record.id), Some(BOT));
        // Only once: what the player selects in the fork afterwards holds.
        d.fork_select(None).unwrap();
        assert!(!sync(&mut d, &mut st));
        assert_eq!((session(&d).selected, st.selected), (None, None));

        // Another team's machine is not the fork's to follow.
        d.close_fork();
        sync(&mut d, &mut st);
        st.selected = Some(RIVAL);
        d.open_fork().unwrap();
        assert!(sync(&mut d, &mut st));
        assert_eq!((session(&d).selected, st.selected), (None, None));
        assert!(st.debug.inspected.is_none());
    }

    #[test]
    fn a_replay_is_debugged_as_the_team_it_records_or_only_watched() {
        let mut d = Driver::open_replay(recorded(), tuning()).unwrap();
        let mut st = state();
        click_file(&d, &mut st, "robots/printer.py", 3);
        assert_eq!(st.status, "");
        let mut opened = false;
        for _ in 0..200 {
            d.advance(0.1);
            if sync(&mut d, &mut st) {
                opened = true;
                break;
            }
        }
        assert!(opened, "the replay's breakpoint hit");
        assert_eq!(st.selected, Some(PRINTER));
        assert_eq!(frame_lines(&d, &st.debug, &printer_text()), vec![3]);

        let mut watched = recorded();
        watched.team = None;
        let mut d = Driver::open_replay(watched, tuning()).unwrap();
        let mut st = state();
        click_file(&d, &mut st, "robots/printer.py", 3);
        assert!(st.status.contains("watched, not debugged"), "{}", st.status);
        assert!(st.debug.breakpoints.files.is_empty());
        // A breakpoint the tree kept from a match is not this replay's.
        let kept = lines(&[(3, None)]);
        st.debug
            .breakpoints
            .files
            .insert("robots/printer.py".into(), kept.clone());
        for _ in 0..30 {
            d.advance(0.1);
            assert!(!sync(&mut d, &mut st));
        }
        assert!(d.debug.watches().is_empty() && d.debug.session.is_none());
        let none = dots(&d, &st.debug.catalog, Some(&kept), None, &printer_text());
        assert_eq!(none, Vec::new());
    }

    #[test]
    fn a_click_scopes_the_breakpoint_to_the_selected_machine_that_runs_the_text() {
        let mut d = driver();
        run_to(&mut d, 40);
        let mut st = state();
        sync(&mut d, &mut st);
        st.selected = Some(BOT);
        click_file(&d, &mut st, "robots/bot.py", 3);
        click_file(&d, &mut st, "robots/printer.py", 2);
        let files = |st: &ViewState, path: &str| st.debug.breakpoints.files.get(path).cloned();
        assert_eq!(files(&st, "robots/bot.py"), Some(lines(&[(3, Some(BOT))])));
        // The bot does not run the printer's file: every machine that does.
        assert_eq!(files(&st, "robots/printer.py"), Some(lines(&[(2, None)])));
        assert_eq!(st.status, "");
        sync(&mut d, &mut st);
        assert_eq!(
            d.debug.watches(),
            [on("printer", 2, None), on("bot", 3, Some(BOT))]
        );

        // Past the last statement there is nothing to stop at.
        click_file(&d, &mut st, "robots/bot.py", 9);
        assert_eq!(st.status, "no statement begins on line 9 or after it");
        assert_eq!(files(&st, "robots/bot.py"), Some(lines(&[(3, Some(BOT))])));
        // A file the tree does not hold takes none.
        click_file(&d, &mut st, "lib/none.py", 1);
        assert_eq!(files(&st, "lib/none.py"), None);

        // An edited file is not exactly the text the bot runs.
        let edited = format!("{COUNTER}\n\ny = 1\n");
        st.editor.files.get_mut("robots/bot.py").unwrap().text = edited;
        st.editor.recompose();
        click_file(&d, &mut st, "robots/bot.py", 5);
        let both = lines(&[(3, Some(BOT)), (7, None)]);
        assert_eq!(files(&st, "robots/bot.py"), Some(both), "settled on line 7");
        click_file(&d, &mut st, "robots/bot.py", 6);
        click_file(&d, &mut st, "robots/bot.py", 3);
        assert_eq!(files(&st, "robots/bot.py"), Some(Lines::new()));
    }

    #[test]
    fn a_breakpoint_is_drawn_where_its_watch_is_and_greyed_while_nothing_runs_it() {
        let mut d = driver();
        run_to(&mut d, 30);
        let mut st = state();
        sync(&mut d, &mut st);
        let catalog = &st.debug.catalog;
        let dot = |line, live, scoped| Dot { line, live, scoped };
        let set = lines(&[(2, None), (4, Some(PRINTER)), (9, None)]);
        let begins: BTreeSet<u32> = [1, 3, 4].into();
        // A file window's: on the statement each settles on.
        assert_eq!(
            dots(&d, catalog, Some(&set), Some(&begins), &printer_text()),
            vec![
                dot(3, true, false),
                dot(4, true, true),
                dot(9, false, false)
            ]
        );
        // Scoped to a machine that does not run the text, it is grey
        // though another machine runs it.
        let theirs = lines(&[(3, Some(RIVAL))]);
        assert_eq!(
            dots(&d, catalog, Some(&theirs), Some(&begins), &printer_text()),
            vec![dot(3, false, true)]
        );
        // A version window's lines are the version's own.
        assert_eq!(
            dots(&d, catalog, Some(&set), None, &printer_text()),
            vec![dot(2, true, false), dot(4, true, true), dot(9, true, false)]
        );
        assert_eq!(
            dots(&d, catalog, None, Some(&begins), &printer_text()),
            Vec::new()
        );
        // No bot is printed yet, and the printer runs no bot's file.
        assert_eq!(
            dots(&d, catalog, Some(&set), Some(&begins), &bot_text()),
            vec![
                dot(3, false, false),
                dot(4, false, true),
                dot(9, false, false)
            ]
        );
        // An edited text is run by nothing.
        let edited = Text::of_tree("robots/printer.py", "x = 1\n");
        let grey = dots(&d, catalog, Some(&set), None, &edited);
        assert!(grey.iter().all(|d| !d.live), "{grey:?}");

        // Against the machines the map draws: a fork of tick 30 has no
        // bot, the match a moment later has.
        d.open_fork().unwrap();
        let mut frames = 0;
        while d.tick < 35 {
            d.advance(0.1);
            frames += 1;
            assert!(frames < 100, "the match stood still at tick {}", d.tick);
        }
        let one = lines(&[(3, None)]);
        let drawn = |d: &Driver| dots(d, catalog, Some(&one), None, &bot_text());
        assert_eq!(drawn(&d), vec![dot(3, false, false)]);
        d.debug.show_match = true;
        assert_eq!(drawn(&d), vec![dot(3, true, false)]);
    }

    /// A call inside the loop: line 2 is begun two frames deep.
    const NESTED: &str =
        "def inner(n):\n    return n + 1\nx = 0\nwhile True:\n    x = inner(x)\n    wait(1)\n";

    #[test]
    fn the_stack_of_the_forks_machine_is_marked_in_the_text_it_runs() {
        let mut d = driver();
        run_to(&mut d, 38);
        let mut st = state();
        click_file(&d, &mut st, "robots/printer.py", 3);
        sync(&mut d, &mut st);
        assert_eq!(
            frame_lines(&d, &st.debug, &printer_text()),
            Vec::<u32>::new()
        );
        d.advance(0.1);
        assert!(sync(&mut d, &mut st));
        assert_eq!(frame_lines(&d, &st.debug, &printer_text()), vec![3]);
        // Not in a text the machine does not run, though it has the name.
        assert_eq!(frame_lines(&d, &st.debug, &bot_text()), Vec::<u32>::new());
        let edited = Text::of_tree("robots/printer.py", "x = 1\n");
        assert_eq!(frame_lines(&d, &st.debug, &edited), Vec::<u32>::new());
        let other = Text::of_bundle(None, "nav.py", printer_text().content);
        assert_eq!(frame_lines(&d, &st.debug, &other), Vec::<u32>::new());
        act(&mut d, Action::Line).unwrap();
        sync(&mut d, &mut st);
        assert_eq!(frame_lines(&d, &st.debug, &printer_text()), vec![4]);
        d.fork_select(None).unwrap();
        sync(&mut d, &mut st);
        assert_eq!(
            frame_lines(&d, &st.debug, &printer_text()),
            Vec::<u32>::new()
        );

        // Two frames deep: the call beneath, the top frame last.
        let mut d = driver_running(NESTED);
        run_to(&mut d, 10);
        let mut st = state_running(NESTED);
        click_file(&d, &mut st, "robots/printer.py", 2);
        sync(&mut d, &mut st);
        d.advance(0.1);
        assert!(sync(&mut d, &mut st));
        let text = Text::of_tree("robots/printer.py", NESTED);
        assert_eq!(frame_lines(&d, &st.debug, &text), vec![5, 2]);

        // *Over* runs the call, *line* goes into it, once no breakpoint
        // stands in the function to stop either.
        click_file(&d, &mut st, "robots/printer.py", 2);
        sync(&mut d, &mut st);
        assert!(d.debug.watches().is_empty());
        let mut stack = |d: &mut Driver, action| {
            act(d, action).unwrap();
            sync(d, &mut st);
            frame_lines(d, &st.debug, &text)
        };
        let mut at = stack(&mut d, Action::Line);
        for _ in 0..10 {
            if at == [5] {
                break;
            }
            at = stack(&mut d, Action::Line);
        }
        assert_eq!(at, vec![5], "about to call");
        assert_eq!(stack(&mut d, Action::Over), vec![6]);
        assert_eq!(stack(&mut d, Action::Back), vec![5]);
        assert_eq!(stack(&mut d, Action::Line), vec![5, 2]);
    }

    #[test]
    fn a_version_windows_breakpoint_hits_the_code_running_and_marks_each_files_frames() {
        let geo = "def one(n):\n    # one more\n    return n + 1\n";
        let nav = "def two(n):\n    return n + 2\n";
        let main = "import geo\nimport nav\nx = 0\nwhile True:\n    x = geo.one(x)\n    wait(1)\n";
        let mut d = driver();
        run_to(&mut d, 10);
        let bundle = sim::Bundle {
            files: vec![
                ("geo.py".into(), geo.into()),
                ("main.py".into(), main.into()),
                ("nav.py".into(), nav.into()),
            ],
        };
        d.submit(sim::CommandKind::Deploy {
            deployment: "printer".into(),
            bundle,
        })
        .unwrap();
        run_to(&mut d, 20);
        // The tree still holds the counter: the versions are the deploy's.
        let mut st = state();
        st.selected = Some(PRINTER);
        sync(&mut d, &mut st);
        let texts = st.debug.catalog.texts_of(
            d.drawn()
                .machines
                .iter()
                .find(|m| m.record.id == PRINTER)
                .unwrap(),
        );
        let geo_text = Text::of_bundle(
            Some("printer"),
            "geo.py",
            lang::content_hash(geo.as_bytes()),
        );
        let main_text = Text::of_bundle(
            Some("printer"),
            "main.py",
            lang::content_hash(main.as_bytes()),
        );
        let nav_text = Text::of_bundle(None, "nav.py", lang::content_hash(nav.as_bytes()));
        assert_eq!(
            texts,
            vec![geo_text.clone(), main_text.clone(), nav_text.clone()]
        );
        assert_eq!(geo_text.deployment, None);
        // A file window offers the versions of its own file, not of the
        // others the bundle carries.
        let of = |path: &str| {
            st.debug
                .catalog
                .versions_of(d.drawn(), sim::TeamId(0), path)
        };
        assert_eq!(of("lib/geo.py"), vec![geo_text.clone()]);
        assert_eq!(of("lib/deep/nav.py"), vec![nav_text]);
        assert_eq!(of("robots/printer.py"), vec![main_text.clone()]);
        assert_eq!(of("lib/other.py"), Vec::<Text>::new());
        let listing = st.debug.catalog.listing(&geo_text).expect("deployed");
        assert_eq!(listing.text, geo);

        // No statement begins on the comment: it settles on the next.
        click_version(&d, &mut st, &geo_text, 2);
        let set = st.debug.breakpoints.versions.get(&geo_text).cloned();
        assert_eq!(
            set,
            Some(lines(&[(3, Some(PRINTER))])),
            "the printer runs it"
        );
        // Its window is opened once, however often it is asked for.
        st.debug.open(geo_text.clone());
        st.debug.open(geo_text.clone());
        assert_eq!(st.debug.windows, std::slice::from_ref(&geo_text));
        assert!(!sync(&mut d, &mut st));
        assert!(st.layout.breakpoints.is_empty(), "not kept with the tree");
        d.advance(0.1);
        assert!(sync(&mut d, &mut st), "it hit");
        assert_eq!(frame_lines(&d, &st.debug, &geo_text), vec![3]);
        assert_eq!(frame_lines(&d, &st.debug, &main_text), vec![5]);
        assert_eq!(
            frame_lines(&d, &st.debug, &printer_text()),
            Vec::<u32>::new()
        );
        // A second click on the line clears it.
        click_version(&d, &mut st, &geo_text, 3);
        sync(&mut d, &mut st);
        assert!(st.debug.breakpoints.versions.is_empty());
        assert!(d.debug.watches().is_empty());
    }

    #[test]
    fn a_click_lands_on_the_line_whose_rows_it_falls_in() {
        // Three lines, the second wrapped onto two rows of 10.
        let tops = [0.0, 10.0, 30.0];
        assert_eq!(line_at(&tops, -1.0), None);
        assert_eq!(line_at(&tops, 0.0), Some(1));
        assert_eq!(line_at(&tops, 9.9), Some(1));
        assert_eq!(line_at(&tops, 10.0), Some(2));
        assert_eq!(line_at(&tops, 25.0), Some(2), "a wrapped line's second row");
        assert_eq!(line_at(&tops, 30.0), Some(3));
        assert_eq!(line_at(&tops, 500.0), Some(3));
        assert_eq!(line_at(&[], 5.0), None);
    }

    #[test]
    fn a_run_steps_along_the_ladder_without_its_pause() {
        let steps = [0, 1, 5, 10, 30];
        assert_eq!(step_run_speed(&steps, 10, true), Some(30));
        assert_eq!(step_run_speed(&steps, 10, false), Some(5));
        assert_eq!(step_run_speed(&steps, 30, true), None);
        assert_eq!(step_run_speed(&steps, 1, false), None, "never to the pause");
        assert_eq!(step_run_speed(&steps, 7, true), Some(10), "off the ladder");
        assert_eq!(step_run_speed(&steps, 7, false), Some(5));
        assert_eq!(step_run_speed(&steps, 0, true), Some(1));
    }
}
