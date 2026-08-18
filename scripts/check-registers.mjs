#!/usr/bin/env node
// PROVENANCE: descended from the predecessor project's check-register-counts.mjs
// (../programming_game_planned). The incidents cited below happened THERE. They
// are kept because they are the evidence that justifies the check — not because
// they happened in this repo.
//
// The registers keep their shape: live docs hold CURRENT STATE ONLY, closed
// records live one-per-file under docs/history/. No deps, like the other checks.
//
//   node scripts/check-registers.mjs docs
//
// ── Why the split ───────────────────────────────────────────────────────────
// A register that appends forever is read in full by everyone who reads it at
// all, forever. The predecessor's QUESTIONS.md reached 166 KB of mostly answered
// material before anyone split it, and its PROBLEMS.md kept every fixed entry
// inline. So:
//
//   QUESTIONS.md   open questions only     PROBLEMS.md   open entries only
//   history/questions-answered/            history/problems-fixed/
//     question-answered-0013.md              problem-fixed-0007.md
//
//   TASKS.md       open tasks only          INBOX.md      untriaged only
//   history/tasks-completed/               history/inbox-triaged/
//     task-completed-0007.md                 inbox-triaged-0007.md
//
// ── Why one file per entry ──────────────────────────────────────────────────
// An earlier version of this scheme batched entries into numbered shards, and
// needed three rules to keep them straight: shards fill in order, each shard's
// H1 states the range it holds (derived and checked), and an empty shard must be
// the last one. All three exist only because a shard holds many entries. One
// file per entry deletes the category: THE FILENAME IS THE INDEX, `ls` is the
// table of contents, and reading one ruling costs exactly one ruling. It also
// removes the failure the shards were already heading toward — the first shard
// hit 24 KB at six entries, and would have needed splitting at about ten.
//
// ── Why the counts are derived ──────────────────────────────────────────────
// "74 opened, 65 fixed — nine open" is a fact about a register, and a fact about
// a register should not be maintained by hand inside it. In the predecessor it
// drifted every time the register changed on 2026-08-15 — the headline, a tally,
// a section preamble, and a mirror of the same number in another file: four
// stale counts in one day, one of them stale again inside the very commit that
// fixed the other three. Splitting closed entries into history makes a hand-kept
// count strictly worse, since the number now lives in a different file from what
// it counts. So it is derived here.
//
// ── What is enforced ────────────────────────────────────────────────────────
//   1. SINGLE SOURCE. Only PROBLEMS.md states register totals; its status
//      headline is recomputed from open entries plus the fixed directory. Other
//      docs name individual entries and their relationships ("P29 closes as a
//      consequence of Q127") — information they own, which cannot go stale as
//      the register grows — but never restate the totals.
//   2. APPEND-ONLY NUMBERING. P- and Q-numbers are dense from 1 and unique
//      across live doc + history. An entry may not be open and closed at once.
//   3. THE FILENAME IS THE RECORD. `question-answered-0013.md` must hold Q13 and
//      nothing else, and its H1 must say so. A file whose name and heading
//      disagree is the one way this scheme can rot, so it is the one thing
//      checked hardest.
//   4. NOTHING GROWS WITHOUT BOUND. Live registers and history entries alike
//      fail above MAX_BYTES.
//   5. EVERY RULING HAS A CONSEQUENCE. An answered question, and a triaged
//      inbox entry, must declare in an `## Outcome` section where it went: docs
//      edited now, or a question / problem / task opened. At least one —
//      commonly two, since a ruling that lands in a Decided section often also
//      creates work. Cited numbers must resolve.
//
//      The inbox alone may also declare `Dropped`, with a reason. An inbox that
//      cannot absorb a false alarm stops being cheap to write to, and then it
//      goes unused and the observations are lost instead. A ruling has no such
//      escape: one that changes nothing is a forgotten propagation.
//
//      This is the check with the most history behind it. The predecessor's
//      characteristic failure was a ruling that was made, recorded, and never
//      propagated to the doc that owned it — leaving the stalest text in the
//      repo sitting in the most authoritative-looking place, where reading
//      passes skim it because it looks settled. A ruling that closes with no
//      consequence anywhere is either not a real ruling or a propagation that
//      was forgotten, and the two are indistinguishable a month later.
//
// SCOPE: the register states its status ONCE, and that block is rewritten in
// place rather than stacked or archived. There is no status log: `git log -p
// docs/PROBLEMS.md` already records every status this file has ever carried,
// dated and attached to the commit that changed it, which is a stricter record
// than a hand-maintained archive and cannot be forgotten.

import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = process.argv[2] ?? "docs";
const historyDir = join(root, "history");

// Roughly 10k tokens. The threshold is arbitrary; having one is not.
const MAX_BYTES = 40 * 1024;

const problems = [];
const note = (msg) => problems.push(msg);

const REGISTERS = [
  {
    what: "problems",
    prefix: "P",
    live: "PROBLEMS.md",
    dir: "problems-fixed",
    filePrefix: "problem-fixed",
  },
  {
    what: "questions",
    prefix: "Q",
    live: "QUESTIONS.md",
    dir: "questions-answered",
    filePrefix: "question-answered",
    // No `Dropped`: a ruling that changes nothing anywhere is a forgotten
    // propagation, which is the failure this section exists to catch.
    outcome: ["Docs", "Question", "Problem", "Task"],
  },
  // Tasks are a register for the same reason the other two are: an answered
  // question's Outcome may cite `T<n>`, and a citation is only worth writing if
  // something checks that it resolves. Milestones remain groupings, not
  // entries — nothing is numbered by milestone.
  {
    what: "tasks",
    prefix: "T",
    live: "TASKS.md",
    dir: "tasks-completed",
    filePrefix: "task-completed",
  },
  // The inbox is a register too, and the only one whose entries may resolve to
  // NOTHING. See `Dropped` below.
  {
    what: "inbox",
    prefix: "I",
    live: "INBOX.md",
    dir: "inbox-triaged",
    filePrefix: "inbox-triaged",
    outcome: ["Docs", "Question", "Problem", "Task", "Dropped"],
  },
];

// Which register each Outcome kind points at, and how its citations are spelled.
// `Docs` is validated by check-links rather than here; `Dropped` cites nothing
// and instead has to say why.
const OUTCOME_KINDS = {
  Question: { what: "questions", re: /\b(Q(\d+))\b/g },
  Problem: { what: "problems", re: /\b(P(\d+))\b/g },
  Task: { what: "tasks", re: /\b(T(\d+))\b/g },
  Docs: null,
  Dropped: null,
};

function checkSize(path) {
  const bytes = statSync(path).size;
  if (bytes > MAX_BYTES) {
    note(
      `${path}  is ${Math.round(bytes / 1024)} KB, over the ${MAX_BYTES / 1024} KB cap — ` +
        `split the entry, or sweep closed entries out of the live register`,
    );
  }
}

/** Entries in a live register are `**P12 — title**` at the start of a line. */
function liveEntries(path, prefix) {
  if (!existsSync(path)) return [];
  const re = new RegExp(`^\\*\\*(${prefix}(\\d+)) —`);
  return readFileSync(path, "utf8")
    .split("\n")
    .flatMap((line, i) => {
      const m = re.exec(line);
      return m ? [{ id: m[1], n: Number(m[2]), file: path, line: i + 1 }] : [];
    });
}

/** One entry per file; the filename carries the number and the H1 must agree. */
const cited = [];

function historyEntries(dir, filePrefix, prefix, { numbered = true, outcome = null } = {}) {
  const path = join(historyDir, dir);
  if (!existsSync(path)) return [];
  const nameRe = new RegExp(`^${filePrefix}-(\\d{4})\\.md$`);
  const found = [];
  for (const name of readdirSync(path).sort()) {
    if (name === "README.md") continue;
    const file = join(path, name);
    if (!name.endsWith(".md")) {
      note(`${file}  is not a markdown file — this directory holds one entry per file`);
      continue;
    }
    const m = nameRe.exec(name);
    if (!m) {
      note(`${file}  is misnamed — expected ${filePrefix}-NNNN.md (four digits)`);
      continue;
    }
    checkSize(file);
    if (!numbered) continue;

    const n = Number(m[1]);
    const text = readFileSync(file, "utf8");
    const heads = text.split("\n").flatMap((l, i) => {
      const h = new RegExp(`^# (${prefix}(\\d+)) — `).exec(l);
      return h ? [{ id: h[1], n: Number(h[2]), line: i + 1 }] : [];
    });
    if (heads.length === 0) {
      note(`${file}:1  no entry heading — expected "# ${prefix}${n} — <title>"`);
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
        `${file}:${heads[0].line}  is named for ${prefix}${n} but its heading says ` +
          `${heads[0].id} — the filename is the index, so the two must agree`,
      );
      continue;
    }
    if (outcome) {
      const body = text.split("\n");
      const at = body.findIndex((l) => /^## Outcome\s*$/.test(l));
      if (at === -1) {
        note(
          `${file}  has no "## Outcome" section — it must say where this went: ` +
            outcome.map((k) => `**${k}:**`).join(", "),
        );
      } else {
        const rest = body.slice(at + 1);
        const stop = rest.findIndex((l) => /^## /.test(l));
        const lines = stop === -1 ? rest : rest.slice(0, stop);
        let declared = 0;
        for (const line of lines) {
          const m = /^- \*\*(\w+):\*\*(.*)$/.exec(line);
          if (!m) continue;
          const [, kind, tail] = m;
          if (!outcome.includes(kind)) {
            note(
              `${file}:${at + 1}  Outcome declares "${kind}", which is not one of ` +
                outcome.join(", "),
            );
            continue;
          }
          declared++;
          // A dropped entry cites nothing, so the reason IS the record. An
          // empty one is indistinguishable from never having triaged it.
          if (kind === "Dropped" && tail.trim().length < 12) {
            note(
              `${file}:${at + 1}  Dropped without a reason — say why it was not real, ` +
                `or the entry reads as untriaged`,
            );
          }
          // EVERY number on the line, not just the first: "T11, T12 and T13" is
          // the normal shape for a ruling that creates several staged items, and
          // a check that validated only T11 would be quietly worthless there.
          const kindSpec = OUTCOME_KINDS[kind];
          if (kindSpec) {
            for (const c of line.matchAll(kindSpec.re)) {
              cited.push({
                what: kindSpec.what,
                id: c[1],
                n: Number(c[2]),
                file,
                line: at + 1,
              });
            }
          }
        }
        if (declared === 0) {
          note(
            `${file}:${at + 1}  the Outcome section declares nothing — expected at least ` +
              `one of ` + outcome.map((k) => `"- **${k}:**"`).join(", "),
          );
        }
      }
    }
    found.push({ id: heads[0].id, n, file, line: heads[0].line });
  }
  return found;
}

for (const reg of REGISTERS) {
  const livePath = join(root, reg.live);
  if (!existsSync(livePath)) {
    note(`missing register: ${livePath}`);
    reg.openEntries = [];
    reg.closedEntries = [];
    continue;
  }
  checkSize(livePath);
  const open = liveEntries(livePath, reg.prefix);
  const closed = historyEntries(reg.dir, reg.filePrefix, reg.prefix, {
    outcome: reg.outcome ?? null,
  });

  const seen = new Map();
  for (const e of [...open, ...closed]) {
    const at = `${e.file}:${e.line}`;
    if (seen.has(e.n)) note(`${e.id} appears twice (${seen.get(e.n)} and ${at})`);
    else seen.set(e.n, at);
  }
  const total = open.length + closed.length;
  for (let n = 1; n <= total; n++) {
    if (!seen.has(n)) {
      note(
        `${reg.prefix}${n} is missing from the ${reg.what} register — ` +
          `numbering must be dense (append, never renumber)`,
      );
    }
  }

  reg.openEntries = open;
  reg.closedEntries = closed;
}

// ── A cited problem or task must exist ──────────────────────────────────────
for (const c of cited) {
  const reg = REGISTERS.find((r) => r.what === c.what);
  const known = new Set([...reg.openEntries, ...reg.closedEntries].map((e) => e.n));
  if (!known.has(c.n)) {
    note(
      `${c.file}:${c.line}  the Outcome cites ${c.id}, which is not in the ${c.what} ` +
        `register — a declared consequence that does not exist is the failure this ` +
        `section was added to catch`,
    );
  }
}

// ── The one headline allowed to state totals ────────────────────────────────
const WORDS = [
  "zero","one","two","three","four","five","six","seven","eight","nine","ten",
  "eleven","twelve","thirteen","fourteen","fifteen","sixteen","seventeen",
  "eighteen","nineteen","twenty",
];
const num = (s) => (/^\d+$/.test(s) ? Number(s) : WORDS.indexOf(s.toLowerCase()));

const registerPath = join(root, "PROBLEMS.md");
const HEADLINE =
  /^\*\*Status (\d{4}-\d{2}-\d{2}): (\d+) opened, (\d+) fixed — ([\w-]+) open\.\*\*/;

if (existsSync(registerPath)) {
  const reg = REGISTERS.find((r) => r.what === "problems");
  const lines = readFileSync(registerPath, "utf8").split("\n");
  const found = lines
    .map((l, i) => ({ m: HEADLINE.exec(l), line: i + 1 }))
    .filter((x) => x.m);

  if (found.length === 0) {
    note(
      `no status headline in ${registerPath} — expected ` +
        `**Status YYYY-MM-DD: N opened, M fixed — K open.**`,
    );
  } else if (found.length > 1) {
    note(
      `${found.length} status headlines (lines ${found.map((f) => f.line).join(", ")}) — ` +
        `the register states its status once and rewrites it in place; git holds the history`,
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
          // A doc explaining this very rule has to be able to quote the shape it
          // forbids. Backticked spans are quotation, not assertion.
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
