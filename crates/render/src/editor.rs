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
pub use sim::script::robot_of;
use sim::script::{
    INTERRUPT_FILES, INTERRUPTS, ROBOTS, Tree, bare_name, compose, deployments_prose,
    is_stray_robot,
};
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
    /// Every file by its tree path (`robots/bot.py`, `lib/nav.py`).
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
    /// The rename field takes focus on the next frame: a menu just made
    /// an entry under a suggested name.
    pub focus_rename: bool,
    /// Rows a window's text box shows.
    pub rows: usize,
    /// Every file's outline, for what resting the pointer on a name shows
    /// (Q57).
    pub index: crate::hover::Index,
    /// The hover box showing, if any.
    pub hover_box: Option<HoverBox>,
    /// A definition to open, asked for from a hover box or by
    /// command-click (Q58); the windows take it on the next frame.
    pub goto: Option<(String, u32)>,
    /// A line a window is bringing into view.
    pub reveal: Option<Reveal>,
}

/// The hover box on screen (Q57). It stays while the pointer travels from
/// the name into it, so its link can be clicked (Q58).
#[derive(Debug, Clone)]
pub struct HoverBox {
    pub path: String,
    /// What the name under the pointer is, if it names something.
    pub hover: Option<crate::hover::Hover>,
    /// What the type check says of the code under the pointer (Q59).
    pub problems: Vec<String>,
    /// The rect the box belongs to on screen — the name's, or else the
    /// underlined code's — which it sits under, or over near the bottom
    /// of the screen.
    pub word: egui::Rect,
    /// Where the box was drawn last frame.
    pub rect: Option<egui::Rect>,
}

/// A line a file's window brings into view (Q58): scrolled to and given
/// the cursor on its first frame, then lit for [`FLASH_SECS`].
#[derive(Debug, Clone)]
pub struct Reveal {
    pub path: String,
    pub line: u32,
    /// When the window first showed it; `None` until then.
    pub since: Option<f64>,
}

/// How long a revealed line stays lit.
const FLASH_SECS: f64 = 1.2;

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

/// Whether the path is one of the fixed entries the player cannot rename,
/// move or delete: a deployment's robot file, or an interrupt file. A
/// robot file for no deployment is the player's to delete (Q42).
pub fn is_fixed(path: &str) -> bool {
    robot_of(path).is_some_and(sim::world::is_deployment_name)
        || interrupt_paths().iter().any(|p| p == path)
}

/// The builtins Q42 removed. A program that still calls one loads, then
/// raises `NameError` and faults every time the call runs.
const REMOVED_BUILTINS: [&str; 2] = ["print", "convert"];

/// The first call in `text` to a builtin Q42 removed, with its line,
/// unless the file binds that name itself: a `def`, an import of it, or
/// an assignment to it.
pub fn removed_call(text: &str) -> Option<(&'static str, u32)> {
    use lang::lexer::Tok;
    let toks = lang::lexer::Lexer::new("", text).tokenize().ok()?;
    let import_lines: Vec<u32> = toks
        .iter()
        .filter(|t| t.tok == Tok::Keyword("import"))
        .map(|t| t.line)
        .collect();
    let defined = |name: &str| {
        let named = Tok::Name(name.to_string());
        toks.windows(2).any(|w| {
            (w[0].tok == Tok::Keyword("def") && w[1].tok == named)
                || (w[0].tok == named && w[1].tok == Tok::Op("="))
        }) || toks
            .iter()
            .any(|t| t.tok == named && import_lines.contains(&t.line))
    };
    toks.windows(2).find_map(|w| match (&w[0].tok, &w[1].tok) {
        (Tok::Name(n), Tok::Op("(")) => REMOVED_BUILTINS
            .iter()
            .find(|r| **r == n.as_str() && !defined(r))
            .map(|r| (*r, w[0].line)),
        _ => None,
    })
}

/// What `removed_call` found, as the window and the tree say it.
pub fn removed_warning(text: &str) -> Option<String> {
    removed_call(text).map(|(name, line)| {
        format!(
            "line {line} calls `{name}`, which Q42 removed — printers print by themselves and \
             nothing converts — so it raises NameError and faults every time it runs"
        )
    })
}

/// What is wrong with a robot file for no deployment (Q42), if `path` is
/// one: it is never deployed, and the match starts without it.
pub fn stray_error(path: &str) -> Option<String> {
    is_stray_robot(path).then(|| {
        format!(
            "`{path}` is for no deployment — a team's are {} (Q42); \
             it is never deployed: move what you want into `{ROBOTS}/bot.py` and delete it",
            deployments_prose()
        )
    })
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
        self.index.update(
            self.files
                .iter()
                .map(|(p, d)| (p.as_str(), d.text.as_str())),
        );
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
            Some(sel)
                if sel.is_empty()
                    || sel == ROBOTS
                    || sel == INTERRUPTS
                    || is_fixed(sel)
                    || robot_of(sel).is_some() =>
            {
                String::new()
            }
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

    /// Rename or move a folder: every file and folder under it moves too.
    pub fn rename_folder(&mut self, from: &str, to: &str) -> Result<(), String> {
        let to = to.trim().trim_matches('/').to_string();
        if !self.folders.contains(from) {
            return Err(format!("`{from}` is not a folder"));
        }
        if to.is_empty() || to == ROBOTS || to == INTERRUPTS || self.folders.contains(&to) {
            return Err(format!("`{to}` is taken"));
        }
        if to.starts_with(&format!("{from}/")) {
            return Err("a folder cannot move into itself".into());
        }
        let prefix = format!("{from}/");
        let moved: Vec<(String, Doc)> = self
            .files
            .iter()
            .filter(|(p, _)| p.starts_with(&prefix))
            .map(|(p, d)| (format!("{to}/{}", &p[prefix.len()..]), d.clone()))
            .collect();
        self.files.retain(|p, _| !p.starts_with(&prefix));
        self.files.extend(moved);
        let folders: Vec<String> = self.folders.iter().cloned().collect();
        self.folders.clear();
        for f in folders {
            if f == from {
                self.folders.insert(to.clone());
            } else if let Some(rest) = f.strip_prefix(&prefix) {
                self.folders.insert(format!("{to}/{rest}"));
            } else {
                self.folders.insert(f);
            }
        }
        self.add_folder(&to)?;
        self.recompose();
        Ok(())
    }

    /// Rename a file or a folder to a leaf name in the same parent.
    pub fn rename_leaf(&mut self, path: &str, leaf: &str) -> Result<String, String> {
        let parent = path
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        let to = Editor::join(&parent, leaf.trim());
        if self.folders.contains(path) {
            self.rename_folder(path, &to)?;
        } else {
            self.rename(path, &to)?;
        }
        Ok(to)
    }

    /// A new folder under `base` with a free name; its path.
    pub fn create_folder_in(&mut self, base: &str) -> Result<String, String> {
        let path = Editor::join(base, &self.suggest_folder(base));
        self.add_folder(&path)?;
        Ok(path)
    }

    /// A new file under `base` with a free name; its path.
    pub fn create_file_in(&mut self, base: &str) -> Result<String, String> {
        let path = Editor::join(base, &self.suggest_file());
        self.add_file(&path)?;
        Ok(path)
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
    squiggles: &[std::ops::Range<usize>],
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
    for s in squiggles {
        for b in [s.start, s.end] {
            if b > range.start && b < range.end {
                cuts.push(b);
            }
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    for w in cuts.windows(2) {
        let underlined = squiggles.iter().any(|s| w[0] < s.end && w[1] > s.start);
        job.append(&text[w[0]..w[1]], 0.0, fmt(underlined));
    }
}

/// Best-effort highlighting: never fails, so a half-typed program is still
/// colored. Keywords are the lexer's own table, so the two cannot drift.
pub fn highlight(
    text: &str,
    font_id: egui::FontId,
    squiggles: &[std::ops::Range<usize>],
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
                squiggles,
            );
        }
        append_span(&mut job, text, start..end, color, &font_id, squiggles);
        plain_start = end;
    }
    if plain_start < text.len() {
        append_span(
            &mut job,
            text,
            plain_start..text.len(),
            HL_PLAIN,
            &font_id,
            squiggles,
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
    if state.editor.error_at(path).is_some()
        || state.editor.compose_error.is_some()
        || is_stray_robot(path)
        || state
            .editor
            .files
            .get(path)
            .is_some_and(|d| removed_call(&d.text).is_some())
    {
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

/// The char under a point in a laid-out text, or `None` if the point is
/// past the end of a line or between rows: the cursor egui puts nearest
/// the point, checked against the glyph on either side of it.
fn char_under(galley: &egui::Galley, local: egui::Vec2) -> Option<usize> {
    let c = galley.cursor_from_pos(local).index.0;
    [c.checked_sub(1), Some(c)]
        .into_iter()
        .flatten()
        .find(|&i| {
            let a = galley.pos_from_cursor(egui::text::CCursor::new(i));
            let b = galley.pos_from_cursor(egui::text::CCursor::new(i.saturating_add(1)));
            a.min.y == b.min.y
                && local.x >= a.min.x
                && local.x < b.min.x
                && local.y >= a.min.y
                && local.y <= a.max.y
        })
}

/// A hover's box: what the type check says of the code (Q59); then the
/// signature highlighted as code, the docstring, and where the name is
/// defined — for a file, a link that goes there (Q58), returned when
/// clicked.
fn hover_ui(
    ui: &mut egui::Ui,
    problems: &[String],
    h: Option<&crate::hover::Hover>,
    font: egui::FontId,
) -> Option<(String, u32)> {
    ui.set_max_width(560.0);
    // What the type check says comes first (Q59).
    for p in problems {
        ui.label(egui::RichText::new(p).color(HL_ERROR));
    }
    let h = h?;
    if !problems.is_empty() {
        ui.separator();
    }
    let mut job = highlight(&h.signature, font, &[]);
    job.wrap.max_width = 560.0;
    ui.label(job);
    if let Some(doc) = &h.doc {
        ui.separator();
        ui.label(doc);
    }
    ui.separator();
    match &h.origin {
        crate::hover::Origin::File { path, line } => {
            let link = egui::RichText::new(format!("go to {}", h.origin.label())).small();
            let key = if cfg!(target_os = "macos") {
                "⌘"
            } else {
                "Ctrl"
            };
            ui.link(link)
                .on_hover_text(format!("or {key}-click the name"))
                .clicked()
                .then(|| (path.clone(), *line))
        }
        o => {
            ui.label(egui::RichText::new(o.label()).small().weak());
            None
        }
    }
}

/// The 1-based line and column of char `at`.
pub fn line_col(text: &str, at: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for c in text.chars().take(at) {
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Each line's number in the gutter ending at `right`, level with the
/// line's first row; a line wrapped onto more rows numbers only the first.
/// The cursor's line is drawn brighter.
fn line_numbers(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    right: f32,
    font: egui::FontId,
    current: Option<usize>,
) {
    let clip = ui.clip_rect();
    let weak = ui.visuals().weak_text_color();
    let strong = ui.visuals().strong_text_color();
    let mut line = 1;
    let mut starts = true;
    for row in &output.galley.rows {
        if starts {
            let r = row.rect().translate(output.galley_pos.to_vec2());
            if r.bottom() >= clip.top() && r.top() <= clip.bottom() {
                let color = if current == Some(line) { strong } else { weak };
                ui.painter().text(
                    egui::pos2(right, r.top()),
                    egui::Align2::RIGHT_TOP,
                    line.to_string(),
                    font.clone(),
                    color,
                );
            }
            line += 1;
        }
        starts = row.ends_with_newline;
    }
}

/// The screen rect of 1-based `line`'s first row, across the text box.
fn line_rect(
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    across: egui::Rect,
    text: &str,
    line: u32,
) -> egui::Rect {
    let start = line_range(text, line).start;
    let at = text.get(..start).map_or(0, |t| t.chars().count());
    let row = galley.pos_from_cursor(egui::text::CCursor::new(at));
    egui::Rect::from_x_y_ranges(
        across.x_range(),
        (galley_pos.y + row.min.y)..=(galley_pos.y + row.max.y),
    )
}

/// The screen rect of chars `start..end`, on the row `start` is on: to
/// `end`, or to the row's end if the span wraps or runs on.
fn span_rect(
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    start: usize,
    end: usize,
) -> egui::Rect {
    let a = galley.pos_from_cursor(egui::text::CCursor::new(start));
    let b = galley.pos_from_cursor(egui::text::CCursor::new(end));
    let right = if b.min.y == a.min.y {
        b.min.x
    } else {
        galley.rect.right()
    };
    egui::Rect::from_min_max(a.min, egui::pos2(right.max(a.min.x + 1.0), a.max.y))
        .translate(galley_pos.to_vec2())
}

/// The screen rect of the identifier under char `at`.
fn word_rect(
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    text: &str,
    at: usize,
) -> Option<egui::Rect> {
    let w = crate::hover::word_at(text, at)?;
    let a = galley.pos_from_cursor(egui::text::CCursor::new(w.start));
    let b = galley.pos_from_cursor(egui::text::CCursor::new(w.end));
    Some(
        egui::Rect::from_min_max(a.min, egui::pos2(b.min.x, a.max.y))
            .translate(galley_pos.to_vec2()),
    )
}

/// Whether the hover box shows, after the pointer moved over `path`'s
/// text (Q57, Q58). A name under a still pointer opens it after egui's
/// tooltip delay, and with a box already open another name takes it at
/// once. The box stays while the pointer is over it or between it and
/// its name, so its link can be reached. A click on the text closes it,
/// and a command-click on a name defined in a file goes there.
fn hover_box(
    ui: &egui::Ui,
    state: &mut ViewState,
    path: &str,
    response: &egui::Response,
    under: Option<(Option<crate::hover::Hover>, Vec<String>, egui::Rect)>,
) {
    let delay = ui.style().interaction.tooltip_delay;
    let (still, clicked, command, pointer) = ui.input(|i| {
        let still = i.pointer.time_since_last_movement();
        (
            still,
            i.pointer.time_since_last_click() < still + 0.1,
            i.modifiers.command,
            i.pointer.hover_pos(),
        )
    });
    let editor = &mut state.editor;
    let here = editor.hover_box.as_ref().filter(|b| b.path == path);
    match under {
        Some((h, _, _)) if response.clicked() && command => {
            if let Some(crate::hover::Origin::File { path, line }) = h.map(|h| h.origin) {
                editor.goto = Some((path, line));
            }
            editor.hover_box = None;
        }
        Some(_) if clicked => editor.hover_box = None,
        Some((hover, problems, word)) => {
            if here.is_some_and(|b| b.word == word && b.hover == hover && b.problems == problems) {
                return;
            }
            if editor.hover_box.is_some() || still >= delay {
                editor.hover_box = Some(HoverBox {
                    path: path.to_string(),
                    hover,
                    problems,
                    word,
                    rect: None,
                });
            } else {
                ui.ctx().request_repaint_after_secs(delay - still);
            }
        }
        None => {
            let keep = here.is_some_and(|b| {
                pointer.is_some_and(|p| {
                    b.rect
                        .is_some_and(|r| r.expand(4.0).contains(p) || r.union(b.word).contains(p))
                })
            });
            if here.is_some() && !keep {
                editor.hover_box = None;
            }
        }
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
    // The type check's underlines (Q59), in chars of the text they were
    // found in; the frame a keystroke lands they may be a char off.
    let typed: Vec<(usize, usize)> = state
        .editor
        .index
        .diags(path)
        .iter()
        .map(|d| (d.start, d.end))
        .collect();
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let text = buf.as_str();
        let mut squiggles: Vec<std::ops::Range<usize>> = squiggle_line
            .map(|l| line_range(text, l))
            .into_iter()
            .collect();
        if !typed.is_empty() {
            let bytes: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
            let byte = |c: usize| bytes.get(c).copied().unwrap_or(text.len());
            squiggles.extend(typed.iter().map(|&(a, b)| byte(a)..byte(b)));
        }
        let mut job = highlight(text, font_id.clone(), &squiggles);
        job.wrap.max_width = wrap_width;
        let galley: Arc<egui::Galley> = ui.fonts_mut(|f| f.layout_job(job));
        galley
    };
    let mut changed = false;
    let hover_font = egui::TextStyle::Monospace.resolve(ui.style());
    let now = ui.input(|i| i.time);
    let mut cursor_at = None;
    let reveal = state
        .editor
        .reveal
        .as_ref()
        .filter(|r| r.path == path)
        .map(|r| (r.line, r.since));
    if let Some(doc) = state.editor.files.get_mut(path) {
        // The gutter fits the last line's number, and never fewer than two
        // digits, so it does not widen at line 10.
        let lines = doc.text.split('\n').count();
        let digits = lines.to_string().len().max(2);
        let digit_w = ui.fonts_mut(|f| f.glyph_width(&hover_font, '0'));
        let gutter_w = digit_w * digits as f32 + 4.0;
        let gutter_font = hover_font.clone();
        let mut output = egui::ScrollArea::vertical()
            .max_height((ui.available_height() - 150.0).max(80.0))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let gutter = ui.allocate_space(egui::vec2(gutter_w, 0.0)).1;
                    let output = egui::TextEdit::multiline(&mut doc.text)
                        .code_editor()
                        .desired_rows(rows)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .show(ui);
                    let current = output
                        .state
                        .cursor
                        .char_range()
                        .map(|r| line_col(&doc.text, r.primary.index.0).0);
                    line_numbers(ui, &output, gutter.right(), gutter_font, current);
                    // A definition gone to (Q58): scrolled into view once, then
                    // lit, fading.
                    if let Some((line, since)) = reveal {
                        let across = output.response.response.rect;
                        let rect =
                            line_rect(&output.galley, output.galley_pos, across, &doc.text, line);
                        if since.is_none() {
                            ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        }
                        let left = 1.0 - since.map_or(0.0, |s| (now - s) / FLASH_SECS);
                        if left > 0.0 {
                            let alpha = (left.min(1.0) * 70.0) as u8;
                            let lit = egui::Color32::from_rgba_unmultiplied(255, 214, 90, alpha);
                            ui.painter().rect_filled(rect, 2.0, lit);
                            ui.ctx().request_repaint();
                        }
                    }
                    output
                })
                .inner
            })
            .inner;
        // Where the cursor is, for the window's foot.
        cursor_at = output.state.cursor.char_range().map(|r| {
            let (line, col) = line_col(&doc.text, r.primary.index.0);
            (line, col, r.primary.index.0.abs_diff(r.secondary.index.0))
        });
        let response = output.response.response.clone();
        changed = response.changed();
        if let Some((line, since)) = reveal {
            if since.is_none() {
                // The cursor goes to the line, so typing starts there.
                let start = line_range(&doc.text, line).start;
                let at = doc.text.get(..start).map_or(0, |t| t.chars().count());
                let cursor = egui::text::CCursorRange::one(egui::text::CCursor::new(at));
                output.state.cursor.set_char_range(Some(cursor));
                output.state.clone().store(ui.ctx(), response.id);
                response.request_focus();
            }
            state.editor.reveal = match since {
                Some(s) if now - s >= FLASH_SECS => None,
                _ => Some(Reveal {
                    path: path.to_string(),
                    line,
                    since: since.or(Some(now)),
                }),
            };
        }
        // Resting on a name shows its signature and docstring (Q57), and
        // on underlined code what the type check says (Q59).
        let under = response.hover_pos().and_then(|pos| {
            let at = char_under(&output.galley, pos - output.galley_pos)?;
            let h = state.editor.index.hover(path, &doc.text, at);
            let hit: Vec<&crate::check::Diag> = state
                .editor
                .index
                .diags(path)
                .iter()
                .filter(|d| d.start <= at && at < d.end)
                .collect();
            let rect = match (&h, hit.first()) {
                (Some(_), _) => word_rect(&output.galley, output.galley_pos, &doc.text, at)?,
                (None, Some(d)) => span_rect(&output.galley, output.galley_pos, d.start, d.end),
                (None, None) => return None,
            };
            let problems = hit.iter().map(|d| d.message.clone()).collect();
            Some((h, problems, rect))
        });
        hover_box(ui, state, path, &response, under);
    }
    // The box itself, over everything, while it is this file's.
    if let Some(b) = state.editor.hover_box.clone().filter(|b| b.path == path) {
        let screen = ui.ctx().viewport_rect();
        let height = b.rect.map_or(160.0, |r| r.height());
        let (pos, pivot) = if b.word.bottom() + 2.0 + height <= screen.bottom() {
            (
                b.word.left_bottom() + egui::vec2(0.0, 2.0),
                egui::Align2::LEFT_TOP,
            )
        } else {
            (
                b.word.left_top() - egui::vec2(0.0, 2.0),
                egui::Align2::LEFT_BOTTOM,
            )
        };
        let shown = egui::Area::new(egui::Id::new(("hover", path)))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos)
            .pivot(pivot)
            .interactable(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        hover_ui(ui, &b.problems, b.hover.as_ref(), hover_font)
                    })
                    .inner
            });
        if let Some(target) = shown.inner {
            state.editor.goto = Some(target);
            state.editor.hover_box = None;
        } else if let Some(open) = state.editor.hover_box.as_mut() {
            open.rect = Some(shown.response.rect);
        }
    }
    if changed {
        state.editor.recompose();
    }
    if let Some(e) = &compose_error {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    } else if let Some(e) = stray_error(path) {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    } else if let Some((dep, e)) = &error {
        ui.colored_label(egui::Color32::LIGHT_RED, format!("{dep}: {e}"));
    }
    if let Some(w) = state
        .editor
        .files
        .get(path)
        .and_then(|d| removed_warning(&d.text))
    {
        ui.colored_label(egui::Color32::LIGHT_RED, w);
    }
    // What the file reaches, and each deployment's state.
    let reaches = state.editor.reaches(path);
    if reaches.is_empty() {
        ui.small(if is_stray_robot(path) {
            "for no deployment: never composed"
        } else if robot_of(path).is_some() {
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
        if let Some((line, col, selected)) = cursor_at {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let picked = if selected > 0 {
                    format!(" ({selected} selected)")
                } else {
                    String::new()
                };
                ui.small(format!("Ln {line}, Col {col}{picked}"));
            });
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

/// What a context menu asked for, applied after the tree is drawn.
enum Menu {
    NewFile(String),
    NewFolder(String),
    Rename(String),
    Delete(String),
}

/// The tree (`docs/07`, Q41): `robots`, `interrupts`, then the player's
/// folders and files; a click opens; new file, new folder, rename and
/// delete on the player's entries.
/// How many `/` a path directly inside the folder `prefix` holds: `prefix`
/// is `""` for the root or a folder's path with a trailing `/`, so `lib/`
/// holds `lib/geo.py` and `lib/sub`, one `/` each.
fn level_depth(prefix: &str) -> usize {
    prefix.matches('/').count()
}

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
        let removed = state
            .editor
            .files
            .get(path)
            .is_some_and(|d| removed_call(&d.text).is_some());
        if state.editor.error_at(path).is_some() || is_stray_robot(path) || removed {
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
    // Files clicked, gathered by the row closure and merged at the end;
    // what the context menus asked for, applied after the tree is drawn.
    let mut opened: Vec<String> = Vec::new();
    let mut menus: Vec<Menu> = Vec::new();
    let mut file_row =
        |ui: &mut egui::Ui, state: &mut ViewState, path: &str, menus: &mut Vec<Menu>| {
            let label = format!("{}{}", bare_name(path), mark(state, path));
            let selected = state.editor.selected.as_deref() == Some(path);
            let r = ui.selectable_label(selected, label);
            if r.clicked() {
                state.editor.selected = Some(path.to_string());
                opened.push(path.to_string());
            }
            let fixed = is_fixed(path);
            r.context_menu(|ui| {
                if ui.button("open").clicked() {
                    opened.push(path.to_string());
                    ui.close();
                }
                if !fixed {
                    // A stray robot file can only go: `robots/` takes no
                    // new name (Q42).
                    if !is_stray_robot(path) && ui.button("rename").clicked() {
                        menus.push(Menu::Rename(path.to_string()));
                        ui.close();
                    }
                    if ui.button("delete").clicked() {
                        menus.push(Menu::Delete(path.to_string()));
                        ui.close();
                    }
                }
            });
        };
    ui.heading("Programs");
    if let Some(e) = &state.editor.compose_error {
        ui.colored_label(egui::Color32::LIGHT_RED, e);
    }
    // The player's folders, nested by path, with the files at each level.
    fn level(
        ui: &mut egui::Ui,
        state: &mut ViewState,
        prefix: &str,
        files: &[String],
        folders: &[String],
        file_row: &mut dyn FnMut(&mut egui::Ui, &mut ViewState, &str, &mut Vec<Menu>),
        menus: &mut Vec<Menu>,
    ) {
        let depth = level_depth(prefix);
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
                level(ui, state, &format!("{f}/"), files, folders, file_row, menus);
            });
            if r.header_response.clicked() {
                state.editor.selected = Some(f.clone());
            }
            r.header_response.context_menu(|ui| {
                if ui.button("new file here").clicked() {
                    menus.push(Menu::NewFile(f.clone()));
                    ui.close();
                }
                if ui.button("new folder here").clicked() {
                    menus.push(Menu::NewFolder(f.clone()));
                    ui.close();
                }
                if ui.button("rename").clicked() {
                    menus.push(Menu::Rename(f.clone()));
                    ui.close();
                }
                if ui.button("delete").clicked() {
                    menus.push(Menu::Delete(f.clone()));
                    ui.close();
                }
            });
        }
        for p in files
            .iter()
            .filter(|p| p.starts_with(prefix) && p.matches('/').count() == depth)
        {
            file_row(ui, state, p, menus);
        }
    }
    let mut player_files: Vec<String> = files.iter().filter(|p| !is_fixed(p)).cloned().collect();
    player_files.sort();
    let folders: Vec<String> = state.editor.folders.iter().cloned().collect();
    // The root, `/`: a folder like any other — collapsed, selected, or
    // right-clicked to add to it — holding the two fixed folders and then
    // the player's.
    let root_selected = state.editor.selected.as_deref() == Some("");
    let root = egui::CollapsingHeader::new(if root_selected { "/ ◂" } else { "/" })
        .id_salt("root")
        .default_open(true)
        .show(ui, |ui| {
            egui::CollapsingHeader::new(ROBOTS)
                .default_open(true)
                .show(ui, |ui| {
                    let mut robots: Vec<String> = files
                        .iter()
                        .filter(|p| robot_of(p).is_some())
                        .cloned()
                        .collect();
                    robots = deployment_order_paths(robots);
                    // One file per deployment, and no others (Q42).
                    for p in &robots {
                        file_row(ui, state, p, &mut menus);
                    }
                });
            // The handlers are fixed: a lock says nothing can be added here.
            egui::CollapsingHeader::new(format!("{INTERRUPTS} \u{1F512}"))
                .id_salt(INTERRUPTS)
                .default_open(true)
                .show(ui, |ui| {
                    for p in interrupt_paths() {
                        file_row(ui, state, &p, &mut menus);
                    }
                })
                .header_response
                .on_hover_text("the two handler files are fixed; nothing can be added here");
            level(
                ui,
                state,
                "",
                &player_files,
                &folders,
                &mut file_row,
                &mut menus,
            );
        });
    if root.header_response.clicked() {
        state.editor.selected = Some(String::new());
    }
    root.header_response.context_menu(|ui| {
        if ui.button("new file").clicked() {
            menus.push(Menu::NewFile(String::new()));
            ui.close();
        }
        if ui.button("new folder").clicked() {
            menus.push(Menu::NewFolder(String::new()));
            ui.close();
        }
    });
    actions.extend(opened.into_iter().map(TreeAction::Open));

    // What the menus asked for: a new entry is made under a free name and
    // left in rename mode, so the real name is one Enter away.
    for m in menus {
        match m {
            Menu::NewFile(base) => match state.editor.create_file_in(&base) {
                Ok(p) => {
                    state.editor.rename_to = bare_name(&p).to_string();
                    state.editor.selected = Some(p.clone());
                    state.editor.focus_rename = true;
                    actions.push(TreeAction::Open(p));
                }
                Err(e) => state.status = e,
            },
            Menu::NewFolder(base) => match state.editor.create_folder_in(&base) {
                Ok(p) => {
                    state.editor.rename_to = bare_name(&p).to_string();
                    state.editor.selected = Some(p);
                    state.editor.focus_rename = true;
                }
                Err(e) => state.status = e,
            },
            Menu::Rename(p) => {
                state.editor.rename_to = bare_name(&p).to_string();
                state.editor.selected = Some(p);
                state.editor.focus_rename = true;
            }
            Menu::Delete(p) => match state.editor.delete(&p) {
                Ok(()) => state.editor.selected = None,
                Err(e) => state.status = e,
            },
        }
    }

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
        && !sel.is_empty()
        && !is_fixed(&sel)
        && sel != ROBOTS
        && sel != INTERRUPTS
    {
        ui.horizontal(|ui| {
            let id = egui::Id::new("tree-rename");
            let r = ui.add(
                egui::TextEdit::singleline(&mut state.editor.rename_to)
                    .id(id)
                    .hint_text(format!("rename {}", bare_name(&sel)))
                    .desired_width(140.0),
            );
            if state.editor.focus_rename {
                state.editor.focus_rename = false;
                ui.memory_mut(|m| m.request_focus(id));
            }
            if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.small_button("rename").clicked()
            {
                let leaf = state.editor.rename_to.clone();
                if !leaf.trim().is_empty() && leaf.trim() != bare_name(&sel) {
                    match state.editor.rename_leaf(&sel, &leaf) {
                        Ok(to) => {
                            state.editor.rename_to.clear();
                            state.editor.selected = Some(to);
                        }
                        Err(e) => state.status = e,
                    }
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

/// Robot files in deployment order: bot, printer, depot (Q42), then
/// anything else a programs directory held.
fn deployment_order_paths(mut paths: Vec<String>) -> Vec<String> {
    let rank = |p: &str| -> usize {
        match robot_of(p) {
            Some(d) => sim::world::DEPLOYMENTS
                .iter()
                .position(|x| *x == d)
                .unwrap_or(sim::world::DEPLOYMENTS.len()),
            None => sim::world::DEPLOYMENTS.len() + 1,
        }
    };
    paths.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_lists_the_paths_directly_inside_it() {
        let inside = |prefix: &str, path: &str| {
            path.starts_with(prefix) && path.matches('/').count() == level_depth(prefix)
        };
        assert!(inside("", "nav.py"));
        assert!(inside("", "lib"));
        assert!(!inside("", "lib/geo.py"));
        assert!(inside("lib/", "lib/geo.py"));
        assert!(inside("lib/", "lib/sub"));
        assert!(!inside("lib/", "lib/sub/deep.py"));
        assert!(inside("lib/sub/", "lib/sub/deep.py"));
    }

    fn spans(text: &str) -> Vec<(String, egui::Color32)> {
        highlight(text, egui::FontId::monospace(12.0), &[])
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
    fn line_and_column_count_from_one() {
        let text = "ab\nc\n\ndé";
        assert_eq!(line_col(text, 0), (1, 1));
        assert_eq!(line_col(text, 2), (1, 3)); // before the newline
        assert_eq!(line_col(text, 3), (2, 1));
        assert_eq!(line_col(text, 6), (4, 1)); // after the empty line
        assert_eq!(line_col(text, 8), (4, 3)); // chars, not bytes
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
        let job = highlight(
            text,
            egui::FontId::monospace(12.0),
            std::slice::from_ref(&(8..15)),
        );
        let mut pos = 0;
        for s in &job.sections {
            assert_eq!(s.byte_range.start.0, pos, "gap or overlap at {pos}");
            pos = s.byte_range.end.0;
            let overlaps = s.byte_range.start.0 < 15 && s.byte_range.end.0 > 8;
            assert_eq!(s.format.underline != egui::Stroke::NONE, overlaps);
        }
        assert_eq!(pos, text.len());
    }

    /// Q42: a robot file for no deployment — `robots/1.py` from a tree
    /// written before it — composes nothing, shows as an error, and is the
    /// player's to delete; the deployments' own files stay fixed.
    #[test]
    fn a_robot_file_for_no_deployment_is_an_error_the_player_may_delete() {
        let mut t = Tree::new();
        t.insert("robots/1.py".into(), "move('e')\n".into());
        t.insert("robots/bot.py".into(), "wait(1)\n".into());
        let mut e = Editor::from_tree(&t);
        assert_eq!(e.bundles.keys().cloned().collect::<Vec<_>>(), ["bot"]);
        assert!(e.compose_error.is_none());
        assert!(stray_error("robots/1.py").is_some());
        assert!(stray_error("robots/bot.py").is_none());
        assert!(is_fixed("robots/bot.py") && !is_fixed("robots/1.py"));
        assert!(e.delete("robots/bot.py").is_err());
        // Selected, it puts new files at the root, not in `robots/`.
        e.selected = Some("robots/1.py".into());
        assert_eq!(e.selected_folder(), "");
        e.delete("robots/1.py").unwrap();
        assert!(!e.files.contains_key("robots/1.py"));
    }

    /// A program still calling a builtin Q42 removed is flagged at its
    /// line; one that defines its own of that name is not.
    #[test]
    fn a_call_to_a_removed_builtin_is_flagged() {
        let old =
            "while True:\n    try:\n        print(1)\n    except ValueError:\n        wait(10)\n";
        assert_eq!(removed_call(old), Some(("print", 3)));
        assert!(removed_warning(old).is_some_and(|w| w.contains("line 3")));
        assert_eq!(removed_call("x = 'print(1)'  # print(2)\n"), None);
        assert_eq!(removed_call("def print(x):\n    log(x)\nprint(1)\n"), None);
        assert_eq!(removed_call("from util import print\nprint(1)\n"), None);
        assert_eq!(removed_call("print = log\nprint(1)\n"), None);
        assert_eq!(removed_call("convert(7)\n"), Some(("convert", 1)));
    }

    #[test]
    fn a_tree_composes_reaches_and_reports_errors_at_the_file() {
        let mut t = Tree::new();
        t.insert("robots/bot.py".into(), "import nav\nx = 1\nif x\n".into());
        t.insert("robots/printer.py".into(), "wait(60)\n".into());
        t.insert("lib/nav.py".into(), "y = 2\n".into());
        let mut e = Editor::from_tree(&t);
        assert!(e.files.contains_key("interrupts/on_fault.py"));
        assert_eq!(e.reaches("robots/bot.py"), ["bot"]);
        assert_eq!(e.reaches("lib/nav.py"), ["bot", "printer"]);
        let (dep, err) = e
            .error_at("robots/bot.py")
            .expect("a load error at the robot file");
        assert_eq!(dep, "bot");
        assert!(err.line >= 1);
        assert!(!line_range(&e.files["robots/bot.py"].text, err.line).is_empty());
        e.files.get_mut("robots/bot.py").unwrap().text = "import nav\nx = 1\n".into();
        e.recompose();
        assert!(e.error_at("robots/bot.py").is_none());
        assert!(matches!(e.loaded.get("bot"), Some(Ok(_))));
        assert!(e.add_file("robots/extra.py").is_err());
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
            e.error_at("robots/bot.py").is_some(),
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
        e.selected = Some("robots/bot.py".into());
        assert_eq!(e.selected_folder(), "");
        assert_eq!(e.suggest_file(), "module.py");
        e.add_file("module.py").unwrap();
        assert_eq!(e.suggest_file(), "module_2.py");
        e.add_folder("a/b/c").unwrap();
        assert!(e.folders.contains("a") && e.folders.contains("a/b"));
        // Menus: a new entry under a free name, then a leaf rename that
        // moves a folder's contents with it.
        let f = e.create_folder_in("a").unwrap();
        assert_eq!(f, "a/folder");
        let file = e.create_file_in(&f).unwrap();
        assert_eq!(file, "a/folder/module_2.py");
        let moved = e.rename_leaf("a/folder", "tools").unwrap();
        assert_eq!(moved, "a/tools");
        assert!(e.files.contains_key("a/tools/module_2.py") && !e.folders.contains("a/folder"));
        assert!(e.rename_leaf("robots/bot.py", "x.py").is_err());
        assert_eq!(line_range("ab\ncd", 7), 4..5);
    }
}
