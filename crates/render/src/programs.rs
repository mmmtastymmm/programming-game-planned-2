//! The player's programs on disk: one tree (`docs/07`, Q41) — `robots/`,
//! `interrupts/`, and the player's own files — read at start and written
//! on export; the game never watches it.

use sim::script::{Tree, compose, read_tree_files};
use sim::{Bundle, CommandKind};
use std::collections::BTreeMap;
use std::path::Path;

/// The tree under `root` and the bundles it composes.
pub fn read_all(root: &Path) -> Result<(Tree, BTreeMap<String, Bundle>), String> {
    let tree = read_tree_files(root)?;
    let bundles = compose(&tree)?;
    Ok((tree, bundles))
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
