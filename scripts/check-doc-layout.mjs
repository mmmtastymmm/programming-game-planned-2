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
// parent to point at. Every OTHER .md under docs/ is checked, at any depth, and
// docs/history/ is skipped: it holds closed records that are deliberately kept
// as they were received.
//
// Depth matters. Discovery used to stop one level down, which is not what the
// convention describes: CLAUDE.md's split is Rust-module style, and Rust modules
// nest. A part at docs/01-language/runtime/vm.md was never opened — and, worse,
// never counted, so the run reported "✓ 1 part files" while silently skipping
// one. A doc directory holding only subdirectories produced an affirmatively
// false "no split docs yet". A nested part names its immediate parent doorway
// (runtime/vm.md points at ../runtime.md), which check-links then resolves.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { join, basename } from "node:path";

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

/** Every part file under a doc directory, at any depth. */
function checkDir(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      checkDir(full);
      continue;
    }
    if (!entry.name.endsWith(".md")) continue;
    const lines = readFileSync(full, "utf8").split("\n");
    checked++;

    // The doorway a part points at is its IMMEDIATE parent directory, so this
    // reads the same at any depth.
    const parent = basename(dir);
    const want = `*Part of [${parent}](../${parent}.md).*`;
    if (lines[0] !== want) {
      const where = lines.findIndex((l) => l.startsWith("*Part of "));
      problems.push(
        where === -1
          ? `${full}:1  no breadcrumb — expected ${want}`
          : `${full}:1  breadcrumb is on line ${where + 1}, not line 1` +
            (lines[where] === want ? "" : `, and reads ${lines[where]}`),
      );
      continue;
    }
    if (!(lines[1] === "" && lines[2]?.startsWith("# "))) {
      problems.push(`${full}:3  expected a blank line then the H1 under the breadcrumb`);
    }
  }
}

for (const entry of readdirSync(root, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name === "history") continue;
  checkDir(join(root, entry.name));
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
