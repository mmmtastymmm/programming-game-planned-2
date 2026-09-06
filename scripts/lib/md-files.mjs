// Every .md file under a root, depth-first. Shared by check-links,
// check-mermaid, check-registers and check-structure so there is one definition
// of "which files are ours".
//
// `withFileTypes` gives one syscall per directory rather than a stat per entry,
// which matters now that check-links walks the whole repo rather than docs/.

import { readdirSync } from "node:fs";
import { join } from "node:path";

// Every caller now walks the repo root, so all three of these are live: a
// nested workspace's target/ once put 528 MB into this repo, node_modules sits
// under scripts/, and .git is .git. A walker that descends into any of them is
// a bug waiting for the first caller who widens a root — which is exactly what
// happened when check-mermaid's default moved from docs/ to the repo root.
//
// `.idea`/`.vscode` are here rather than only in check-checks' fixture list: a
// stray `.idea/notes.md` is nobody's corpus, and skipping it in the fixture but
// not in the walker made the meta-check's baseline pass a tree the real run
// fails — while its failure message asserts the fixture "fails here for the same
// reason it would fail on its own".
export const SKIP = new Set([".git", "node_modules", "target", ".idea", ".vscode"]);

export function markdownFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (SKIP.has(entry.name)) return [];
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return markdownFiles(full);
    return full.endsWith(".md") ? [full] : [];
  });
}
