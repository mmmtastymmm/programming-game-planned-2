#!/usr/bin/env node
// PROVENANCE: ported from the predecessor project (../programming_game_planned).
// The incidents cited below happened THERE. They are kept because they are the
// evidence that justifies the check — not because they happened in this repo.
// Every split-doc *part* file opens the same way: the breadcrumb on line 1, a
// blank, then the H1. No deps, like the link checker.
//
//   node scripts/check-doc-layout.mjs docs
//
// Why this exists: CLAUDE.md's split convention leans on the breadcrumb to tell
// a reader they are inside a directory rather than at a doorway, and seven of
// the 62 part files had quietly inverted it — H1 first, breadcrumb second. That
// is invisible to a reader who opens one file and fatal to any sweep that
// assumes line 1 (`head -1`, a future front-matter parser, a doc index built by
// script). It was found by a review reading all 62 files by hand, which is the
// expensive way to check a mechanical property.
//
// SCOPE: doorways (docs/NN-name.md) and the registers are exempt — they have no
// parent to point at. Only files one level down inside a doc directory are
// checked, and docs/history/ is skipped: it holds closed records that are
// deliberately kept as they were received.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { join, basename, dirname } from "node:path";

const root = process.argv[2] ?? "docs";
if (!existsSync(root)) {
  console.error(`check-doc-layout: no such directory: ${root}`);
  process.exit(2);
}

// docs/NN-name/part.md — the directory name is the doorway the crumb must name.
const problems = [];
let checked = 0;

// Zero part files is legitimate — no doc has been split yet — but an empty or
// wrong ROOT is not, and the two are indistinguishable in a "✓ 0" line. Assert
// the corpus is there; let the part-file count be zero honestly.
const corpus = readdirSync(root).filter((f) => f.endsWith(".md"));
if (corpus.length === 0) {
  console.error(`✗ check-doc-layout: no markdown directly under ${root} — wrong root?`);
  process.exit(2);
}

for (const entry of readdirSync(root, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name === "history") continue;
  const dir = join(root, entry.name);
  for (const f of readdirSync(dir)) {
    if (!f.endsWith(".md")) continue;
    const file = join(dir, f);
    const lines = readFileSync(file, "utf8").split("\n");
    checked++;

    const want = `*Part of [${entry.name}](../${entry.name}.md).*`;
    if (lines[0] !== want) {
      const where = lines.findIndex((l) => l.startsWith("*Part of "));
      problems.push(
        where === -1
          ? `${file}:1  no breadcrumb — expected ${want}`
          : `${file}:1  breadcrumb is on line ${where + 1}, not line 1` +
            (lines[where] === want ? "" : `, and reads ${lines[where]}`),
      );
      continue;
    }
    if (!(lines[1] === "" && lines[2]?.startsWith("# "))) {
      problems.push(`${file}:3  expected a blank line then the H1 under the breadcrumb`);
    }
  }
}

if (problems.length) {
  console.error(`✗ ${problems.length} part file(s) with the wrong opening:\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

console.log(
  checked === 0
    ? `✓ no split docs yet (${corpus.length} files at the top level, none with a parts directory)`
    : `✓ ${checked} part files open with their breadcrumb, then the H1`,
);
