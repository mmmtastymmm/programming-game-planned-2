#!/usr/bin/env node
// PROVENANCE: descended from the predecessor project's check-register-counts.mjs
// (../programming_game_planned). The incidents cited below happened THERE. They
// are kept because they are the evidence that justifies the check — not because
// they happened in this repo.
//
// The registers keep their shape: live docs hold CURRENT STATE ONLY, closed
// records live in bounded history shards. No deps, like the other doc checks.
//
//   node scripts/check-registers.mjs docs
//
// ── Why the split ───────────────────────────────────────────────────────────
// A register that appends forever is read in full by everyone who reads it at
// all, forever. The predecessor's QUESTIONS.md reached 166 KB of mostly
// answered material before anyone split it, and its PROBLEMS.md kept every
// fixed entry inline. Both files were opened constantly and were almost
// entirely stale on any given read. So:
//
//   QUESTIONS.md   open questions only     PROBLEMS.md   open entries only
//   history/questions-answered-NNN.md      history/problems-fixed-NNN.md
//   history/questions-worksheets-NNN.md
//
// ── Why the counts are derived ──────────────────────────────────────────────
// "74 opened, 65 fixed — nine open" is a fact about a register, and a fact
// about a register should not be maintained by hand inside it. In the
// predecessor it drifted every single time the register changed on 2026-08-15 —
// the headline, a tally, a section preamble, and a mirror of the same number in
// another file: four stale counts in one day, one of them stale again inside
// the very commit that fixed the other three. That is not carelessness that
// more care fixes; it is a derived value stored in prose. Splitting the fixed
// entries into history makes it worse, not better, by hand — the number now
// lives in a different file from the entries it counts. So it is derived here.
//
// ── What is enforced ────────────────────────────────────────────────────────
//   1. SINGLE SOURCE. Only PROBLEMS.md states register totals; its `(latest)`
//      headline is recomputed from open entries plus every fixed shard. Other
//      docs name individual entries and their relationships ("P29 closes as a
//      consequence of Q127") — information they own, which cannot go stale as
//      the register grows — but never restate the totals.
//   2. APPEND-ONLY NUMBERING. P- and Q-numbers are dense from 1 and unique
//      across live doc + history. An entry may not be open and closed at once.
//   3. SHARDS ARE ORDERED AND SELF-LABELLING. Shard NNN holds a contiguous
//      block of numbers, all of them above every number in shard NNN-1, and its
//      H1 states the range it actually contains — derived and checked, like the
//      headline. No index table anywhere: the headings are the index.
//        grep -H '^# ' docs/history/*.md
//   4. NOTHING GROWS WITHOUT BOUND. Live registers and history shards alike
//      fail above MAX_BYTES. That is what makes sharding mechanical rather than
//      a judgment call nobody makes until the file is already 166 KB.
//
// SCOPE: only the `(latest)` status block is validated. Earlier dated blocks are
// closed records of what was true on their date — "40 opened, 33 fixed" on
// 2026-08-14 is history, not a claim about today, and recomputing it would
// destroy the record.

import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = process.argv[2] ?? "docs";
const historyDir = join(root, "history");

// Roughly 10k tokens. The threshold is arbitrary; having one is not. A file
// this size is still openable, and the fix (start the next shard, or sweep
// closed entries out of a live register) is cheap at this point and expensive
// three doublings later.
const MAX_BYTES = 40 * 1024;

const problems = [];
const note = (msg) => problems.push(msg);

// ── Registers ───────────────────────────────────────────────────────────────
// `live` holds current state; `shards` hold closed records. Adding a register
// means adding a row here and nothing else.
const REGISTERS = [
  {
    what: "problems",
    prefix: "P",
    live: "PROBLEMS.md",
    liveSection: "open",
    shardPattern: /^problems-fixed-(\d{3})\.md$/,
    closedSection: "fixed",
  },
  {
    what: "questions",
    prefix: "Q",
    live: "QUESTIONS.md",
    liveSection: "open",
    shardPattern: /^questions-answered-(\d{3})\.md$/,
    closedSection: "answered",
  },
  {
    what: "worksheets",
    prefix: "Q",
    // Worksheets have no live half: an open question's worksheet lives in
    // QUESTIONS.md as part of the question itself, and moves here on answering.
    live: null,
    shardPattern: /^questions-worksheets-(\d{3})\.md$/,
    closedSection: "worksheet",
    // A question can be answered without ever having had a worksheet, so this
    // family is not checked for density.
    sparse: true,
  },
];

/** Entries are `**P12 — title**` at the start of a line. */
function entriesIn(path, prefix) {
  if (!existsSync(path)) return [];
  const re = new RegExp(`^\\*\\*(${prefix}(\\d+)) —`);
  return readFileSync(path, "utf8")
    .split("\n")
    .flatMap((line, i) => {
      const m = re.exec(line);
      return m ? [{ id: m[1], n: Number(m[2]), file: path, line: i + 1 }] : [];
    });
}

function shardsFor(pattern) {
  if (!existsSync(historyDir)) return [];
  return readdirSync(historyDir)
    .flatMap((name) => {
      const m = pattern.exec(name);
      return m ? [{ name, index: Number(m[1]), path: join(historyDir, name) }] : [];
    })
    .sort((a, b) => a.index - b.index);
}

function checkSize(path) {
  const bytes = statSync(path).size;
  if (bytes > MAX_BYTES) {
    note(
      `${path}  is ${Math.round(bytes / 1024)} KB, over the ${MAX_BYTES / 1024} KB cap — ` +
        `close this shard and start the next, or sweep closed entries into history`,
    );
  }
}

// ── Per-register checks ─────────────────────────────────────────────────────
for (const reg of REGISTERS) {
  const livePath = reg.live ? join(root, reg.live) : null;
  if (livePath && !existsSync(livePath)) {
    note(`missing register: ${livePath}`);
    continue;
  }

  const open = livePath ? entriesIn(livePath, reg.prefix) : [];
  if (livePath) checkSize(livePath);

  const shards = shardsFor(reg.shardPattern);
  const closed = [];
  let previousMax = 0;

  for (const shard of shards) {
    checkSize(shard.path);
    const here = entriesIn(shard.path, reg.prefix);
    closed.push(...here);

    // Ordered: every number in this shard is above every number in the last.
    const lo = here.length ? Math.min(...here.map((e) => e.n)) : null;
    const hi = here.length ? Math.max(...here.map((e) => e.n)) : null;
    if (lo !== null && lo <= previousMax) {
      note(
        `${shard.path}  holds ${reg.prefix}${lo}, which belongs in an earlier shard ` +
          `(the previous shard reaches ${reg.prefix}${previousMax}) — shards are filled in order`,
      );
    }
    if (hi !== null) previousMax = Math.max(previousMax, hi);

    // Self-labelling: the H1 states the range it actually holds.
    const first = readFileSync(shard.path, "utf8").split("\n").find((l) => l.startsWith("# "));
    if (!first) {
      note(`${shard.path}:1  no H1 — a shard's heading is how a reader finds it`);
    } else {
      const want = lo === null ? "(empty)" : `${reg.prefix}${lo}–${reg.prefix}${hi}`;
      const said = / — (.+)$/.exec(first)?.[1];
      if (said !== want) {
        note(
          `${shard.path}:1  heading says "${said ?? "no range"}", contents are ${want} — ` +
            `the range is derived, so state what is actually there`,
        );
      }
      if (lo === null && shard.index !== shards[shards.length - 1].index) {
        note(`${shard.path}  is empty but is not the last shard`);
      }
    }
  }

  // Unique, and never both open and closed.
  const seen = new Map();
  for (const e of [...open, ...closed]) {
    const at = `${e.file}:${e.line}`;
    if (seen.has(e.n)) note(`${e.id} appears twice (${seen.get(e.n)} and ${at})`);
    else seen.set(e.n, at);
  }

  // Dense: numbering is append-only and never renumbered, so gaps are lost
  // entries rather than deliberate holes.
  if (!reg.sparse) {
    const total = open.length + closed.length;
    for (let n = 1; n <= total; n++) {
      if (!seen.has(n)) {
        note(
          `${reg.prefix}${n} is missing from the ${reg.what} register — ` +
            `numbering must be dense (append, never renumber)`,
        );
      }
    }
  }

  reg.openEntries = open;
  reg.closedEntries = closed;
}

// ── The one headline allowed to state totals ────────────────────────────────
const WORDS = [
  "zero","one","two","three","four","five","six","seven","eight","nine","ten",
  "eleven","twelve","thirteen","fourteen","fifteen","sixteen","seventeen",
  "eighteen","nineteen","twenty",
];
const num = (s) => (/^\d+$/.test(s) ? Number(s) : WORDS.indexOf(s.toLowerCase()));

const registerPath = join(root, "PROBLEMS.md");
const HEADLINE = /^\*\*Status (\d{4}-\d{2}-\d{2}) \(latest\): (\d+) opened, (\d+) fixed — ([\w-]+) open\.\*\*/;

if (existsSync(registerPath)) {
  const reg = REGISTERS.find((r) => r.what === "problems");
  const lines = readFileSync(registerPath, "utf8").split("\n");
  const found = lines
    .map((l, i) => ({ m: HEADLINE.exec(l), line: i + 1 }))
    .filter((x) => x.m);

  if (found.length === 0) {
    note(
      `no "(latest)" status headline in ${registerPath} — expected ` +
        `**Status YYYY-MM-DD (latest): N opened, M fixed — K open.**`,
    );
  } else if (found.length > 1) {
    note(
      `${found.length} headlines claim "(latest)" (lines ${found.map((f) => f.line).join(", ")}) — exactly one may`,
    );
  } else {
    const [, date, opened, fixedSaid, openSaid] = found[0].m;
    const at = `${registerPath}:${found[0].line}`;
    const want = [
      ["opened", Number(opened), reg.openEntries.length + reg.closedEntries.length],
      ["fixed", Number(fixedSaid), reg.closedEntries.length],
      ["open", num(openSaid), reg.openEntries.length],
    ];
    for (const [what, said, real] of want) {
      if (said !== real) {
        note(
          `${at}  the ${date} headline says ${said} ${what}, the register has ${real}` +
            (what === "open" ? ` (${reg.openEntries.map((e) => e.id).join(", ")})` : ""),
        );
      }
    }
  }
}

// ── Nobody else states the totals ───────────────────────────────────────────
const TOTALS = [
  /\b\d+ opened, \d+ fixed\b/,
  /problem register carries [\w-]+ open entr/i,
  /\bregister (?:carries|holds) \d+\b/i,
];
function walk(dir) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    // history/ is exempt: a closed record of what was true on its date is not a
    // claim about today.
    if (e.name === "history") continue;
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.endsWith(".md") && p !== registerPath) {
      readFileSync(p, "utf8")
        .split("\n")
        .forEach((l, i) => {
          // A doc explaining this very rule has to be able to quote the shape
          // it forbids. Backticked spans are quotation, not assertion — the
          // same carve-out check-links makes for link syntax.
          const said = l.replace(/`[^`]*`/g, "");
          if (TOTALS.some((re) => re.test(said))) {
            note(
              `${p}:${i + 1}  restates a register total — counts live in PROBLEMS.md only; ` +
                `cite entries, not totals`,
            );
          }
        });
    }
  }
}
if (existsSync(root)) walk(root);

if (problems.length) {
  console.error(`✗ ${problems.length} register problem(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

const summary = REGISTERS.map(
  (r) => `${r.what} ${r.openEntries.length} open / ${r.closedEntries.length} closed`,
).join(", ");
console.log(`✓ registers consistent and within size: ${summary}`);
