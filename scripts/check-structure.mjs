#!/usr/bin/env node
// Structural sanity for every markdown file: no heading appears twice in one
// file, and every table is well formed.
//
//   node scripts/check-structure.mjs .
//
// WHY THIS EXISTS. Revising the Q13 ruling from a narrow language subset to a
// broad one located the end of the old text with `index("---")` — which matched
// the `|---|---|` separator inside the old boundary table rather than the
// horizontal rule below it. The new ruling was spliced in and ninety-one lines
// of the retired one survived underneath, so the authoritative record for the
// language boundary asserted both that `class` was excluded and that it was
// included, and that Q8's strictest option was viable and that it was harder.
// The corpus has a rule for a fact stated in two places; it had no check for a
// fact stated twice inside ONE file, and nothing noticed for two commits.
//
// Both symptoms of that splice are mechanical:
//   * `### Divergences from Python, in full` appeared twice.
//   * The surviving table lost its header, leaving a bare `---|---|` with no row
//     above it — which renders as literal text.
//
// SCOPE: fenced code blocks are stripped first, because this corpus quotes
// headings and tables constantly.

import { readFileSync } from "node:fs";
import { markdownFiles } from "./lib/md-files.mjs";
import { stripFences, cells, isTableSeparator } from "./lib/markdown.mjs";

const root = process.argv[2] ?? ".";
const files = markdownFiles(root);
if (files.length === 0) {
  console.error(`✗ check-structure: no markdown under ${root} — the check is checking nothing`);
  process.exit(2);
}

const problems = [];
let tables = 0;
let headings = 0;

for (const file of files) {
  const lines = stripFences(readFileSync(file, "utf8"));

  // ── One heading, one place ────────────────────────────────────────────────
  const seen = new Map();
  lines.forEach((line, i) => {
    const m = /^(#{1,6})\s+(.*\S)\s*$/.exec(line);
    if (!m) return;
    headings++;
    const key = `${m[1]} ${m[2]}`;
    if (seen.has(key)) {
      problems.push(
        `${file}:${i + 1}  duplicate heading "${m[2]}" (also at line ${seen.get(key)}) — ` +
          `a file that says the same thing twice is a splice, and the two copies disagree`,
      );
    } else {
      seen.set(key, i + 1);
    }
  });

  // ── Tables are well formed ────────────────────────────────────────────────
  lines.forEach((line, i) => {
    if (!isTableSeparator(line)) return;
    tables++;
    const header = lines[i - 1];
    if (header === undefined || !header.includes("|") || header.trim() === "") {
      problems.push(
        `${file}:${i + 1}  table separator with no header row above it — ` +
          `the table lost its head, and renders as literal text`,
      );
      return;
    }
    const want = cells(header).length;
    if (cells(line).length !== want) {
      problems.push(
        `${file}:${i + 1}  separator has ${cells(line).length} columns, header has ${want}`,
      );
      return;
    }
    // Body rows, until the table ends.
    for (let j = i + 1; j < lines.length; j++) {
      const row = lines[j];
      if (!row.includes("|") || row.trim() === "") break;
      if (cells(row).length !== want) {
        problems.push(
          `${file}:${j + 1}  table row has ${cells(row).length} columns, header has ${want}`,
        );
      }
    }
  });
}

if (problems.length) {
  console.error(`✗ ${problems.length} structural problem(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

console.log(`✓ ${headings} headings unique per file, ${tables} tables well formed`);
