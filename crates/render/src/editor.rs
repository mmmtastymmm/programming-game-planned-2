//! The editor (`docs/07`, Q33): a working copy of every deployment's
//! bundle, edited in a panel beside the map and deployed from there as one
//! `Deploy` carrying the working copy's files. The programs directory is
//! where the opening bundles were read from and where `export` writes to;
//! the game never watches it. Highlighting and the load-error squiggle are
//! the build's choices; nothing here reaches a peer except through the
//! command log.

use crate::app::{DriverResource, ViewState};
use crate::driver::Driver;
use crate::input::submit;
use crate::view;
use bevy_egui::egui;
use lang::errors::LoadError;
use lang::{Limits, Program};
use sim::snapshot::Snapshot;
use sim::{Bundle, CommandKind, TeamId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One deployment's working copy.
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub files: Vec<(String, String)>,
    /// The file open in the panel.
    pub file: usize,
    /// The working copy's version, when it loads; the error when it does
    /// not (`01`), shown at its file and line.
    pub version: Option<u64>,
    pub error: Option<LoadError>,
    /// A file name being typed for "new file".
    pub new_name: String,
}

impl Doc {
    pub fn from_bundle(b: &Bundle) -> Doc {
        let mut d = Doc {
            files: b.files.clone(),
            ..Default::default()
        };
        d.check();
        d
    }

    pub fn bundle(&self) -> Bundle {
        let mut files = self.files.clone();
        files.sort();
        Bundle { files }
    }

    /// Load the working copy the way the sim will, for its version or its
    /// first error.
    pub fn check(&mut self) {
        let limits = Limits::parse(lang::data::LIMITS_TOML).expect("limits");
        let refs: Vec<(&str, &str)> = self
            .files
            .iter()
            .map(|(n, s)| (n.as_str(), s.as_str()))
            .collect();
        match Program::load(&refs, &limits) {
            Ok(p) => {
                self.version = Some(p.version);
                self.error = None;
            }
            Err(e) => {
                self.version = None;
                self.error = Some(e);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Editor {
    pub docs: BTreeMap<String, Doc>,
    pub active: Option<String>,
    /// Rows the text box shows.
    pub rows: usize,
}

impl Doc {
    /// A working copy with nothing in it: no file with any text.
    pub fn is_empty(&self) -> bool {
        self.files.iter().all(|(_, t)| t.trim().is_empty())
    }

    /// Whether the working copy is ahead of what the deployment runs: it
    /// loads, differs, and is not an untouched empty copy of nothing.
    pub fn ahead(&self, running: Option<u64>) -> bool {
        match self.version {
            Some(v) => running != Some(v) && (running.is_some() || !self.is_empty()),
            None => false,
        }
    }
}

/// The tabs' order: the printer, the depot, then the colors as
/// `data/machines.toml` lists them, then anything else by name.
pub fn deployment_order(names: impl Iterator<Item = String>) -> Vec<String> {
    let colors = sim::Data::load()
        .map(|d| d.machines.colors)
        .unwrap_or_default();
    let rank = |n: &str| -> usize {
        match n {
            "printer" => 0,
            "depot" => 1,
            _ => colors
                .iter()
                .position(|c| c == n)
                .map(|i| i + 2)
                .unwrap_or(usize::MAX),
        }
    };
    let mut v: Vec<String> = names.collect();
    v.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    v
}

/// The tab to open first: the first color deployment, which is what the
/// player edits most; failing that, the first tab.
fn first_tab(names: &[String]) -> Option<String> {
    names
        .iter()
        .find(|n| n.as_str() != "printer" && n.as_str() != "depot")
        .or(names.first())
        .cloned()
}

impl Editor {
    /// Working copies from the opening bundles, one per deployment
    /// directory (Q33).
    pub fn from_bundles(bundles: &BTreeMap<String, Bundle>) -> Editor {
        let docs: BTreeMap<String, Doc> = bundles
            .iter()
            .map(|(n, b)| (n.clone(), Doc::from_bundle(b)))
            .collect();
        Editor {
            active: first_tab(&deployment_order(docs.keys().cloned())),
            docs,
            rows: 28,
        }
    }

    /// A working copy for every deployment the team holds, empty for one
    /// no directory seeded.
    pub fn ensure(&mut self, deployments: &BTreeMap<String, Option<u64>>) {
        for name in deployments.keys() {
            self.docs.entry(name.clone()).or_insert_with(|| {
                let mut d = Doc {
                    files: vec![("main.py".into(), String::new())],
                    ..Default::default()
                };
                d.check();
                d
            });
        }
        if self.active.is_none() {
            self.active = first_tab(&deployment_order(self.docs.keys().cloned()));
        }
    }

    /// The deployments whose working copy loads and differs from what the
    /// team runs.
    pub fn changed(&self, snap: &Snapshot, player: TeamId) -> Vec<String> {
        let Some(team) = snap.teams.iter().find(|t| t.id == player) else {
            return Vec::new();
        };
        self.docs
            .iter()
            .filter(|(name, doc)| doc.ahead(team.deployments.get(*name).copied().flatten()))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Deploy one working copy; `true` if the log took it.
pub fn deploy_doc(driver: &mut Driver, state: &mut ViewState, name: &str) -> bool {
    let Some(doc) = state.editor.docs.get_mut(name) else {
        return false;
    };
    doc.check();
    if let Some(e) = &doc.error {
        state.status = format!("{name}: {e}");
        return false;
    }
    let bundle = doc.bundle();
    submit(
        driver,
        state,
        CommandKind::Deploy {
            deployment: name.to_string(),
            bundle,
        },
    )
}

/// Write every working copy to the programs directory, one directory per
/// deployment; files the copy no longer has are left alone.
pub fn export(state: &mut ViewState) {
    let Some(root) = state.programs.clone() else {
        state.status = "no programs directory (--programs DIR)".into();
        return;
    };
    let mut n = 0;
    for (name, doc) in &state.editor.docs {
        let dir = root.join(name);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            state.status = format!("{}: {e}", dir.display());
            return;
        }
        for (file, text) in &doc.files {
            if let Err(e) = std::fs::write(dir.join(file), text) {
                state.status = format!("{}: {e}", dir.join(file).display());
                return;
            }
            n += 1;
        }
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

// ── the panel ───────────────────────────────────────────────────────────────

/// The editor panel's contents: deployment tabs, file tabs, the text, the
/// buttons, and the fault summary (Q34).
pub fn panel(ui: &mut egui::Ui, driver: &mut DriverResource, state: &mut ViewState) {
    let d = &mut driver.0;
    let snap = d.snapshot().clone();
    let player = d.player;
    let team = snap.teams.iter().find(|t| t.id == player);
    if let Some(t) = team {
        state.editor.ensure(&t.deployments);
    }
    ui.heading("Programs");
    ui.horizontal_wrapped(|ui| {
        let names = deployment_order(state.editor.docs.keys().cloned());
        for name in names {
            let running = team.and_then(|t| t.deployments.get(&name).copied().flatten());
            let doc = &state.editor.docs[&name];
            let label = if doc.error.is_some() {
                format!("{name} !")
            } else if doc.ahead(running) {
                format!("{name} *")
            } else {
                name.clone()
            };
            let active = state.editor.active.as_deref() == Some(name.as_str());
            if ui.selectable_label(active, label).clicked() {
                state.editor.active = Some(name.clone());
            }
        }
    });
    let Some(name) = state.editor.active.clone() else {
        ui.small("no deployments");
        return;
    };
    let running = team.and_then(|t| t.deployments.get(&name).copied().flatten());
    let rows = state.editor.rows;
    let mut deploy_now = false;
    let mut export_now = false;
    let mut deploy_all = false;
    {
        let doc = state.editor.docs.get_mut(&name).expect("active doc");
        // File tabs and a new-file box.
        ui.horizontal_wrapped(|ui| {
            for i in 0..doc.files.len() {
                let f = doc.files[i].0.clone();
                if ui.selectable_label(doc.file == i, f).clicked() {
                    doc.file = i;
                }
            }
            let edit = ui.add(
                egui::TextEdit::singleline(&mut doc.new_name)
                    .hint_text("new.py")
                    .desired_width(70.0),
            );
            if edit.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !doc.new_name.is_empty()
            {
                let mut n = doc.new_name.trim().to_string();
                if !n.ends_with(".py") {
                    n.push_str(".py");
                }
                if !doc.files.iter().any(|(f, _)| *f == n) {
                    doc.files.push((n, String::new()));
                    doc.file = doc.files.len() - 1;
                    doc.check();
                }
                doc.new_name.clear();
            }
        });
        doc.file = doc.file.min(doc.files.len().saturating_sub(1));
        let error = doc.error.clone();
        let file_name = doc
            .files
            .get(doc.file)
            .map(|f| f.0.clone())
            .unwrap_or_default();
        let squiggle_line = error
            .as_ref()
            .filter(|e| e.file == file_name)
            .map(|e| e.line);
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
        let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let text = buf.as_str();
            let squiggle = squiggle_line.map(|l| line_range(text, l));
            let mut job = highlight(text, font_id.clone(), squiggle);
            job.wrap.max_width = wrap_width;
            let galley: Arc<egui::Galley> = ui.fonts_mut(|f| f.layout_job(job));
            galley
        };
        if let Some((_, text)) = doc.files.get_mut(doc.file) {
            let response = egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 180.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(text)
                            .code_editor()
                            .desired_rows(rows)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    )
                })
                .inner;
            if response.changed() {
                doc.check();
            }
        }
        match (&doc.error, doc.version, running) {
            (Some(e), _, _) => {
                ui.colored_label(egui::Color32::LIGHT_RED, format!("{e}"));
            }
            (None, Some(v), Some(r)) if v == r => {
                ui.small(format!("running {v:016x} — the working copy is what runs"));
            }
            (None, Some(v), Some(r)) => {
                ui.small(format!("running {r:016x} · working {v:016x} (ahead)"));
            }
            (None, Some(v), None) => {
                ui.small(format!("nothing running · working {v:016x}"));
            }
            (None, None, _) => {}
        }
        ui.horizontal(|ui| {
            let ready = doc.ahead(running);
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
    }
    if deploy_now {
        deploy_doc(d, state, &name);
    }
    if deploy_all {
        crate::input::deploy(d, state);
    }
    if export_now {
        export(state);
    }

    // The fault summary (Q34): this deployment's machines' latest records
    // by file and line, newest first, with the version each names.
    ui.separator();
    let mut groups: BTreeMap<(String, u32, u64), (usize, u64)> = BTreeMap::new();
    for m in &snap.machines {
        if m.record.team != player || m.record.deployment.as_deref() != Some(name.as_str()) {
            continue;
        }
        if let Some(f) = &m.fault {
            let key = (
                f.file.clone().unwrap_or_else(|| "?".into()),
                f.line,
                f.version,
            );
            let g = groups.entry(key).or_insert((0, 0));
            g.0 += 1;
            g.1 = g.1.max(f.tick);
        }
    }
    if groups.is_empty() {
        ui.small(format!("{name}: no faults"));
    } else {
        ui.strong(format!("{name}: faults"));
        let mut rows: Vec<_> = groups.into_iter().collect();
        rows.sort_by_key(|(_, (_, tick))| std::cmp::Reverse(*tick));
        for ((file, line, version), (count, tick)) in rows {
            let old = if Some(version) != running {
                " (an earlier version)"
            } else {
                ""
            };
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("{count} at {file}:{line}, last tick {tick}{old}"),
            );
        }
    }
    let _ = view::fault_what;
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
    fn a_load_error_names_a_line_the_editor_can_squiggle() {
        let mut doc = Doc {
            files: vec![("main.py".into(), "x = 1\nif x\n    y = 2\n".into())],
            ..Default::default()
        };
        doc.check();
        let e = doc.error.clone().expect("a load error");
        assert_eq!(e.file, "main.py");
        assert!(e.line >= 1);
        let r = line_range(&doc.files[0].1, e.line);
        assert!(!r.is_empty());
        doc.files[0].1 = "x = 1\nif x:\n    y = 2\n".into();
        doc.check();
        assert!(doc.error.is_none() && doc.version.is_some());
        assert_eq!(line_range("ab\ncd", 7), 4..5);
    }
}
