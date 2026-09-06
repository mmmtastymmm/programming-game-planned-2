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
// A table can lose either half, and for a long time only one half was checked.
// Every rule below was reached through `isTableSeparator`, so a table that lost
// its SEPARATOR instead of its header was not a malformed table — it was not a
// table at all: no header check, no column check, and not counted in the "N
// tables well formed" tick, which is the mislabelled-count failure this repo
// keeps finding. It renders on GitHub exactly as badly as the case above, and it
// is exactly as mechanical to spot.
//
// SCOPE: code blocks are stripped first — fenced and four-space-indented alike —
// because this corpus quotes headings and tables constantly.

import { readFileSync } from "node:fs";
import { markdownFiles } from "./lib/md-files.mjs";
import { scanCode, cells, isTableSeparator } from "./lib/markdown.mjs";

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
  const { lines, unterminated } = scanCode(readFileSync(file, "utf8"));

  // ── Fences close ──────────────────────────────────────────────────────────
  // An unclosed fence is not a cosmetic defect: everything below it is blanked
  // for THIS check, for check-registers and for check-links, all three of which
  // then report their usual ✓ over a file they cannot see. Whoever runs the
  // checks has to be told, because nothing downstream can tell them.
  if (unterminated) {
    problems.push(
      `${file}:${unterminated.line}  unterminated ${unterminated.char.repeat(unterminated.len)} ` +
        `fence — every line below it is invisible to the structure, register and link checks`,
    );
  }

  // ── One heading, one place ────────────────────────────────────────────────
  // Keyed on the TEXT, not on "level + text". A splice that re-levels one of the
  // two copies is the same splice — and GitHub slugs `## M0 — Scaffolding` and
  // `### M0 — Scaffolding` to the same `#m0--scaffolding`, so the two are not
  // even distinguishable to a link. The Q13 incident above happened to be
  // same-level; nothing about it says the next one will be. The levels go in the
  // message instead, where they tell the reader which copy to keep.
  const seen = new Map();
  lines.forEach((line, i) => {
    const m = /^(#{1,6})\s+(.*\S)\s*$/.exec(line);
    if (!m) return;
    headings++;
    const first = seen.get(m[2]);
    if (first) {
      problems.push(
        `${file}:${i + 1}  duplicate heading "${m[2]}" (${m[1]} here, ${first.level} at line ` +
          `${first.line}) — a file that says the same thing twice is a splice, and the two ` +
          `copies disagree`,
      );
    } else {
      seen.set(m[2], { line: i + 1, level: m[1] });
    }
  });

  // ── A table has a separator ───────────────────────────────────────────────
  // A run of pipe rows with no `|---|` among them is not a table to GFM, so it
  // renders as literal `| a | b |` text. Only rows that START with a pipe count:
  // that is how every table in this corpus is written, and requiring it keeps an
  // ordinary sentence containing a pipe from being read as a broken table. Two
  // rows is the threshold because GFM needs a header AND a separator before
  // anything is a table, so a single stray pipe line is not a truncated one.
  for (let i = 0; i < lines.length; i++) {
    if (!/^\s*\|/.test(lines[i])) continue;
    let j = i;
    while (j < lines.length && /^\s*\|/.test(lines[j])) j++;
    const run = lines.slice(i, j);
    if (run.length >= 2 && !run.some(isTableSeparator)) {
      problems.push(
        `${file}:${i + 1}  ${run.length} pipe rows with no |---|---| separator among them — ` +
          `GFM needs one under the header, and without it the whole block renders as ` +
          `literal text`,
      );
    }
    i = j - 1;
  }

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
    // Body rows, until the table ends. A row STARTS with a pipe, exactly as the
    // separator run above requires and for the same reason it gives: an ordinary
    // sentence that happens to contain a pipe — `a | b`, directly under the table
    // with no blank line — was column-counted as a body row and reported as a
    // broken table, a complaint about the one line in the block that is not part
    // of one.
    for (let j = i + 1; j < lines.length; j++) {
      const row = lines[j];
      if (!/^\s*\|/.test(row)) break;
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

console.log(`✓ ${headings} headings unique per file, ${tables} tables well formed, every fence closed`);
