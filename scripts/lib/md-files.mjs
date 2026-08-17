// Every .md file under a root, depth-first. Shared by check-links.mjs and
// check-mermaid.mjs so there is one definition of "which files are ours".
//
// `withFileTypes` gives one syscall per directory rather than a stat per entry,
// which matters now that check-links walks the whole repo rather than docs/.

import { readdirSync } from "node:fs";
import { join } from "node:path";

// check-links runs from the repo root and meets all three; check-mermaid only
// ever walks docs/ and meets none. Kept in one place regardless — a walker that
// descends into target/ is a bug waiting for the first caller who widens a root.
const SKIP = new Set([".git", "node_modules", "target"]);

export function markdownFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (SKIP.has(entry.name)) return [];
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return markdownFiles(full);
    return full.endsWith(".md") ? [full] : [];
  });
}
