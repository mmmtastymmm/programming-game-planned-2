//! Scripted teams (`docs/04`): a command log written in advance, one
//! directory per card under `data/opposition/`, with tiles relative to the
//! team's first starting tile.

use crate::command::{Bundle, Command, CommandKind, PlanKind, TeamId};
use crate::map::TilePos;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    /// `(tick, kind)` with tiles still relative; `resolve` places them.
    pub entries: Vec<(u64, CommandKind)>,
}

impl Script {
    /// Read `<dir>/script.toml` and the bundles it names, each a directory
    /// of `.py` files under `<dir>`.
    pub fn load(dir: &Path) -> Result<Script, String> {
        let text = std::fs::read_to_string(dir.join("script.toml")).map_err(|e| e.to_string())?;
        let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let name = t
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("script.toml lacks `name`")?
            .to_string();
        let mut entries = Vec::new();
        let commands = t
            .get("command")
            .and_then(|v| v.as_array())
            .ok_or("script.toml lacks `[[command]]`")?;
        for c in commands {
            let tick = c
                .get("tick")
                .and_then(|v| v.as_integer())
                .and_then(|i| u64::try_from(i).ok())
                .ok_or("a command lacks `tick`")?;
            let kind = c
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or("a command lacks `kind`")?;
            let at = |key: &str| -> Result<TilePos, String> {
                let a = c
                    .get(key)
                    .and_then(|v| v.as_array())
                    .ok_or(format!("a command lacks `{key}`"))?;
                let x = a
                    .first()
                    .and_then(|v| v.as_integer())
                    .and_then(|i| i32::try_from(i).ok())
                    .ok_or("x")?;
                let y = a
                    .get(1)
                    .and_then(|v| v.as_integer())
                    .and_then(|i| i32::try_from(i).ok())
                    .ok_or("y")?;
                Ok(TilePos::new(x, y))
            };
            let kind = match kind {
                "Deploy" => {
                    let deployment = c
                        .get("deployment")
                        .and_then(|v| v.as_str())
                        .ok_or("a Deploy lacks `deployment`")?;
                    let bundle_name = c
                        .get("bundle")
                        .and_then(|v| v.as_str())
                        .ok_or("a Deploy lacks `bundle`")?;
                    CommandKind::Deploy {
                        deployment: deployment.to_string(),
                        bundle: read_bundle(&dir.join(bundle_name))?,
                    }
                }
                "Mark" => {
                    let plan = c
                        .get("plan")
                        .and_then(|v| v.as_str())
                        .and_then(PlanKind::from_name)
                        .ok_or("a Mark lacks a `plan` kind")?;
                    let value = c.get("value").and_then(|v| v.as_str()).map(str::to_string);
                    CommandKind::Mark {
                        at: at("at")?,
                        plan,
                        value,
                    }
                }
                "Unmark" => {
                    let plan = c
                        .get("plan")
                        .and_then(|v| v.as_str())
                        .and_then(PlanKind::from_name)
                        .ok_or("an Unmark lacks a `plan` kind")?;
                    CommandKind::Unmark {
                        at: at("at")?,
                        plan,
                    }
                }
                "SetSpeed" => {
                    let s = c
                        .get("speed")
                        .and_then(|v| v.as_integer())
                        .and_then(|i| u64::try_from(i).ok())
                        .ok_or("a SetSpeed lacks `speed`")?;
                    CommandKind::SetSpeed(s)
                }
                "Resign" => CommandKind::Resign,
                other => return Err(format!("unknown command kind `{other}`")),
            };
            entries.push((tick, kind));
        }
        Ok(Script { name, entries })
    }

    /// The script's commands for `team`, tiles resolved against `origin`,
    /// sequence numbers assigned in order.
    pub fn resolve(&self, team: TeamId, origin: TilePos) -> Vec<Command> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, (tick, kind))| {
                let kind = match kind {
                    CommandKind::Mark { at, plan, value } => CommandKind::Mark {
                        at: TilePos::new(
                            origin.x.saturating_add(at.x),
                            origin.y.saturating_add(at.y),
                        ),
                        plan: *plan,
                        value: value.clone(),
                    },
                    CommandKind::Unmark { at, plan } => CommandKind::Unmark {
                        at: TilePos::new(
                            origin.x.saturating_add(at.x),
                            origin.y.saturating_add(at.y),
                        ),
                        plan: *plan,
                    },
                    other => other.clone(),
                };
                Command::new(*tick, team, i as u32, kind)
            })
            .collect()
    }
}

/// Every `.py` file in a directory, sorted by name, as a bundle.
pub fn read_bundle(dir: &Path) -> Result<Bundle, String> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".py") {
            let text = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            files.push((name, text));
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!("{} holds no .py file", dir.display()));
    }
    Ok(Bundle { files })
}
