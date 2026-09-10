//! The player's programs on disk: one directory per deployment under a
//! root, each holding the bundle's `.py` files. The renderer reads them and
//! submits a `Deploy` for each; nothing else about a program lives in the
//! game (docs/05: programs live in the player's editor).

use sim::{Bundle, CommandKind};
use std::collections::BTreeMap;
use std::path::Path;

/// Every bundle under `root`, by deployment name, in name order.
pub fn read_all(root: &Path) -> Result<BTreeMap<String, Bundle>, String> {
    let mut out = BTreeMap::new();
    let entries = std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        match sim::script::read_bundle(&entry.path()) {
            Ok(b) => {
                out.insert(name, b);
            }
            Err(_) => continue,
        }
    }
    Ok(out)
}

/// The deploys for every bundle whose files differ from `deployed`.
pub fn deploys_for(
    bundles: &BTreeMap<String, Bundle>,
    deployed: &BTreeMap<String, Bundle>,
) -> Vec<CommandKind> {
    bundles
        .iter()
        .filter(|(name, b)| deployed.get(*name) != Some(b))
        .map(|(name, b)| CommandKind::Deploy {
            deployment: name.clone(),
            bundle: b.clone(),
        })
        .collect()
}
