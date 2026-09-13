//! The editor (`docs/07`, Q33 as Q41 amended): the team's program tree —
//! `robots/` with a file per deployment, `interrupts/` with the two hook
//! files, and the player's own files and folders — held as text, composed
//! into one bundle per deployment, and deployed as one `Deploy` per
//! deployment a file reaches. The programs directory is where the tree was
//! read from and where `export` writes to; the game never watches it.
//! Highlighting and the load-error squiggle are the build's choices;
//! nothing here reaches a peer except through the command log.

use crate::app::{DriverResource, ViewState};
use crate::driver::Driver;
use crate::input::submit;
use crate::view;
use bevy_egui::egui;
use lang::errors::LoadError;
use lang::{Limits, Program};
use sim::script::{INTERRUPT_FILES, INTERRUPTS, ROBOTS, Tree, bare_name, compose};
use sim::snapshot::Snapshot;
use sim::{Bundle, CommandKind, TeamId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// One file of the tree, as text.
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub text: String,
}

/// The tree and what it composes to.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    /// Every file by its tree path (`robots/1.py`, `lib/nav.py`).
    pub files: BTreeMap<String, Doc>,
    /// The player's folders, including empty ones.
    pub folders: BTreeSet<String>,
    /// What the tree composes to, per deployment.
    pub bundles: BTreeMap<String, Bundle>,
    /// Each composed bundle loaded the sim's way: its version, or the
    /// error (`01`) at a bundle file name.
    pub loaded: BTreeMap<String, Result<u64, LoadError>>,
    /// A tree that does not compose at all (a name shared by two files).
    pub compose_error: Option<String>,
    /// The tree panel's selection, for rename and delete.
    pub selected: Option<String>,
    /// Text being typed for a new file, a new folder, or a rename.
    pub new_file: String,
    pub new_folder: String,
    pub rename_to: String,
    /// Rows a window's text box shows.
    pub rows: usize,
}

/// The fixed files a tree always holds (Q41), besides the robots.
fn interrupt_paths() -> Vec<String> {
    INTERRUPT_FILES
        .iter()
        .map(|f| format!("{INTERRUPTS}/{f}"))
        .collect()
}

pub fn robot_path(deployment: &str) -> String {
    format!("{ROBOTS}/{deployment}.py")
}

/// The deployment a robot file is, if `path` is one.
pub fn robot_of(path: &str) -> Option<&str> {
    path.strip_prefix(&format!("{ROBOTS}/"))
        .and_then(|f| f.strip_suffix(".py"))
        .filter(|d| !d.contains('/'))
}

/// Whether the path is one of the fixed entries the player cannot rename,
/// move or delete.
pub fn is_fixed(path: &str) -> bool {
    robot_of(path).is_some() || interrupt_paths().iter().any(|p| p == path)
}

impl Editor {
    /// The tree read from the programs directory (Q41).
    pub fn from_tree(tree: &Tree) -> Editor {
        let mut e = Editor {
            rows: 28,
            ..Default::default()
        };
        for (path, text) in tree {
            e.files.insert(path.clone(), Doc { text: text.clone() });
            if let Some((dir, _)) = path.rsplit_once('/') {
                e.folders.insert(dir.to_string());
            }
        }
        for p in interrupt_paths() {
            e.files.entry(p).or_default();
        }
        e.folders.retain(|f| f != ROBOTS && f != INTERRUPTS);
        e.recompose();
        e
    }

    /// A robot file for every deployment the team holds, and the
    /// interrupt files, before the tree is drawn.
    pub fn ensure(&mut self, deployments: &BTreeMap<String, Option<u64>>) {
        let mut added = false;
        for path in deployments
            .keys()
            .map(|n| robot_path(n))
            .chain(interrupt_paths())
        {
            if let std::collections::btree_map::Entry::Vacant(v) = self.files.entry(path) {
                v.insert(Doc::default());
                added = true;
            }
        }
        if added {
            self.recompose();
        }
    }

    pub fn tree(&self) -> Tree {
        self.files
            .iter()
            .map(|(p, d)| (p.clone(), d.text.clone()))
            .collect()
    }

    /// Compose every bundle and load each the sim's way.
    pub fn recompose(&mut self) {
        let limits = Limits::parse(lang::data::LIMITS_TOML).expect("limits");
        match compose(&self.tree()) {
            Ok(bundles) => {
                self.compose_error = None;
                self.loaded = bundles
                    .iter()
                    .map(|(d, b)| {
                        let refs: Vec<(&str, &str)> = b
                            .files
                            .iter()
                            .map(|(n, s)| (n.as_str(), s.as_str()))
                            .collect();
                        (d.clone(), Program::load(&refs, &limits).map(|p| p.version))
                    })
                    .collect();
                self.bundles = bundles;
            }
            Err(e) => {
                self.compose_error = Some(e);
                self.bundles.clear();
                self.loaded.clear();
            }
        }
    }

    /// The deployments a file reaches: a robot file its own, any other
    /// every composed deployment.
    pub fn reaches(&self, path: &str) -> Vec<String> {
        match robot_of(path) {
            Some(d) => {
                if self.bundles.contains_key(d) {
                    vec![d.to_string()]
                } else {
                    Vec::new()
                }
            }
            None => self.bundles.keys().cloned().collect(),
        }
    }

    /// The tree path a bundle file name of `deployment` came from.
    pub fn path_of(&self, deployment: &str, bundle_file: &str) -> Option<String> {
        if bundle_file == "main.py" {
            return Some(robot_path(deployment));
        }
        self.files
            .keys()
            .find(|p| !p.starts_with(&format!("{ROBOTS}/")) && bare_name(p) == bundle_file)
            .cloned()
    }

    /// The first load error that names `path`, with the deployment it was
    /// found loading.
    pub fn error_at(&self, path: &str) -> Option<(String, LoadError)> {
        for (d, r) in &self.loaded {
            if let Err(e) = r
                && self.path_of(d, &e.file).as_deref() == Some(path)
            {
                return Some((d.clone(), e.clone()));
            }
        }
        None
    }

    /// Whether a deployment's composed bundle loads and differs from what
    /// the team runs; an empty robot file composes nothing and is never
    /// ahead.
    pub fn deployment_ahead(&self, deployment: &str, running: Option<u64>) -> bool {
        matches!(self.loaded.get(deployment), Some(Ok(v)) if running != Some(*v))
    }

    /// Whether any deployment the file reaches is ahead.
    pub fn ahead(&self, path: &str, snap: &Snapshot, player: TeamId) -> bool {
        let Some(team) = snap.teams.iter().find(|t| t.id == player) else {
            return false;
        };
        self.reaches(path)
            .iter()
            .any(|d| self.deployment_ahead(d, team.deployments.get(d).copied().flatten()))
    }

    /// Every deployment that is ahead.
    pub fn changed(&self, snap: &Snapshot, player: TeamId) -> Vec<String> {
        let Some(team) = snap.teams.iter().find(|t| t.id == player) else {
            return Vec::new();
        };
        self.bundles
            .keys()
            .filter(|d| self.deployment_ahead(d, team.deployments.get(*d).copied().flatten()))
            .cloned()
            .collect()
    }

    /// The number after the highest robot file: what *next* opens (Q40).
    pub fn next_deployment(&self) -> String {
        let max = self
            .files
            .keys()
            .filter_map(|p| robot_of(p))
            .filter_map(sim::world::deployment_number)
            .max()
            .unwrap_or(0);
        (max + 1).to_string()
    }

    /// Add a file at `path` if none exists; `Err` names why not.
    pub fn add_file(&mut self, path: &str) -> Result<(), String> {
        let path = path.trim().trim_matches('/').to_string();
        let bare = bare_name(&path);
        if !lang_name(bare) {
            return Err(format!("`{bare}` is not `[a-z_][a-z0-9_]*.py`"));
        }
        if path.starts_with(&format!("{ROBOTS}/")) || path.starts_with(&format!("{INTERRUPTS}/")) {
            return Err(format!(
                "`{ROBOTS}/` and `{INTERRUPTS}/` hold only their fixed files"
            ));
        }
        if self.files.contains_key(&path) {
            return Err(format!("`{path}` exists"));
        }
        if let Some((dir, _)) = path.rsplit_once('/') {
            self.folders.insert(dir.to_string());
        }
        self.files.insert(path, Doc::default());
        self.recompose();
        Ok(())
    }

    pub fn add_folder(&mut self, path: &str) -> Result<(), String> {
        let path = path.trim().trim_matches('/').to_string();
        if path.is_empty() || path == ROBOTS || path == INTERRUPTS {
            return Err("a folder needs a name of its own".into());
        }
        if !path.split('/').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }) {
            return Err("a folder name is letters, digits and `_`".into());
        }
        // Every folder on the way exists too.
        let mut so_far = String::new();
        for seg in path.split('/') {
            so_far = Editor::join(&so_far, seg);
            self.folders.insert(so_far.clone());
        }
        Ok(())
    }

    /// Rename or move a player file.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        if is_fixed(from) {
            return Err(format!("`{from}` is fixed"));
        }
        let Some(doc) = self.files.get(from).cloned() else {
            return Err(format!("`{from}` does not exist"));
        };
        self.add_file(to)?;
        self.files
            .insert(to.trim().trim_matches('/').to_string(), doc);
        self.files.remove(from);
        self.recompose();
        Ok(())
    }

    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        if is_fixed(path) {
            return Err(format!("`{path}` is fixed"));
        }
        if self.files.remove(path).is_none() {
            if self.folders.remove(path) {
                let under: Vec<String> = self
                    .files
                    .keys()
                    .filter(|p| p.starts_with(&format!("{path}/")))
                    .cloned()
                    .collect();
                if !under.is_empty() {
                    self.folders.insert(path.to_string());
                    return Err(format!("`{path}` is not empty"));
                }
                return Ok(());
            }
            return Err(format!("`{path}` does not exist"));
        }
        self.recompose();
        Ok(())
    }
}

impl Editor {
    /// The folder new entries go into: the selected folder, the selected
    /// file's folder, or the root (`""`).
    pub fn selected_folder(&self) -> String {
        match &self.selected {
            Some(sel) if self.folders.contains(sel) => sel.clone(),
            Some(sel) if sel == ROBOTS || sel == INTERRUPTS || is_fixed(sel) => String::new(),
            Some(sel) => sel
                .rsplit_once('/')
                .map(|(d, _)| d.to_string())
                .unwrap_or_default(),
            None => String::new(),
        }
    }

    /// `base/leaf`, or `leaf` at the root.
    pub fn join(base: &str, leaf: &str) -> String {
        if base.is_empty() {
            leaf.to_string()
        } else {
            format!("{base}/{leaf}")
        }
    }

    /// A folder name not yet taken under `base`: `folder`, `folder_2`, …
    pub fn suggest_folder(&self, base: &str) -> String {
        (1..)
            .map(|i| {
                if i == 1 {
                    "folder".to_string()
                } else {
                    format!("folder_{i}")
                }
            })
            .find(|n| {
                let p = Editor::join(base, n);
                !self.folders.contains(&p) && p != ROBOTS && p != INTERRUPTS
            })
            .unwrap_or_default()
    }

    /// A module name not yet taken anywhere in the tree: `module.py`,
    /// `module_2.py`, … — anywhere, since stems are unique across it.
    pub fn suggest_file(&self) -> String {
        (1..)
            .map(|i| {
                if i == 1 {
                    "module.py".to_string()
                } else {
                    format!("module_{i}.py")
                }
            })
            .find(|n| !self.files.keys().any(|p| bare_name(p) == n))
            .unwrap_or_default()
    }
}

/// A file name the language accepts (`01`).
fn lang_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".py") else {
        return false;
    };
    let mut chars = stem.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Deploy every deployment a file reaches that is ahead; `true` if any
/// was taken.
pub fn deploy_file(driver: &mut Driver, state: &mut ViewState, path: &str) -> bool {
    let player = driver.player;
    let snap = driver.snapshot().clone();
    let team = snap.teams.iter().find(|t| t.id == player);
    let mut any = false;
    for d in state.editor.reaches(path) {
        let running = team.and_then(|t| t.deployments.get(&d).copied().flatten());
        match state.editor.loaded.get(&d) {
            Some(Err(e)) => {
                state.status = format!("{d}: {e}");
            }
            Some(Ok(_)) if state.editor.deployment_ahead(&d, running) => {
                let bundle = state.editor.bundles[&d].clone();
                any |= submit(
                    driver,
                    state,
                    CommandKind::Deploy {
                        deployment: d.clone(),
                        bundle,
                    },
                );
            }
            _ => {}
        }
    }
    any
}

/// Deploy every deployment that is ahead (Shift+D).
pub fn deploy_changed(driver: &mut Driver, state: &mut ViewState) {
    let names = state.editor.changed(driver.snapshot(), driver.player);
    if names.is_empty() {
        state.status = "every deployment runs what the tree composes".into();
    }
    for d in names {
        let bundle = state.editor.bundles[&d].clone();
        submit(
            driver,
            state,
            CommandKind::Deploy {
                deployment: d.clone(),
                bundle,
            },
        );
    }
}

/// Write the tree to the programs directory, folders included; files the
/// tree no longer has are left alone.
pub fn export(state: &mut ViewState) {
    let Some(root) = state.programs.clone() else {
        state.status = "no programs directory (--programs DIR)".into();
        return;
    };
    let mut n = 0;
    for f in &state.editor.folders {
        if let Err(e) = std::fs::create_dir_all(root.join(f)) {
            state.status = format!("{}: {e}", root.join(f).display());
            return;
        }
    }
    for (path, doc) in &state.editor.files {
        let full = root.join(path);
        if let Some(dir) = full.parent()
            && let Err(e) = std::fs::create_dir_all(dir)
        {
            state.status = format!("{}: {e}", dir.display());
            return;
        }
        if let Err(e) = std::fs::write(&full, &doc.text) {
            state.status = format!("{}: {e}", full.display());
            return;
        }
        n += 1;
    }
    state.status = format!("exported {n} file(s) to {}", root.display());
}

// ── highlighting ────────────────────────────────────────────────────────────

const HL_KEYWORD: egui::Color32 = egui::Color32::from_rgb(197, 134, 192);
const HL_FUNCTION: egui::Color32 = egui::Color32::from_rgb(220, 220, 130);
const HL_VARIABLE: egui::Color32 = egui::Color32::from_rgb(156, 220, 254);
const HL_NUMBER: egui::Color32 = egui::Color32::from_rgb(181, 206, 168);
const HL_STRING: egui::Color32 = egui::Color32::from_rgb(206, 145, 120);
const HL_COMMENT: egui::Color32 = egui::Color32::from_rgb(106, 153, 85);
const HL_PLAIN: egui::Color32 = egui::Color32::from_rgb(212, 212, 212);
const HL_ERROR: egui::Color32 = egui::Color32::from_rgb(235, 80, 70);

fn append_span(
    job: &mut egui::text::LayoutJob,
    text: &str,
    range: std::ops::Range<usize>,
    color: egui::Color32,
    font_id: &egui::FontId,
    squiggle: &Option<std::ops::Range<usize>>,
) {
    let fmt = |underline: bool| egui::text::TextFormat {
        font_id: font_id.clone(),
        color,
        underline: if underline {
            egui::Stroke::new(1.5, HL_ERROR)
        } else {
            egui::Stroke::NONE
        },
        ..Default::default()
    };
    let mut cuts = vec![range.start, range.end];
    if let Some(s) = squiggle {
        for b in [s.start, s.end] {
            if b > range.start && b < range.end {
                cuts.push(b);
            }
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    for w in cuts.windows(2) {
        let underlined = squiggle
            .as_ref()
            .is_some_and(|s| w[0] < s.end && w[1] > s.start);
        job.append(&text[w[0]..w[1]], 0.0, fmt(underlined));
    }
}

/// Best-effort highlighting: never fails, so a half-typed program is still
/// colored. Keywords are the lexer's own table, so the two cannot drift.
pub fn highlight(
    text: &str,
    font_id: egui::FontId,
    squiggle: Option<std::ops::Range<usize>>,
) -> egui::text::LayoutJob {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let byte_at = |i: usize| chars.get(i).map_or(text.len(), |&(b, _)| b);
    let mut job = egui::text::LayoutJob::default();
    let n = chars.len();
    let mut plain_start = 0;
    let mut i = 0;
    while i < n {
        let (start, c) = chars[i];
        let (end, color) = if c == '#' {
            while i < n && chars[i].1 != '\n' {
                i += 1;
            }
            (byte_at(i), HL_COMMENT)
        } else if c == '"' || c == '\'' {
            let q = c;
            let triple = chars.get(i + 1).map(|&(_, c)| c) == Some(q)
                && chars.get(i + 2).map(|&(_, c)| c) == Some(q);
            if triple {
                i += 3;
                while i < n
                    && !(chars[i].1 == q
                        && chars.get(i + 1).map(|&(_, c)| c) == Some(q)
                        && chars.get(i + 2).map(|&(_, c)| c) == Some(q))
                {
                    i += 1;
                }
                if i < n {
                    i += 3;
                }
            } else {
                i += 1;
                while i < n && chars[i].1 != q && chars[i].1 != '\n' {
                    i += if chars[i].1 == '\\' { 2 } else { 1 };
                }
                if i < n && chars[i].1 == q {
                    i += 1;
                }
            }
            (byte_at(i.min(n)), HL_STRING)
        } else if c.is_ascii_digit() {
            while i < n
                && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '.' || chars[i].1 == '_')
            {
                i += 1;
            }
            (byte_at(i), HL_NUMBER)
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < n && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let end = byte_at(i);
            let word = &text[start..end];
            let color = if lang::lexer::KEYWORDS.contains(&word) {
                HL_KEYWORD
            } else {
                let mut j = i;
                while j < n && chars[j].1 == ' ' {
                    j += 1;
                }
                if j < n && chars[j].1 == '(' {
                    HL_FUNCTION
                } else {
                    HL_VARIABLE
                }
            };
            (end, color)
        } else {
            i += 1;
            continue;
        };
        if plain_start < start {
            append_span(
                &mut job,
                text,
                plain_start..start,
                HL_PLAIN,
                &font_id,
                &squiggle,
            );
        }
        append_span(&mut job, text, start..end, color, &font_id, &squiggle);
        plain_start = end;
    }
    if plain_start < text.len() {
        append_span(
            &mut job,
            text,
            plain_start..text.len(),
            HL_PLAIN,
            &font_id,
            &squiggle,
        );
    }
    job
}

/// The bytes of 1-based `line`, for the squiggle; the last visible
/// character when the line is past the end.
pub fn line_range(text: &str, line: u32) -> std::ops::Range<usize> {
    let idx = (line as usize).saturating_sub(1);
    let mut offset = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == idx {
            let trimmed = l.len() - l.trim_start().len();
            if !l.trim().is_empty() {
                return offset + trimmed..offset + l.len();
            }
            break;
        }
        offset += l.len() + 1;
    }
    text.char_indices()
        .rev()
        .find(|(_, c)| *c != '\n')
        .map_or(0..0, |(b, c)| b..b + c.len_utf8())
}

// ── the windows ─────────────────────────────────────────────────────────────

/// A file window's title: its bare name, `*` when a deployment it reaches
/// is ahead, `!` when it carries a load error.
pub fn title(state: &ViewState, path: &str, snap: &Snapshot, player: TeamId) -> String {
    let bare = bare_name(path);
    if state.editor.error_at(path).is_some() || state.editor.compose_error.is_some() {
        format!("{bare} !")
    } else if state.editor.ahead(path, snap, player) {
        format!("{bare} *")
    } else {
        bare.to_string()
    }
}

/// Every deployment the team holds has a robot file, before the tree is
/// drawn.
pub fn sync(driver: &DriverResource, state: &mut ViewState) {
    let snap = driver.0.snapshot();
    if let Some(t) = snap.teams.iter().find(|t| t.id == driver.0.player) {
        state.editor.ensure(&t.deployments);
    }
}

/// One file's window (`docs/07`, Q41): the text, what it reaches and the
/// state of each, the buttons, and the faults naming this file (Q34).
pub fn window_body(
    ui: &mut egui::Ui,
    driver: &mut DriverResource,
    state: &mut ViewState,
    path: &str,
) {
    let d = &mut driver.0;
    let snap = d.snapshot().clone();
    let player = d.player;
    let team = snap.teams.iter().find(|t| t.id == player);
    let rows = state.editor.rows;
    let error = state.editor.error_at(path);
    let compose_error = state.editor.compose_error.clone();
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let squiggle_line = error.as_ref().map(|(_, e)| e.line);
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let text = buf.as_str();
        let squiggle = squiggle_line.map(|l| line_range(text, l));
        let mut job = highlight(text, font_id.clone(), squiggle);
        job.wrap.max_width = wrap_width;
        let galley: Arc<egui::Galley> = ui.fonts_mut(|f| f.layout_job(job));
        galley
    };
    let mut changed = false;
    if let Some(doc) = state.editor.files.get_mut(path) {
        let response = egui::ScrollArea::vertical()
            .max_height((ui.available_height() - 150.0).max(80.0))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut doc.text)
                        .code_editor()
                        .desired_rows(rows)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                )
            })
            .inner;
        changed = response.changed();
    }
    if changed {
        state.editor.recompose();
    }
    if let Some(e) = &compose_error {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    } else if let Some((dep, e)) = &error {
        ui.colored_label(egui::Color32::LIGHT_RED, format!("{dep}: {e}"));
    }
    // What the file reaches, and each deployment's state.
    let reaches = state.editor.reaches(path);
    if reaches.is_empty() {
        ui.small(if robot_of(path).is_some() {
            "empty: composes nothing until it has text"
        } else {
            "reaches no deployment yet"
        });
    } else {
        let parts: Vec<String> = reaches
            .iter()
            .map(|dep| {
                let running = team.and_then(|t| t.deployments.get(dep).copied().flatten());
                match state.editor.loaded.get(dep) {
                    Some(Ok(v)) if running == Some(*v) => format!("{dep} ="),
                    Some(Ok(_)) => format!("{dep} *"),
                    _ => format!("{dep} !"),
                }
            })
            .collect();
        ui.small(format!("reaches {}", parts.join("  ")));
    }
    let mut deploy_now = false;
    let mut deploy_all = false;
    let mut export_now = false;
    ui.horizontal(|ui| {
        let ready = state.editor.ahead(path, &snap, player);
        if ui.add_enabled(ready, egui::Button::new("deploy")).clicked() {
            deploy_now = true;
        }
        if ui.button("deploy all changed (Shift+D)").clicked() {
            deploy_all = true;
        }
        if ui.button("export").clicked() {
            export_now = true;
        }
    });
    if deploy_now {
        deploy_file(d, state, path);
    }
    if deploy_all {
        deploy_changed(d, state);
    }
    if export_now {
        export(state);
    }

    // The fault summary (Q34): the latest records naming this file, by
    // line, across every deployment it reaches, newest first.
    ui.separator();
    let mut groups: BTreeMap<(u32, u64), (usize, u64)> = BTreeMap::new();
    for m in &snap.machines {
        if m.record.team != player {
            continue;
        }
        let Some(dep) = m.record.deployment.as_deref() else {
            continue;
        };
        if !reaches.iter().any(|r| r == dep) {
            continue;
        }
        if let Some(f) = &m.fault
            && let Some(file) = &f.file
            && state.editor.path_of(dep, file).as_deref() == Some(path)
        {
            let g = groups.entry((f.line, f.version)).or_insert((0, 0));
            g.0 += 1;
            g.1 = g.1.max(f.tick);
        }
    }
    let bare = bare_name(path);
    if groups.is_empty() {
        ui.small(format!("{bare}: no faults"));
    } else {
        ui.strong(format!("{bare}: faults"));
        let mut rows: Vec<_> = groups.into_iter().collect();
        rows.sort_by_key(|(_, (_, tick))| std::cmp::Reverse(*tick));
        let current: BTreeSet<u64> = reaches
            .iter()
            .filter_map(|dep| team.and_then(|t| t.deployments.get(dep).copied().flatten()))
            .collect();
        for ((line, version), (count, tick)) in rows {
            let old = if current.contains(&version) {
                ""
            } else {
                " (an earlier version)"
            };
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("{count} at line {line}, last tick {tick}{old}"),
            );
        }
    }
    let _ = view::fault_what;
}

// ── the tree panel ──────────────────────────────────────────────────────────

/// What the tree panel asks of the app: open a file's window.
pub enum TreeAction {
    Open(String),
}

/// The tree (`docs/07`, Q41): `robots`, `interrupts`, then the player's
/// folders and files; a click opens; new file, new folder, rename and
/// delete on the player's entries.
pub fn tree_panel(
    ui: &mut egui::Ui,
    state: &mut ViewState,
    snap: &Snapshot,
    player: TeamId,
    open: &BTreeSet<String>,
) -> Vec<TreeAction> {
    let mut actions = Vec::new();
    let mark = |state: &ViewState, path: &str| -> String {
        let mut s = String::new();
        if state.editor.error_at(path).is_some() {
            s.push_str(" !");
        } else if state.editor.ahead(path, snap, player) {
            s.push_str(" *");
        }
        if open.contains(path) {
            s.push_str(" •");
        }
        s
    };
    let files: Vec<String> = state.editor.files.keys().cloned().collect();
    // Files clicked, gathered by the row closure and merged at the end.
    let mut opened: Vec<String> = Vec::new();
    let mut file_row = |ui: &mut egui::Ui, state: &mut ViewState, path: &str| {
        let label = format!("{}{}", bare_name(path), mark(state, path));
        let selected = state.editor.selected.as_deref() == Some(path);
        let r = ui.selectable_label(selected, label);
        if r.clicked() {
            state.editor.selected = Some(path.to_string());
            opened.push(path.to_string());
        }
    };
    if ui
        .add(
            egui::Label::new(egui::RichText::new("Programs").heading()).sense(egui::Sense::click()),
        )
        .on_hover_text("click to make the root the place for new files and folders")
        .clicked()
    {
        state.editor.selected = None;
    }
    if let Some(e) = &state.editor.compose_error {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    }
    egui::CollapsingHeader::new(ROBOTS)
        .default_open(true)
        .show(ui, |ui| {
            let mut robots: Vec<String> = files
                .iter()
                .filter(|p| robot_of(p).is_some())
                .cloned()
                .collect();
            robots = deployment_order_paths(robots);
            for p in &robots {
                file_row(ui, state, p);
            }
            let next = state.editor.next_deployment();
            if ui
                .small_button("+")
                .on_hover_text(format!("a file for deployment {next}"))
                .clicked()
            {
                let p = robot_path(&next);
                state.editor.files.entry(p.clone()).or_default();
                state.editor.recompose();
                actions.push(TreeAction::Open(p));
            }
        });
    egui::CollapsingHeader::new(INTERRUPTS)
        .default_open(true)
        .show(ui, |ui| {
            for p in interrupt_paths() {
                file_row(ui, state, &p);
            }
        });
    // The player's tree: folders nested by path, files at each level.
    let mut player_files: Vec<String> = files.iter().filter(|p| !is_fixed(p)).cloned().collect();
    player_files.sort();
    let folders: Vec<String> = state.editor.folders.iter().cloned().collect();
    fn level(
        ui: &mut egui::Ui,
        state: &mut ViewState,
        prefix: &str,
        files: &[String],
        folders: &[String],
        file_row: &mut dyn FnMut(&mut egui::Ui, &mut ViewState, &str),
    ) {
        let depth = if prefix.is_empty() {
            0
        } else {
            prefix.matches('/').count() + 1
        };
        let mut subfolders: Vec<&String> = folders
            .iter()
            .filter(|f| {
                f.starts_with(prefix) && f.matches('/').count() == depth && f.as_str() != prefix
            })
            .collect();
        subfolders.sort();
        for f in subfolders {
            let name = f.rsplit('/').next().unwrap_or(f).to_string();
            let selected = state.editor.selected.as_deref() == Some(f.as_str());
            let header = egui::CollapsingHeader::new(if selected {
                format!("{name} ◂")
            } else {
                name
            })
            .id_salt(f)
            .default_open(true);
            let r = header.show(ui, |ui| {
                level(ui, state, &format!("{f}/"), files, folders, file_row);
            });
            if r.header_response.clicked() {
                state.editor.selected = Some(f.clone());
            }
        }
        for p in files
            .iter()
            .filter(|p| p.starts_with(prefix) && p.matches('/').count() == depth)
        {
            file_row(ui, state, p);
        }
    }
    level(ui, state, "", &player_files, &folders, &mut file_row);
    actions.extend(opened.into_iter().map(TreeAction::Open));

    ui.separator();
    // New file and new folder go into the selected folder (or the selected
    // file's), each with a free name filled in; rename and delete act on
    // the player's selection.
    let base = state.editor.selected_folder();
    if state.editor.new_file.is_empty() {
        state.editor.new_file = state.editor.suggest_file();
    }
    if state.editor.new_folder.is_empty() {
        state.editor.new_folder = state.editor.suggest_folder(&base);
    }
    let prefix = if base.is_empty() {
        String::new()
    } else {
        format!("{base}/")
    };
    ui.horizontal(|ui| {
        if !prefix.is_empty() {
            ui.small(&prefix);
        }
        let r = ui.add(egui::TextEdit::singleline(&mut state.editor.new_file).desired_width(120.0));
        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || ui.small_button("new file").clicked()
        {
            let leaf = state.editor.new_file.trim().to_string();
            if !leaf.is_empty() {
                let path = Editor::join(&base, &leaf);
                match state.editor.add_file(&path) {
                    Ok(()) => {
                        state.editor.new_file.clear();
                        state.editor.selected = Some(path.clone());
                        actions.push(TreeAction::Open(path));
                    }
                    Err(e) => state.status = e,
                }
            }
        }
    });
    ui.horizontal(|ui| {
        if !prefix.is_empty() {
            ui.small(&prefix);
        }
        let r =
            ui.add(egui::TextEdit::singleline(&mut state.editor.new_folder).desired_width(120.0));
        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || ui.small_button("new folder").clicked()
        {
            let leaf = state.editor.new_folder.trim().to_string();
            if !leaf.is_empty() {
                let path = Editor::join(&base, &leaf);
                match state.editor.add_folder(&path) {
                    Ok(()) => {
                        state.editor.new_folder.clear();
                        state.editor.selected = Some(path);
                    }
                    Err(e) => state.status = e,
                }
            }
        }
    });
    if let Some(sel) = state.editor.selected.clone()
        && !is_fixed(&sel)
    {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.editor.rename_to)
                    .hint_text(format!("rename {}", bare_name(&sel)))
                    .desired_width(140.0),
            );
            if ui.small_button("rename").clicked() {
                let to = state.editor.rename_to.clone();
                match state.editor.rename(&sel, &to) {
                    Ok(()) => {
                        state.editor.rename_to.clear();
                        state.editor.selected = Some(to.trim().trim_matches('/').to_string());
                    }
                    Err(e) => state.status = e,
                }
            }
            if ui.small_button("delete").clicked() {
                match state.editor.delete(&sel) {
                    Ok(()) => state.editor.selected = None,
                    Err(e) => state.status = e,
                }
            }
        });
    }
    actions
}

/// Robot files in deployment order: printer, depot, then the numbers.
fn deployment_order_paths(mut paths: Vec<String>) -> Vec<String> {
    let rank = |p: &str| -> (u64, u64) {
        match robot_of(p) {
            Some("printer") => (0, 0),
            Some("depot") => (1, 0),
            Some(d) => match sim::world::deployment_number(d) {
                Some(k) => (2, k),
                None => (3, 0),
            },
            None => (4, 0),
        }
    };
    paths.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<(String, egui::Color32)> {
        highlight(text, egui::FontId::monospace(12.0), None)
            .sections
            .iter()
            .map(|s| {
                (
                    text[s.byte_range.start.0..s.byte_range.end.0].to_string(),
                    s.format.color,
                )
            })
            .collect()
    }

    #[test]
    fn highlights_keywords_calls_names_and_literals() {
        let got = spans("if x > 1:\n    move_to(target, \"hi\") # go\n");
        for expected in [
            ("if", HL_KEYWORD),
            ("x", HL_VARIABLE),
            ("1", HL_NUMBER),
            ("move_to", HL_FUNCTION),
            ("target", HL_VARIABLE),
            ("\"hi\"", HL_STRING),
            ("# go", HL_COMMENT),
        ] {
            assert!(
                got.contains(&(expected.0.to_string(), expected.1)),
                "missing {expected:?} in {got:?}"
            );
        }
    }

    #[test]
    fn highlight_covers_every_byte_once_and_squiggles_the_range() {
        let text = "move_to(closest(ore).expect())\n# comment\nwhile True:\n    x = x + 1\n";
        let job = highlight(text, egui::FontId::monospace(12.0), Some(8..15));
        let mut pos = 0;
        for s in &job.sections {
            assert_eq!(s.byte_range.start.0, pos, "gap or overlap at {pos}");
            pos = s.byte_range.end.0;
            let overlaps = s.byte_range.start.0 < 15 && s.byte_range.end.0 > 8;
            assert_eq!(s.format.underline != egui::Stroke::NONE, overlaps);
        }
        assert_eq!(pos, text.len());
    }

    #[test]
    fn a_tree_composes_reaches_and_reports_errors_at_the_file() {
        let mut t = Tree::new();
        t.insert("robots/1.py".into(), "import nav\nx = 1\nif x\n".into());
        t.insert("robots/printer.py".into(), "print(1)\n".into());
        t.insert("lib/nav.py".into(), "y = 2\n".into());
        let mut e = Editor::from_tree(&t);
        assert!(e.files.contains_key("interrupts/on_fault.py"));
        assert_eq!(e.reaches("robots/1.py"), ["1"]);
        assert_eq!(e.reaches("lib/nav.py"), ["1", "printer"]);
        let (dep, err) = e
            .error_at("robots/1.py")
            .expect("a load error at the robot file");
        assert_eq!(dep, "1");
        assert!(err.line >= 1);
        assert!(!line_range(&e.files["robots/1.py"].text, err.line).is_empty());
        e.files.get_mut("robots/1.py").unwrap().text = "import nav\nx = 1\n".into();
        e.recompose();
        assert!(e.error_at("robots/1.py").is_none());
        assert!(matches!(e.loaded.get("1"), Some(Ok(_))));
        assert_eq!(e.next_deployment(), "2");
        assert!(e.add_file("robots/3.py").is_err());
        assert!(e.add_file("lib/Nav.py").is_err());
        e.add_file("util/nav.py").unwrap();
        assert!(
            e.compose_error
                .as_deref()
                .is_some_and(|m| m.contains("share the name"))
        );
        e.delete("util/nav.py").unwrap();
        assert!(e.compose_error.is_none());
        assert!(e.delete("interrupts/on_fault.py").is_err());
        e.rename("lib/nav.py", "lib/path.py").unwrap();
        assert!(
            e.error_at("robots/1.py").is_some(),
            "the import of nav now fails"
        );
        // Suggestions: a free name under the selected folder, the root when
        // a fixed entry or nothing is selected.
        e.selected = Some("lib".into());
        assert_eq!(e.selected_folder(), "lib");
        assert_eq!(e.suggest_folder("lib"), "folder");
        e.add_folder("lib/folder").unwrap();
        assert_eq!(e.suggest_folder("lib"), "folder_2");
        e.selected = Some("lib/path.py".into());
        assert_eq!(e.selected_folder(), "lib");
        e.selected = Some("robots/1.py".into());
        assert_eq!(e.selected_folder(), "");
        assert_eq!(e.suggest_file(), "module.py");
        e.add_file("module.py").unwrap();
        assert_eq!(e.suggest_file(), "module_2.py");
        e.add_folder("a/b/c").unwrap();
        assert!(e.folders.contains("a") && e.folders.contains("a/b"));
        assert_eq!(line_range("ab\ncd", 7), 4..5);
    }
}
