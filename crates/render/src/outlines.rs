//! Team colors (`docs/07`, Machines on the map; Q43, Q45): a bot's lights
//! glow in its team's color and a building is outlined in it — the
//! player's own green by default, every other team's the next of the list
//! in `data/interface.toml` — and the player may change any of them, the
//! choice saved in `colors.toml` beside the programs. Nothing here reaches
//! the sim or a peer.

use crate::palette::Interface;
use bevy::prelude::*;
use sim::TeamId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's name beside the programs.
pub const FILE: &str = "colors.toml";

/// `colors.toml`: the player's choices, by team number, as `#rrggbb`.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Saved {
    #[serde(default)]
    teams: BTreeMap<String, String>,
}

/// A color as `#rrggbb`.
pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// `#rrggbb` as a color, or `None` if it is not one.
pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

pub fn to_color(rgb: [u8; 3]) -> Color {
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
}

/// The materials a team's machines share, recolored in place.
#[derive(Clone)]
pub struct TeamMats {
    /// A building's outline (Q43).
    pub outline: Handle<StandardMaterial>,
    /// A bot's 3D lens glass (Q45).
    pub glass: Handle<StandardMaterial>,
    /// A lit segment of a bot's number (Q46).
    pub segment: Handle<StandardMaterial>,
}

/// The player's outline choices for this match, and the materials every
/// team's outline and lens glass share, recolored in place when a choice
/// changes.
#[derive(Resource, Default)]
pub struct TeamColors {
    chosen: BTreeMap<u32, [u8; 3]>,
    path: Option<PathBuf>,
    /// Each team's shared materials, made on first use.
    pub mats: BTreeMap<u32, TeamMats>,
    /// Each bot body by (team, paint atlas), its lights glowing in the
    /// team's color (Q45), made on first use.
    pub bodies: BTreeMap<(u32, String), Handle<StandardMaterial>>,
    /// A choice changed since the materials were last recolored.
    pub dirty: bool,
}

impl TeamColors {
    pub fn path(programs: Option<&Path>) -> Option<PathBuf> {
        programs.map(|p| p.join(FILE))
    }

    /// The saved choices at `path`; none when there is no file, and an
    /// entry that is not a team number and a color is skipped.
    pub fn load(path: Option<PathBuf>) -> TeamColors {
        let saved: Saved = path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();
        let chosen = saved
            .teams
            .iter()
            .filter_map(|(k, v)| Some((k.parse().ok()?, parse_hex(v)?)))
            .collect();
        TeamColors {
            chosen,
            path,
            ..TeamColors::default()
        }
    }

    /// The team's outline color: the player's choice, or the default.
    pub fn rgb(&self, interface: &Interface, player: TeamId, team: TeamId) -> [u8; 3] {
        self.chosen
            .get(&team.0)
            .copied()
            .unwrap_or_else(|| interface.default_outline(player, team))
    }

    pub fn color(&self, interface: &Interface, player: TeamId, team: TeamId) -> Color {
        to_color(self.rgb(interface, player, team))
    }

    pub fn color32(
        &self,
        interface: &Interface,
        player: TeamId,
        team: TeamId,
    ) -> bevy_egui::egui::Color32 {
        let [r, g, b] = self.rgb(interface, player, team);
        bevy_egui::egui::Color32::from_rgb(r, g, b)
    }

    /// Choose `rgb` for `team` and save every choice beside the programs.
    pub fn set(&mut self, team: TeamId, rgb: [u8; 3]) -> Result<(), String> {
        self.chosen.insert(team.0, rgb);
        self.dirty = true;
        let Some(path) = &self.path else {
            return Ok(());
        };
        let saved = Saved {
            teams: self
                .chosen
                .iter()
                .map(|(t, c)| (t.to_string(), hex(*c)))
                .collect(),
        };
        let text = toml::to_string(&saved).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interface() -> Interface {
        Interface::load().unwrap()
    }

    #[test]
    fn hex_round_trips_and_rejects_what_is_not_a_color() {
        assert_eq!(parse_hex("#4fa3f2"), Some([0x4f, 0xa3, 0xf2]));
        assert_eq!(hex([0x4f, 0xa3, 0xf2]), "#4fa3f2");
        for bad in ["4fa3f2", "#4fa3f", "#4fa3f2a", "#zzzzzz", "#4fa3é"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_player_defaults_to_green_and_the_rest_follow_the_list() {
        let i = interface();
        let c = TeamColors::default();
        let list: Vec<[u8; 3]> = i
            .teams
            .outlines
            .iter()
            .map(|s| parse_hex(s).unwrap())
            .collect();
        let player = parse_hex(&i.teams.player).unwrap();
        assert_eq!(c.rgb(&i, TeamId(0), TeamId(0)), player);
        assert_eq!(c.rgb(&i, TeamId(0), TeamId(1)), list[0], "blue first");
        assert_eq!(c.rgb(&i, TeamId(0), TeamId(2)), list[1]);
        // As team 1, team 0 takes the first of the list, skipping the player.
        assert_eq!(c.rgb(&i, TeamId(1), TeamId(1)), player);
        assert_eq!(c.rgb(&i, TeamId(1), TeamId(0)), list[0]);
        assert_eq!(c.rgb(&i, TeamId(1), TeamId(2)), list[1]);
    }

    #[test]
    fn a_choice_wins_is_saved_and_is_restored() {
        let i = interface();
        let dir = std::env::temp_dir().join(format!("pg2-colors-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = TeamColors::path(Some(&dir));
        let _ = std::fs::remove_file(path.as_ref().unwrap());
        let mut c = TeamColors::load(path.clone());
        c.set(TeamId(1), [1, 2, 3]).unwrap();
        assert!(c.dirty);
        assert_eq!(c.rgb(&i, TeamId(0), TeamId(1)), [1, 2, 3]);
        let again = TeamColors::load(path.clone());
        assert_eq!(again.rgb(&i, TeamId(0), TeamId(1)), [1, 2, 3]);
        assert_eq!(
            again.rgb(&i, TeamId(0), TeamId(0)),
            parse_hex(&i.teams.player).unwrap(),
            "an unchosen team keeps its default"
        );
        // A file that does not parse, or an entry that is not a color, is
        // the defaults.
        std::fs::write(path.as_ref().unwrap(), "teams = 3").unwrap();
        assert!(TeamColors::load(path.clone()).chosen.is_empty());
        std::fs::write(
            path.as_ref().unwrap(),
            "[teams]\n1 = \"red\"\nx = \"#010203\"\n",
        )
        .unwrap();
        assert!(TeamColors::load(path).chosen.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
