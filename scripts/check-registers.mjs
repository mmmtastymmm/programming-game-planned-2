#!/usr/bin/env node
// PROVENANCE: descended from the predecessor project's check-register-counts.mjs
// (../programming_game_planned). The incidents cited below happened THERE. They
// are kept because they are the evidence that justifies the check — not because
// they happened in this repo.
//
//   node scripts/check-registers.mjs docs
//
// ── The scheme ──────────────────────────────────────────────────────────────
// Four registers. Each keeps OPEN entries in a live doc and moves each CLOSED
// entry to a file of its own under docs/history/:
//
//   QUESTIONS.md  -> history/questions-answered/question-answered-0013.md
//   PROBLEMS.md   -> history/problems-fixed/problem-fixed-0007.md
//   TASKS.md      -> history/tasks-completed/task-completed-0003.md
//   INBOX.md      -> history/inbox-triaged/inbox-triaged-0007.md
//
// One file per entry is what makes the filename the index: `ls` is the table of
// contents, so there is no index table to maintain and none to go stale, and
// reading one ruling costs one ruling. An earlier scheme batched entries into
// numbered shards and needed three extra rules to keep them straight; all three
// existed only because a shard held many entries.
//
// ── Why the counts are derived ──────────────────────────────────────────────
// "74 opened, 65 fixed — nine open" is a fact about a register, and a fact about
// a register should not be maintained by hand inside it. In the predecessor it
// drifted four times in one day, one of them stale again inside the very commit
// that fixed the other three. Splitting closed entries into history makes a
// hand-kept count strictly worse, since the number now lives in a different file
// from what it counts.
//
// ── What is enforced ────────────────────────────────────────────────────────
//   1. SINGLE SOURCE. Only PROBLEMS.md states register totals, and its headline
//      is recomputed from the entries. No doc restates a total.
//   2. APPEND-ONLY NUMBERING, dense from 1, unique across live doc + history.
//      An entry is open or closed, never both.
//   3. THE FILENAME IS THE RECORD. `question-answered-0013.md` holds Q13 and
//      nothing else, and its heading must say so.
//   4. NOTHING GROWS WITHOUT BOUND. Every markdown file under the corpus is
//      capped, which is what makes splitting a doc mechanical rather than a
//      judgment call nobody makes until the file is already 166 KB.
//   5. EVERY CLOSURE DECLARES ITS CONSEQUENCE. An answered question and a
//      triaged inbox entry carry an `## Outcome` naming where it went, and every
//      number they cite must resolve. The predecessor's characteristic failure
//      was a ruling made, recorded, and never propagated to the doc that owned
//      it — leaving the stalest text in the corpus in its most
//      authoritative-looking place. Only the inbox may declare `Dropped`: an
//      inbox that cannot absorb a false alarm stops being cheap to write to.
//   6. ONE STATUS BLOCK PER REGISTER, rewritten in place. `git log -p` is the
//      history; there is no archive, so stacking is the failure to catch.

import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { execFileSync } from "node:child_process";
import { markdownFiles } from "./lib/md-files.mjs";
import { stripFences } from "./lib/markdown.mjs";
import { MAX_BYTES, OUTCOME_KINDS, REGISTERS } from "./lib/registers.mjs";

// Takes the REPO root, not docs/. The registers live under docs/, but the size
// cap and the no-restated-totals rule have to cover CLAUDE.md and README.md too:
// CLAUDE.md is the file that *describes* the register scheme and so the one most
// likely to restate a total, and it was exempt from both while scoped to docs/.
const repoRoot = process.argv[2] ?? ".";
const root = join(repoRoot, "docs");
const historyDir = join(root, "history");

const problems = [];
const note = (msg) => problems.push(msg);

// A check that scores green on zero inputs is the failure this repo keeps
// naming and keeps committing: the language spike passed four processes that ran
// nothing. crates/sim/tests/no_floats.rs guards this; the .mjs checks did not.
const allDocs = existsSync(repoRoot) ? markdownFiles(repoRoot) : [];
if (allDocs.length === 0) {
  console.error(`✗ check-registers: no markdown found under ${repoRoot} — the check is checking nothing`);
  process.exit(2);
}

const cited = [];
// Derived per-run state lives here, NOT on the objects exported by
// lib/registers.mjs. That module is documented as the authoritative description
// of the scheme and is now shared with check-vocabulary; a consumer writing scan
// results into it means reading it no longer tells you an entry's shape, and two
// checks sharing a process would see each other's data.
const scanned = new Map();
const entriesOf = (what) => scanned.get(what) ?? { open: [], closed: [] };

// ── Entries ─────────────────────────────────────────────────────────────────
/** Live-register entries are `**P12 — title**` at the start of a line. */
function liveEntries(path, prefix) {
  if (!existsSync(path)) return [];
  const re = new RegExp(`^\\*\\*(${prefix}(\\d+)) —`);
  return stripFences(readFileSync(path, "utf8")).flatMap((line, i) => {
    const m = re.exec(line);
    return m ? [{ id: m[1], n: Number(m[2]), file: path, line: i + 1 }] : [];
  });
}

/**
 * Parse an `## Outcome` section into bullets.
 *
 * Bullets WRAP, and markdown allows the wrap to be indented or "lazy"
 * (unindented). Bullets also nest. A parser that only accumulates indented
 * continuations misses every citation on a lazy wrap; one that anchors bullet
 * starts at column 0 swallows nested bullets as prose, so their kind is never
 * validated and their citations never resolve. Both holes shipped before this
 * was written as one loop.
 */
function parseBullets(lines, firstLineNo) {
  const START = /^\s*[-*+] \*\*(\w+):\*\*(.*)$/;
  const out = [];
  for (let i = 0; i < lines.length; i++) {
    const m = START.exec(lines[i]);
    if (!m) continue;
    let text = m[2];
    let j = i + 1;
    while (j < lines.length && lines[j].trim() !== "" && !START.test(lines[j]) && !/^#/.test(lines[j])) {
      text += " " + lines[j].trim();
      j++;
    }
    out.push({ kind: m[1], text, line: firstLineNo + i });
    i = j - 1;
  }
  return out;
}

function checkOutcome(file, text, allowed) {
  const body = stripFences(text);
  const at = body.findIndex((l) => /^## Outcome\s*$/.test(l));
  if (at === -1) {
    note(
      `${file}  has no "## Outcome" section — it must say where this went: ` +
        allowed.map((k) => `**${k}:**`).join(", "),
    );
    return;
  }
  const rest = body.slice(at + 1);
  const stop = rest.findIndex((l) => /^## /.test(l));
  const lines = stop === -1 ? rest : rest.slice(0, stop);
  const bullets = parseBullets(lines, at + 2);

  let declared = 0;
  for (const b of bullets) {
    if (!allowed.includes(b.kind)) {
      note(`${file}:${b.line}  Outcome declares "${b.kind}", which is not one of ${allowed.join(", ")}`);
      continue;
    }
    declared++;
    const spec = OUTCOME_KINDS[b.kind];
    if (spec) {
      const hits = [...b.text.matchAll(spec.re)];
      if (hits.length === 0) {
        note(
          `${file}:${b.line}  "${b.kind}:" cites no number — a declared consequence ` +
            `with nothing to resolve is not a consequence`,
        );
      }
      for (const c of hits) {
        cited.push({ what: spec.what, id: c[1], n: Number(c[2]), file, line: b.line });
      }
    } else if (b.kind === "Docs" && !/\]\(/.test(b.text)) {
      // The cheapest way to satisfy this gate used to be a bare "- **Docs:**",
      // which is precisely the un-propagated state the gate exists to catch.
      note(`${file}:${b.line}  "Docs:" links nothing — link the edits it claims`);
    } else if (b.kind === "Dropped" && b.text.trim().length < 12) {
      note(`${file}:${b.line}  Dropped without a reason — say why it was not real`);
    }
  }
  if (declared === 0) {
    note(
      `${file}:${at + 1}  the Outcome section declares nothing — expected at least one of ` +
        allowed.map((k) => `"- **${k}:**"`).join(", "),
    );
  }
}

/** One entry per file; the filename carries the number and the heading must agree. */
function historyEntries(reg) {
  const path = join(historyDir, reg.dir);
  if (!existsSync(path)) {
    // Silence here is how a renamed or typo'd directory stays invisible for any
    // register that happens to have no closed entries yet.
    note(`${path}  does not exist — every register keeps its closed entries in one`);
    return [];
  }
  const nameRe = new RegExp(`^${reg.filePrefix}-(\\d{4})\\.md$`);
  const found = [];
  for (const name of readdirSync(path).sort()) {
    if (name === "README.md" || name.startsWith(".")) continue;
    const file = join(path, name);
    if (!name.endsWith(".md")) {
      note(`${file}  is not a markdown file — this directory holds one entry per file`);
      continue;
    }
    const m = nameRe.exec(name);
    if (!m) {
      note(`${file}  is misnamed — expected ${reg.filePrefix}-NNNN.md (four digits)`);
      continue;
    }
    const n = Number(m[1]);
    if (n === 0) {
      note(`${file}  is numbered 0 — registers number from 1 (${reg.prefix}1…)`);
      continue;
    }
    const text = readFileSync(file, "utf8");
    const heads = stripFences(text).flatMap((l, i) => {
      const h = new RegExp(`^# (${reg.prefix}(\\d+)) — `).exec(l);
      return h ? [{ id: h[1], n: Number(h[2]), line: i + 1 }] : [];
    });
    if (heads.length === 0) {
      note(`${file}:1  no entry heading — expected "# ${reg.prefix}${n} — <title>"`);
      continue;
    }
    if (heads.length > 1) {
      note(
        `${file}  holds ${heads.length} entries (${heads.map((h) => h.id).join(", ")}) — ` +
          `one entry per file, so that reading one ruling costs one ruling`,
      );
      continue;
    }
    if (heads[0].n !== n) {
      note(
        `${file}:${heads[0].line}  is named for ${reg.prefix}${n} but its heading says ` +
          `${heads[0].id} — the filename is the index, so the two must agree`,
      );
      continue;
    }
    if (reg.outcome) checkOutcome(file, text, reg.outcome);
    found.push({ id: heads[0].id, n, file, line: heads[0].line });
  }
  return found;
}

for (const reg of REGISTERS) {
  const livePath = join(root, reg.live);
  if (!existsSync(livePath)) {
    note(`missing register: ${livePath}`);
    scanned.set(reg.what, { open: [], closed: [] });
    continue;
  }
  const open = liveEntries(livePath, reg.prefix);
  const closed = historyEntries(reg);

  const seen = new Map();
  for (const e of [...open, ...closed]) {
    const at = `${e.file}:${e.line}`;
    if (e.n === 0) note(`${at}  is numbered 0 — registers number from 1 (${reg.prefix}1…)`);
    if (seen.has(e.n)) note(`${e.id} appears twice (${seen.get(e.n)} and ${at})`);
    else seen.set(e.n, at);
  }
  // Dense from 1. Note the honest limit: the upper bound is derived from the
  // entry count, so deleting the HIGHEST-numbered entry shrinks the range and
  // is undetectable here. Gaps below the top are caught.
  const total = open.length + closed.length;
  for (let n = 1; n <= total; n++) {
    if (!seen.has(n)) {
      note(
        `${reg.prefix}${n} is missing from the ${reg.what} register — ` +
          `numbering must be dense (append, never renumber)`,
      );
    }
  }

  scanned.set(reg.what, { open, closed });
}

// ── A cited number must resolve ─────────────────────────────────────────────
for (const c of cited) {
  const { open, closed } = entriesOf(c.what);
  const known = new Set([...open, ...closed].map((e) => e.n));
  if (!known.has(c.n)) {
    note(
      `${c.file}:${c.line}  the Outcome cites ${c.id}, which is not in the ${c.what} ` +
        `register — a declared consequence that does not exist is the failure this ` +
        `section was added to catch`,
    );
  }
}

// ── A cited commit must exist ───────────────────────────────────────────────
// Only inside history/, only backticked, 7-40 hex chars, and only if it contains
// a digit — which excludes all-letter words that happen to be valid hex
// ("defaced") while keeping every realistic short hash.
{
  const HASH = /`([0-9a-f]{7,40})`/g;
  let gitWorks = true;
  const seen = new Map();
  for (const file of allDocs) {
    if (!file.includes(`${sep}history${sep}`)) continue;
    for (const m of readFileSync(file, "utf8").matchAll(HASH)) {
      if (!/\d/.test(m[1])) continue;
      if (!seen.has(m[1])) seen.set(m[1], file);
    }
  }
  for (const [hash, file] of seen) {
    if (!gitWorks) break;
    try {
      // Run in the repo, not in $root: the pre-commit hook points $root at an
      // extracted index with no .git in it.
      const kind = execFileSync("git", ["cat-file", "-t", hash], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim();
      if (kind !== "commit") {
        note(`${file}  cites \`${hash}\`, which is a ${kind}, not a commit`);
      }
    } catch (err) {
      if (err.code === "ENOENT") {
        note("git not found — cannot verify the commit hashes cited in history/");
        gitWorks = false;
      } else {
        note(
          `${file}  cites \`${hash}\`, which is not a commit in this repository — ` +
            `a history rewrite invalidates every hash recorded before it`,
        );
      }
    }
  }
}

// ── One status block per register, rewritten in place ───────────────────────
const WORDS = [
  "zero","one","two","three","four","five","six","seven","eight","nine","ten",
  "eleven","twelve","thirteen","fourteen","fifteen","sixteen","seventeen",
  "eighteen","nineteen","twenty",
];
const STATUS = /^\*\*Status (\d{4}-\d{2}-\d{2})/;
const COUNTED = /^\*\*Status (\d{4}-\d{2}-\d{2}): (\d+) opened, (\d+) fixed — ([\w-]+) open\.\*\*/;

for (const reg of REGISTERS.filter((r) => r.status)) {
  const path = join(root, reg.live);
  if (!existsSync(path)) continue;
  const lines = stripFences(readFileSync(path, "utf8"));
  const blocks = lines.map((l, i) => ({ l, line: i + 1 })).filter((x) => STATUS.test(x.l));

  if (blocks.length === 0) {
    note(`${path}  has no status block — expected one "**Status YYYY-MM-DD…**"`);
    continue;
  }
  if (blocks.length > 1) {
    note(
      `${path}  has ${blocks.length} status blocks (lines ${blocks.map((b) => b.line).join(", ")}) — ` +
        `a register states its status once and rewrites it in place; git holds the history`,
    );
    continue;
  }
  if (reg.status !== "counted") continue;

  const here = entriesOf(reg.what);
  const m = COUNTED.exec(blocks[0].l);
  if (!m) {
    note(
      `${path}:${blocks[0].line}  malformed status headline — expected ` +
        `**Status YYYY-MM-DD: N opened, M fixed — K open.**`,
    );
    continue;
  }
  const [, date, opened, fixedSaid, openSaid] = m;
  const at = `${path}:${blocks[0].line}`;
  let openNum = /^\d+$/.test(openSaid) ? Number(openSaid) : WORDS.indexOf(openSaid.toLowerCase());
  if (openNum < 0) {
    note(`${at}  "${openSaid}" is not a number this checker knows — use digits above twenty`);
    openNum = null;
  }
  const want = [
    ["opened", Number(opened), here.open.length + here.closed.length],
    ["fixed", Number(fixedSaid), here.closed.length],
    ["open", openNum, here.open.length],
  ];
  for (const [what, said, real] of want) {
    if (said !== null && said !== real) {
      note(
        `${at}  the ${date} headline says ${said} ${what}, the register has ${real}` +
          (what === "open" ? ` (${here.open.map((e) => e.id).join(", ")})` : ""),
      );
    }
  }
}

// ── Nobody else states the totals ───────────────────────────────────────────
const NUM = "\\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|" +
  "thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty";
const TOTALS = [
  /\b\d+ opened, \d+ fixed\b/,
  /problem register carries [\w-]+ open entr/i,
  /\bregister (?:carries|holds) \d+\b/i,
  // "Eight questions are open." is the same hand-maintained derived value as a
  // problem-register total, in the file the tooling is supposed to protect. It
  // drifted twice in the session that introduced it. The adverb slot is there
  // because "three questions are STILL open" is the same claim.
  new RegExp(`\\b(${NUM})\\s+(questions?|tasks?|problems?|entries)\\s+(?:\\w+\\s+)?` +
    `(?:are|is|remain|remains)\\b`, "i"),
];
const registerPath = join(root, "PROBLEMS.md");

// One traversal, through the shared walker that already skips .git/node_modules/
// target — rather than a third hand-rolled one. `md-files.mjs` exists to be "one
// definition of which files are ours".
{
  for (const file of allDocs) {
    const bytes = statSync(file).size;
    if (bytes > MAX_BYTES) {
      note(
        `${file}  is ${Math.round(bytes / 1024)} KB, over the ${MAX_BYTES / 1024} KB cap — ` +
          `split it (CLAUDE.md, "Splitting a doc"), or sweep closed entries into history`,
      );
    }
    // history/ is exempt from the totals rule: a closed record of what was true
    // on its date is not a claim about today.
    const rel = relative(repoRoot, file).split(sep);
    if ((rel[0] === "docs" && rel[1] === "history") || file === registerPath) continue;
    stripFences(readFileSync(file, "utf8")).forEach((l, i) => {
      // A doc explaining this very rule has to be able to quote the shape it
      // forbids. Backticked spans are quotation, not assertion.
      const said = l.replace(/`[^`]*`/g, "");
      if (TOTALS.some((re) => re.test(said))) {
        note(
          `${file}:${i + 1}  restates a register total — counts live in PROBLEMS.md only; ` +
            `cite entries, not totals`,
        );
      }
    });
  }
}

if (problems.length) {
  console.error(`✗ ${problems.length} register problem(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

const summary = REGISTERS.map((r) => {
  const { open, closed } = entriesOf(r.what);
  return `${r.what} ${open.length} open / ${closed.length} closed`;
}).join(", ");
console.log(`✓ registers consistent and within size: ${summary}`);
