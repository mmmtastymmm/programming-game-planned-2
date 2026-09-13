//! The windows' layout (`docs/07`, Q39): each window's position, size and
//! whether it is open, remembered in `windows.toml` beside the programs
//! whenever it changes and restored at start. A missing or malformed file
//! is the default layout. Nothing here reaches a peer.

use bevy_egui::egui;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub open: bool,
}

impl Placement {
    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(self.x, self.y), egui::vec2(self.w, self.h))
    }

    /// Whether `rect` moves or resizes this placement by more than a pixel.
    pub fn differs_from(&self, rect: egui::Rect) -> bool {
        (self.x - rect.min.x).abs() > 1.0
            || (self.y - rect.min.y).abs() > 1.0
            || (self.w - rect.width()).abs() > 1.0
            || (self.h - rect.height()).abs() > 1.0
    }

    pub fn set_rect(&mut self, rect: egui::Rect) {
        self.x = rect.min.x;
        self.y = rect.min.y;
        self.w = rect.width();
        self.h = rect.height();
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    #[serde(default)]
    pub windows: BTreeMap<String, Placement>,
}

/// The file's name beside the programs.
pub const FILE: &str = "windows.toml";

impl Layout {
    pub fn path(programs: Option<&Path>) -> Option<PathBuf> {
        programs.map(|p| p.join(FILE))
    }

    /// The saved layout, or the default when there is none or it does not
    /// parse.
    pub fn load(path: Option<&Path>) -> Layout {
        let Some(path) = path else {
            return Layout::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The window's placement, seeded with `default` on first sight.
    pub fn placement(&mut self, name: &str, default: impl FnOnce() -> Placement) -> &mut Placement {
        self.windows.entry(name.to_string()).or_insert_with(default)
    }
}

/// Where a window goes when the layout does not say: the deployments
/// cascade from the top left, the tools and the inspector stack at the
/// right, the log lies along the bottom. `open` is for a deployment; the
/// rest start open.
pub fn default_placement(name: &str, index: usize, screen: egui::Rect, open: bool) -> Placement {
    let (w, h, open) = match name {
        "inspector" => (320.0, 520.0, true),
        "tools" => (320.0, 230.0, true),
        "log" => (900.0, 200.0, true),
        _ => (520.0, 620.0, open),
    };
    let (x, y) = match name {
        "tools" => (screen.max.x - w - 10.0, screen.min.y + 10.0),
        "inspector" => (screen.max.x - w - 10.0, screen.min.y + 250.0),
        "log" => (screen.min.x + 10.0, screen.max.y - h - 10.0),
        _ => (
            screen.min.x + 10.0 + 30.0 * index as f32,
            screen.min.y + 10.0 + 30.0 * index as f32,
        ),
    };
    Placement { x, y, w, h, open }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_round_trips_and_a_bad_file_is_the_default() {
        let dir = std::env::temp_dir().join(format!("windows-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        let mut l = Layout::default();
        l.placement("red", || Placement {
            x: 1.0,
            y: 2.0,
            w: 300.0,
            h: 400.0,
            open: true,
        });
        l.save(&path).unwrap();
        let back = Layout::load(Some(&path));
        assert_eq!(back.windows["red"], l.windows["red"]);
        std::fs::write(&path, "not = [toml").unwrap();
        assert!(Layout::load(Some(&path)).windows.is_empty());
        assert!(
            Layout::load(Some(&dir.join("absent.toml")))
                .windows
                .is_empty()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_placement_notices_a_move_of_more_than_a_pixel() {
        let p = Placement {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 50.0,
            open: true,
        };
        assert!(!p.differs_from(p.rect().translate(egui::vec2(0.5, 0.5))));
        assert!(p.differs_from(p.rect().translate(egui::vec2(3.0, 0.0))));
    }
}
