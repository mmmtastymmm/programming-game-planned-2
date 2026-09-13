//! Scripted teams (`docs/04`): a command log written in advance, one
//! directory per card under `data/opposition/`, with tiles relative to the
//! team's first starting tile.

use crate::command::{Bundle, Command, CommandKind, PlanKind, TeamId};
use crate::map::TilePos;
use std::collections::BTreeMap;
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
        let tree = if dir.join("robots").is_dir() {
            read_tree(dir)?
        } else {
            BTreeMap::new()
        };
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
                    // `bundle` names a deployment of the card's tree (Q41),
                    // or, for a fixture, a raw directory of `.py` files.
                    let bundle = match tree.get(bundle_name) {
                        Some(b) => b.clone(),
                        None => read_bundle(&dir.join(bundle_name))?,
                    };
                    CommandKind::Deploy {
                        deployment: deployment.to_string(),
                        bundle,
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

/// The fixed folders of a programs tree (`docs/07`, Q41).
pub const ROBOTS: &str = "robots";
pub const INTERRUPTS: &str = "interrupts";
/// The interrupt files every bundle carries.
pub const INTERRUPT_FILES: [&str; 2] = ["on_fault.py", "on_dying.py"];

/// A programs tree as text: every `.py` file by its path relative to the
/// root, with `/` separators.
pub type Tree = BTreeMap<String, String>;

/// Read every `.py` file under `root`, recursively, by relative path.
pub fn read_tree_files(root: &Path) -> Result<Tree, String> {
    fn walk(root: &Path, dir: &Path, out: &mut Tree) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut paths: Vec<_> = entries
            .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        paths.sort();
        for p in paths {
            if p.is_dir() {
                walk(root, &p, out)?;
            } else if p.extension().is_some_and(|x| x == "py") {
                let rel = p
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                let text =
                    std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                out.insert(rel, text);
            }
        }
        Ok(())
    }
    let mut out = Tree::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

/// The bare file name of a tree path.
pub fn bare_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Compose one bundle per deployment from a tree (`docs/07`, Q41):
/// `robots/<d>.py` is `main.py`, plus every interrupt file and every
/// player file by its bare name. A robot file that is empty makes no
/// bundle; two files with one stem are an error, since folders are
/// organisational only and the language has no packages.
pub fn compose(tree: &Tree) -> Result<BTreeMap<String, Bundle>, String> {
    let mut shared: Vec<(String, String)> = Vec::new();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for name in INTERRUPT_FILES {
        let path = format!("{INTERRUPTS}/{name}");
        let text = tree.get(&path).cloned().unwrap_or_default();
        seen.insert(name.to_string(), path);
        shared.push((name.to_string(), text));
    }
    for (path, text) in tree {
        let bare = bare_name(path);
        if path.starts_with(&format!("{ROBOTS}/")) {
            if path.matches('/').count() != 1 {
                return Err(format!("`{path}`: `{ROBOTS}/` holds files only"));
            }
            continue;
        }
        if path.starts_with(&format!("{INTERRUPTS}/")) {
            if !INTERRUPT_FILES.contains(&bare) || path.matches('/').count() != 1 {
                return Err(format!(
                    "`{path}`: `{INTERRUPTS}/` holds only {}",
                    INTERRUPT_FILES.join(" and ")
                ));
            }
            continue;
        }
        if bare == "main.py" {
            return Err(format!(
                "`{path}`: `main.py` is every robot's entry name; a player file cannot take it"
            ));
        }
        if let Some(other) = seen.get(bare) {
            return Err(format!(
                "`{path}` and `{other}` share the name `{bare}`; a module's name is its file's stem, unique across the tree"
            ));
        }
        seen.insert(bare.to_string(), path.clone());
        shared.push((bare.to_string(), text.clone()));
    }
    let mut out = BTreeMap::new();
    for (path, text) in tree {
        let Some(file) = path.strip_prefix(&format!("{ROBOTS}/")) else {
            continue;
        };
        let Some(deployment) = file.strip_suffix(".py") else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let mut files = shared.clone();
        files.push(("main.py".to_string(), text.clone()));
        files.sort();
        out.insert(deployment.to_string(), Bundle { files });
    }
    Ok(out)
}

/// The bundles a programs directory composes.
pub fn read_tree(root: &Path) -> Result<BTreeMap<String, Bundle>, String> {
    compose(&read_tree_files(root)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_composes_one_bundle_per_robot_with_the_shared_files() {
        let mut t = Tree::new();
        t.insert("robots/1.py".into(), "import nav\n".into());
        t.insert("robots/2.py".into(), "".into());
        t.insert("robots/printer.py".into(), "print(1)\n".into());
        t.insert(
            "interrupts/on_fault.py".into(),
            "def on_fault(e):\n    pass\n".into(),
        );
        t.insert("lib/nav.py".into(), "x = 1\n".into());
        let b = compose(&t).unwrap();
        assert_eq!(b.keys().cloned().collect::<Vec<_>>(), ["1", "printer"]);
        let names: Vec<&str> = b["1"].files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["main.py", "nav.py", "on_dying.py", "on_fault.py"]);
        assert_eq!(b["1"].files[0].1, "import nav\n");
        assert_eq!(b["1"].files[3].1, "def on_fault(e):\n    pass\n");
        // The interrupt files are in every bundle even when the tree lacks
        // them, empty.
        assert_eq!(
            b["printer"]
                .files
                .iter()
                .find(|(n, _)| n == "on_dying.py")
                .unwrap()
                .1,
            ""
        );
        t.insert("other/nav.py".into(), "".into());
        assert!(compose(&t).unwrap_err().contains("share the name `nav.py`"));
        t.remove("other/nav.py");
        t.insert("interrupts/extra.py".into(), "".into());
        assert!(compose(&t).unwrap_err().contains("holds only"));
        t.remove("interrupts/extra.py");
        t.insert("old/main.py".into(), "".into());
        assert!(
            compose(&t)
                .unwrap_err()
                .contains("every robot's entry name")
        );
    }
}
