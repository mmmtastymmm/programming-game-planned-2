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
// SCOPE: only `docs/NN-name/` directories hold part files, and only they are
// checked — at any depth inside them. Every .md down there is a part, with one
// exception: a README.md is refused outright, and told which file already is the
// index. Doorways (docs/NN-name.md) and the registers sit at the top level and
// have no parent to point at; docs/history/ holds closed records kept as they
// were received; and any other directory a future commit adds (assets/,
// templates/) is not a split doc, so the breadcrumb this file demands would mean
// nothing there.
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
import { breadcrumb } from "./lib/doc-layout.mjs";

const root = process.argv[2] ?? "docs";
if (!existsSync(root)) {
  console.error(`check-doc-layout: no such directory: ${root}`);
  process.exit(2);
}

// docs/NN-name/part.md — the directory name is the doorway the crumb must name.
const problems = [];
let checked = 0;
let doorways = 0;

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
  // THE OTHER HALF OF THE SPLIT, and the half nothing looked for. CLAUDE.md
  // defines the convention as "a doorway `NN-name.md` beside an `NN-name/`
  // directory", and the doorway is where the invariants that cross the parts
  // live — the drift CLAUDE.md itself calls the split's characteristic failure
  // mode. A parts directory with no doorway at all passed clean: every part file
  // carried a correct breadcrumb, and the breadcrumb pointed at nothing. It was
  // caught only incidentally, by check-links resolving the crumb's href, so a
  // part naming its doorway in prose rather than as a link was invisible to both.
  //
  // Checked at EVERY level, because the parts nest: docs/01-language/runtime/
  // needs docs/01-language/runtime.md exactly as docs/01-language/ needs
  // docs/01-language.md.
  doorways++;
  if (!existsSync(`${dir}.md`)) {
    problems.push(
      `${dir}.md  a parts directory with no doorway beside it — ${basename(dir)}/ holds part ` +
        `files whose breadcrumb names ${basename(dir)}.md, which does not exist; the doorway ` +
        `owns the invariants that cross the parts`,
    );
  }
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      checkDir(full);
      continue;
    }
    if (!entry.name.endsWith(".md")) continue;

    // A README here is the one .md that is NOT a part file, and the old rule
    // could only tell it to become one: it was reported as missing a breadcrumb
    // that would have named the doorway it sits inside, so following the message
    // produced a file whose line 1 claims to be part of itself. Neither telling
    // it to lie nor waving it through is right — a split doc already HAS an
    // index, and CLAUDE.md says which one: the doorway holds "a table of what
    // each part owns". A second index beside the parts is the fact-stated-twice
    // failure the whole corpus is arranged against, and it is the copy a reader
    // arriving from `ls` would find first.
    if (entry.name === "README.md") {
      problems.push(
        `${full}:1  a split doc's index is its doorway ${basename(dir)}.md, which owns the ` +
          `table of what each part holds — a README here is a second index beside the parts`,
      );
      continue;
    }

    const lines = readFileSync(full, "utf8").split("\n");
    checked++;

    // The doorway a part points at is its IMMEDIATE parent directory, so this
    // reads the same at any depth.
    const parent = basename(dir);
    const want = breadcrumb(parent);
    if (lines[0] !== want) {
      const where = lines.findIndex((l) => l.startsWith("*Part of "));
      // Three distinct failures, and they used to share one message: a crumb on
      // line 1 naming the WRONG doorway — the copy-paste a new part file
      // actually makes — reported "breadcrumb is on line 1, not line 1", a
      // position complaint about a file whose position is right.
      problems.push(
        where === -1
          ? `${full}:1  no breadcrumb — expected ${want}`
          : where === 0
            ? `${full}:1  breadcrumb names the wrong doorway — expected ${want}, reads ${lines[0]}`
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

// A parts directory is `NN-name/`, beside its `NN-name.md` doorway — that is the
// convention CLAUDE.md defines, and the breadcrumb this check demands only makes
// sense for it. Descending into every non-history directory meant the first
// docs/assets/ or docs/templates/ holding a .md would be told to grow a doorway
// that has no reason to exist.
for (const entry of readdirSync(root, { withFileTypes: true })) {
  if (!entry.isDirectory() || !/^\d\d-/.test(entry.name)) continue;
  checkDir(join(root, entry.name));
}

if (problems.length) {
  // Not "part file(s) with the wrong opening" any more: the README case is a
  // file that must not be there at all, and a headline naming only the other
  // failure describes it wrongly.
  console.error(`✗ ${problems.length} problem(s) in split-doc directories:\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

console.log(
  checked === 0
    ? `✓ no split docs yet (${corpus.length} files at the top level, none with a parts directory)`
    : `✓ ${checked} part files open with their breadcrumb, then the H1, under ${doorways} doorway(s)`,
);
