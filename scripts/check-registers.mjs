#!/usr/bin/env node
// PROVENANCE: descended from the predecessor project's check-register-counts.mjs
// (../programming_game_planned). The incidents cited below happened THERE. They
// are kept because they are the evidence that justifies the check — not because
// they happened in this repo.
//
//   node scripts/check-registers.mjs .        # the REPO root, not docs/
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
import { stripCode } from "./lib/markdown.mjs";
import {
  CITATION_CLAIMS,
  HASH_MARKER,
  HISTORY_BANNER,
  MAX_BYTES,
  NUMBER_WORDS,
  OUTCOME_KINDS,
  REGISTERS,
  TOTAL_CLAIMS,
} from "./lib/registers.mjs";

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

/** Is this file a closed record? history/ is exempt from several live rules. */
const inHistory = (file) => {
  const rel = relative(repoRoot, file).split(sep);
  return rel[0] === "docs" && rel[1] === "history";
};

const cited = [];
// Derived per-run state lives here, NOT on the objects exported by
// lib/registers.mjs. That module is documented as the authoritative description
// of the scheme and is now shared with check-vocabulary; a consumer writing scan
// results into it means reading it no longer tells you an entry's shape, and two
// checks sharing a process would see each other's data.
const scanned = new Map();
const entriesOf = (what) => scanned.get(what) ?? { open: [], closed: [] };

// A backticked short hash. Only counted when it contains a digit, which excludes
// all-letter words that happen to be valid hex ("defaced") while keeping every
// realistic hash — the same rule the existence scan below applies, shared so the
// two cannot disagree about what a commit citation looks like.
const HASH = /`([0-9a-f]{7,40})`/g;
const citesCommit = (text) => [...text.matchAll(HASH)].some((m) => /\d/.test(m[1]));

/**
 * Every file under docs/history/ opens with the closed-record banner.
 *
 * A reader arriving mid-file — from a grep, from a link — needs to know at the
 * top that they are reading history rather than spec; CLAUDE.md calls that the
 * load-bearing convention of the split. It was held by imitation across sixteen
 * files, written down nowhere, and deleting it from one passed clean. That is
 * the same shape check-doc-layout exists to prevent for split-doc part files,
 * where seven of the predecessor's 62 had quietly inverted their breadcrumb.
 */
function checkBanner(file, text = readFileSync(file, "utf8")) {
  const first = text.split("\n")[0];
  if (first !== HISTORY_BANNER) {
    note(
      `${file}:1  does not open with the closed-record banner — line 1 must read ` +
        `exactly ${HISTORY_BANNER}`,
    );
  }
}

// ── Entries ─────────────────────────────────────────────────────────────────
const PREFIXES = REGISTERS.map((r) => r.prefix).join("");

/**
 * Every register-entry opener in a live doc — `**P12 — title**` at the start of
 * a line — WHICHEVER register's prefix it carries, and whatever dash it used.
 *
 * Three widenings close the same fail-open hole. The pattern demanded an em dash, so
 * `**P1 - a real open problem**` matched nothing: the entry was invisible to the
 * density rule, to the open/closed uniqueness rule, and to the derived headline,
 * which went on validating "zero open" against a file that visibly had one. And
 * each doc was scanned only for its own prefix, so `**Q99 — …**` filed into
 * TASKS.md was not a misfiling but a non-event — with QUESTIONS.md still
 * claiming to be the only place an open question may live. And it anchored at
 * column 0, so `- **P1 — reopened, quietly**` was not an entry either: the live
 * register visibly listed an open P1 that history said was fixed, and the
 * open/closed rule saw nothing. A list marker or an indent is how a person
 * writes a line into a list, not a way to opt out of the register.
 *
 * LEAD is every decoration a person can put in front of the bold text while
 * still writing the line into the document as an entry. Only the BULLET markers
 * were tolerated, so three more spellings opted out silently: `1. **T7 — …**`,
 * `> **T7 — …**` and a first table cell `| **T7 — …** | … |` were each
 * uncounted, undeduped and undensity-checked, while the derived headline went on
 * validating its count against a file that visibly listed the entry. Those last
 * two are the worse half in the OTHER direction as well: in a doc that is not a
 * register, `> **Q6 — a second, contradictory statement**` is precisely the
 * "restated in two places" failure the scheme is built around, and the citation
 * scan stays quiet about it because the number still resolves.
 *
 * The INDENT stays unbounded here on purpose, and the four-space case is handled
 * one layer down instead. `    **P1 — four-space indent**` is an indented code
 * block — it renders as code, which is why a writer reaches for it — and it used
 * to declare a live entry that no reader sees as one. Refusing it here by
 * capping the indent would have been wrong in the other direction: four spaces
 * under a list marker is ordinary continuation text, renders as an entry, and
 * would have stopped counting as one. So the question "is this line code?" is
 * answered by stripCode() in lib/markdown.mjs, which every check shares and
 * which follows what markdown renders, and this pattern answers only "is this
 * line an entry?".
 */
const LEAD = `(?:[-*+]\\s+|\\d+[.)]\\s+|>\\s*|\\|\\s*)*`;
const OPENER = new RegExp(`^\\s*${LEAD}\\*\\*([${PREFIXES}])(\\d+)\\s*([—–-])`);

function openers(path) {
  if (!existsSync(path)) return [];
  return stripCode(readFileSync(path, "utf8")).flatMap((line, i) => {
    const m = OPENER.exec(line);
    return m
      ? [{ prefix: m[1], id: `${m[1]}${m[2]}`, n: Number(m[2]), dash: m[3], file: path, line: i + 1 }]
      : [];
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
 *
 * A THIRD hole shipped after that: the start pattern demanded the colon inside
 * the bold and exactly one space after the list marker, so a near miss —
 * `- **Task**: T4242`, or `-   **Task:** …` — matched nothing, got absorbed by
 * the continuation loop as prose belonging to the bullet above, and had its
 * kind unvalidated and its citations unresolved. It needed only one well-formed
 * bullet above it, and CLAUDE.md says two bullets is normal. The worst shape was
 * `- **Dropped**: …` inside an answered question, which is exactly the
 * inbox-only rule this section exists to enforce. Both spellings are now read as
 * the same kind: they render alike, so accepting one and silently ignoring the
 * other is the failure, not the leniency.
 */
const BULLET = /^\s*[-*+]\s+(.*)$/;
const KIND = /^\*\*(\w+):\*\*(.*)$/; // canonical: colon inside the bold
const KIND_LOOSE = /^\*\*(\w+)\*\*\s*:(.*)$/; // colon just outside it
// `- **Dropped** — turned out not to matter` renders the same and is this
// corpus's own punctuation everywhere else (`**P1 — title**`), so it was the
// fourth way to have a bullet's kind absorbed as prose and never validated —
// with the inbox-only Dropped smuggled into a ruling as the worst case, which is
// exactly what the two patterns above were added for.
const KIND_DASH = /^\*\*(\w+)\*\*\s*[—–-]\s*(.*)$/;

/** The kind a bullet declares, or null for an ordinary prose bullet. */
function bulletKind(line) {
  const b = BULLET.exec(line);
  if (!b) return null;
  return KIND.exec(b[1]) ?? KIND_LOOSE.exec(b[1]) ?? KIND_DASH.exec(b[1]);
}

function parseBullets(lines, firstLineNo) {
  const out = [];
  for (let i = 0; i < lines.length; i++) {
    const k = bulletKind(lines[i]);
    if (!k) continue;
    let text = k[2];
    let j = i + 1;
    // A nested PROSE bullet is still continuation of this one — only another
    // kind-bearing bullet ends it.
    //
    // And a BLANK LINE does not end it either: CommonMark keeps an indented
    // paragraph inside the same list item, and it renders as part of that
    // bullet. Stopping at the blank was the fourth instance of this function's
    // recurring hole — a citation absorbed into something the parser does not
    // scan, so its kind went unvalidated and its numbers unresolved.
    while (j < lines.length) {
      const line = lines[j];
      if (line.trim() === "") {
        const next = lines[j + 1];
        const continues =
          next !== undefined && /^\s{2,}\S/.test(next) && !bulletKind(next) && !/^#/.test(next);
        if (!continues) break;
        text += " " + next.trim();
        j += 2;
        continue;
      }
      if (bulletKind(line) || /^#/.test(line)) break;
      text += " " + line.trim();
      j++;
    }
    out.push({ kind: k[1], text, line: firstLineNo + i });
    i = j - 1;
  }
  return out;
}

function checkOutcome(file, text, allowed) {
  const body = stripCode(text);
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
    if (name.startsWith(".")) continue;
    // The directory README carries the banner too, and nothing else here
    // applies to it.
    if (name === "README.md") {
      checkBanner(join(path, name));
      continue;
    }
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
    checkBanner(file, text);
    const heads = stripCode(text).flatMap((l, i) => {
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
    // "Fixing one records the commit that closed it" (CLAUDE.md) — stated in
    // three docs and enforced nowhere, so an entry with no hash at all passed
    // clean. The scan below validates hashes that ARE present; only the branch
    // that fires had ever been exercised. What is required here is presence: a
    // fixed problem and a completed task each close in code, and this file is
    // the only record of which commit did it.
    if (reg.closingCommit && !citesCommit(text)) {
      note(
        `${file}  records no commit — a closed ${reg.what} entry names the commit that ` +
          `closed it (CLAUDE.md, "The four registers")`,
      );
    }
    found.push({ id: heads[0].id, n, file, line: heads[0].line });
  }
  return found;
}

// An entry opener in a doc that is not a register at all — `**Q6 — a second,
// contradictory statement**` in 00-overview — was invisible to everything: the
// number resolves, so the citation scan says nothing, and openers() only ever
// read the four live registers. That is the "a question restated in two places
// gets answered in one of them" failure the scheme is built around, and every
// doc T8 and T9 will write was unguarded. Same widening the ⚠HASH scan got.
{
  const liveRegisters = new Set(REGISTERS.map((r) => join(root, r.live)));
  for (const path of allDocs) {
    if (inHistory(path) || liveRegisters.has(path)) continue;
    for (const e of openers(path)) {
      const owner = REGISTERS.find((r) => r.prefix === e.prefix);
      note(
        `${e.file}:${e.line}  ${e.id} opens a register entry in a doc that is not a ` +
          `register — entries live in docs/${owner.live}, and a restated one gets ` +
          `answered in the other place`,
      );
    }
  }
}

for (const reg of REGISTERS) {
  const livePath = join(root, reg.live);
  if (!existsSync(livePath)) {
    note(`missing register: ${livePath}`);
    scanned.set(reg.what, { open: [], closed: [] });
    continue;
  }
  const here = openers(livePath);
  const open = here.filter((e) => e.prefix === reg.prefix);
  for (const e of here) {
    if (e.prefix !== reg.prefix) {
      const owner = REGISTERS.find((r) => r.prefix === e.prefix);
      note(
        `${e.file}:${e.line}  ${e.id} opens an entry of the ${owner.what} register here — ` +
          `it belongs in docs/${owner.live}, which is the only place ${owner.what} live`,
      );
    } else if (e.dash !== "—") {
      note(
        `${e.file}:${e.line}  ${e.id} opens with "${e.dash}" — an entry reads ` +
          `**${e.id} — title** with an em dash, and every register rule keys on that line`,
      );
    }
  }
  const closed = historyEntries(reg);

  const seen = new Map();
  for (const e of [...open, ...closed]) {
    const at = `${e.file}:${e.line}`;
    // NOT added to `seen`. It used to be, and that is the same phantom the
    // comment below records for the open-and-closed-at-once case, one entry
    // apart: a live `**P0 — …**` was reported correctly as numbered 0, and then
    // counted, so the dense range grew to 1 and the run ALSO said "P1 is
    // missing". Following that instruction creates a P1 beside a P0 — inventing
    // an entry the append-only rule forbids, on the strength of an entry the
    // line above just rejected. Zero is not a position in the numbering, so it
    // takes no position in the range.
    if (e.n === 0) {
      note(`${at}  is numbered 0 — registers number from 1 (${reg.prefix}1…)`);
      continue;
    }
    if (seen.has(e.n)) note(`${e.id} appears twice (${seen.get(e.n)} and ${at})`);
    else seen.set(e.n, at);
  }
  // Dense from 1. Note the honest limit: the upper bound is derived from the
  // entry count, so deleting the HIGHEST-numbered entry shrinks the range and
  // is undetectable here. Gaps below the top are caught.
  //
  // From the DISTINCT numbers, not from the two array lengths: a number that is
  // open and closed at once counted twice, so the range grew by one and the run
  // reported a phantom "Q16 is missing" alongside the real "Q6 appears twice" —
  // an instruction that, followed, invents an entry the append-only rule forbids.
  const total = seen.size;
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

// ── docs/history/ holds one directory per register, and nothing else ────────
// historyEntries only ever reads join(historyDir, reg.dir), so a loose file
// dropped straight into docs/history/ escaped the banner rule, the naming rule
// and the one-entry-per-file rule at once — which is the exact shape of the
// batched shards this scheme replaced (`questions-answered-001.md`,
// `tasks-completed.md`). One could come back tomorrow and nothing would notice.
{
  const known = new Set(REGISTERS.map((r) => r.dir));
  if (existsSync(historyDir)) {
    for (const entry of readdirSync(historyDir, { withFileTypes: true })) {
      if (entry.name.startsWith(".")) continue;
      const path = join(historyDir, entry.name);
      if (entry.isDirectory()) {
        if (!known.has(entry.name)) {
          note(
            `${path}  is not one of the register directories (${[...known].join(", ")}) — ` +
              `history holds one per register and nothing else`,
          );
        }
        continue;
      }
      if (entry.name === "README.md") continue;
      note(
        `${path}  is a loose file in docs/history/ — a closed entry lives in its ` +
          `register's directory, one entry per file, named for its number`,
      );
    }
  }
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

// ── Inline citations resolve, and "open — Q12" means open ───────────────────
// CLAUDE.md sanctions the inline form in any doc, and design-invariant DI9 calls
// it the case to hunt: a ruling moves to history and the citation keeps
// resolving while meaning the wrong thing. Only `## Outcome` citations were
// checked, which is the smaller half of the corpus's citations by a wide margin.
//
// Backticked spans are quotation, not citation — the same rule the totals and
// marker scans use, and what lets a doc show `Q<n>` or an invented example.
{
  const CITE = new RegExp(`(?<![\\w])([${PREFIXES}])(\\d+)\\b`, "g");
  // The claim patterns themselves are in lib/registers.mjs, one entry per SHAPE
  // with an id on it, because check-checks.mjs demands a seeded defect per
  // pattern: its per-check guard is satisfied by any one of sixty register
  // mutations, and rules shipped with none at all behind that.
  const openClaims = CITATION_CLAIMS.filter((c) => c.claims === "open");
  const closedClaims = CITATION_CLAIMS.filter((c) => c.claims === "closed");
  const asserts = (claims, before, after) =>
    claims.some((c) => c.re.test(c.side === "before" ? before : after));
  for (const file of allDocs) {
    // Per PARAGRAPH, like the totals and marker scans: a backticked example
    // number that wraps across a line break is still quotation, and this corpus
    // wraps at about 80 columns and writes `Q73` and `P29` as examples.
    for (const p of paragraphs(stripCode(readFileSync(file, "utf8")))) {
      const text = p.text.replace(/`[^`]*`/g, "");
      for (const m of text.matchAll(CITE)) {
        const reg = REGISTERS.find((r) => r.prefix === m[1]);
        const { open, closed } = entriesOf(reg.what);
        const n = Number(m[2]);
        if (![...open, ...closed].some((e) => e.n === n)) {
          note(
            `${file}:${p.line}  cites ${m[0]}, which is not in the ${reg.what} register — ` +
              `neither open nor closed`,
          );
          continue;
        }
        // history/ is exempt from both claims, for the same reason it is exempt
        // from the totals rule: a closed record describes what was true on its
        // date, it is append-only, and "fixing" it would mean editing a file the
        // scheme forbids editing.
        if (inHistory(file)) continue;
        const before = text.slice(0, m.index);
        const after = text.slice(m.index + m[0].length);
        const isOpen = open.some((e) => e.n === n);
        if (!isOpen && asserts(openClaims, before, after)) {
          note(
            `${file}:${p.line}  calls ${m[0]} open, but it is closed — the citation still ` +
              `resolves while meaning the opposite`,
          );
        }
        if (isOpen && asserts(closedClaims, before, after)) {
          note(
            `${file}:${p.line}  calls ${m[0]} answered, but it is open in ${reg.live} — ` +
              `a ruling that was never made is the worst thing a Decided section can carry`,
          );
        }
      }
    }
  }
}

// ── A cited commit must exist ───────────────────────────────────────────────
// Only inside history/, only backticked, 7-40 hex chars, and only if it contains
// a digit — which excludes all-letter words that happen to be valid hex
// ("defaced") while keeping every realistic short hash.
{
  let gitWorks = true;
  const seen = new Map();
  for (const file of allDocs) {
    // inHistory(), not a second, looser spelling of it. `includes("/history/")`
    // matches a `history` segment ANYWHERE, so a future spikes/foo/history/notes.md
    // would have its backticked hex validated as commit hashes here while every
    // other rule in this file treated it as a live doc — one definition of which
    // files are ours, which is the doctrine md-files.mjs exists for.
    if (!inHistory(file)) continue;
    for (const m of readFileSync(file, "utf8").matchAll(HASH)) {
      if (!/\d/.test(m[1])) continue;
      if (!seen.has(m[1])) seen.set(m[1], file);
    }
  }
  // Everything git can fail for that is NOT "this hash rotted" is established
  // ONCE, up front. The catch below used to special-case only ENOENT and report
  // every other failure as rot — so "not a git repository" and a `--depth 1`
  // clone both accused the author of a history rewrite, with git's own stderr
  // discarded. A wrong diagnosis in a check that names a specific culprit costs
  // more than no diagnosis.
  const git = (...args) => {
    try {
      return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    } catch (err) {
      return err.code === "ENOENT" ? null : "";
    }
  };
  const inRepo = git("rev-parse", "--git-dir");
  if (inRepo === null) {
    note("git not found — cannot verify the commit hashes cited in history/");
    gitWorks = false;
  } else if (inRepo === "") {
    note(
      "not a git repository — cannot verify the commit hashes cited in history/ " +
        "(this is a limit of where the check is running, not a defect in the corpus)",
    );
    gitWorks = false;
  } else if (git("rev-parse", "--is-shallow-repository") === "true" && seen.size > 0) {
    note(
      "shallow clone — cannot verify the commit hashes cited in history/; " +
        "fetch with depth 0 before trusting this check",
    );
    gitWorks = false;
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
    } catch {
      note(
        `${file}  cites \`${hash}\`, which is not a commit in this repository — ` +
          `a history rewrite invalidates every hash recorded before it`,
      );
    }
  }
}

// ── One status block per register, rewritten in place ───────────────────────
const WORDS = NUMBER_WORDS;
const STATUS = /^\*\*Status (\d{4}-\d{2}-\d{2})/;
const COUNTED = /^\*\*Status (\d{4}-\d{2}-\d{2}): (\d+) opened, (\d+) fixed — ([\w-]+) open\.\*\*/;

for (const reg of REGISTERS.filter((r) => r.status)) {
  const path = join(root, reg.live);
  if (!existsSync(path)) continue;
  const lines = stripCode(readFileSync(path, "utf8"));
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

// ── The ⚠HASH marker belongs to one register ────────────────────────────────
// It marks a task whose PR regenerates the golden fixture, and a reviewer greps
// one file for it. Used in a second register it stops being a way in: either the
// definition stays narrow and the other register's usage contradicts it, or it
// widens to cover both and marks nearly every open question, since almost
// everything changes sim behavior while the sim is unbuilt. Both states shipped.
//
// Backticked mentions are quotation, not marking — the docs that describe this
// rule have to be able to name the marker, exactly as with the totals rule below.
//
// LIVE REGISTERS ONLY, deliberately. docs/history/ is a closed record and is
// allowed to narrate the marker: question-answered-0013.md's Outcome says three
// tasks "each become a staged, ⚠HASH implementation item", which is correct
// prose about TASKS.md and would be a false positive here. The marker's job is
// to be greppable in the open work; a closed ruling is not open work.
{
  const owner = REGISTERS.find((r) => r.what === HASH_MARKER.what);
  const ownerPath = join(root, owner.live);
  // EVERY live doc, not just the other registers: CLAUDE.md says the marker
  // "marks nothing else", and the loop covered three files out of a corpus that
  // will hold docs/01–06. A ⚠HASH in a numbered doc defeats the one property the
  // marker has — that a reviewer greps one file for it.
  // EVERY live doc means every one, including README.md and anything future at
  // the repo root — the `docs/` restriction that used to sit here contradicted
  // the sentence above it. The files that DEFINE the marker are not exempted by
  // path; they are exempt because they backtick it, which is the same
  // quotation-not-assertion rule the totals and citation scans use.
  for (const path of allDocs.filter((f) => !inHistory(f) && f !== ownerPath)) {
    // Per PARAGRAPH, for the same reason the totals scan below is: this corpus
    // wraps at about 80 columns and markdown lets a code span cross a newline,
    // so a line-at-a-time scan rejected a quotation of the marker that happened
    // to wrap. The two scans in one file disagreeing about what a code span is
    // is the drift this shares one helper to avoid.
    for (const p of paragraphs(stripCode(readFileSync(path, "utf8")))) {
      if (p.text.replace(/`[^`]*`/g, "").includes(HASH_MARKER.text)) {
        note(
          `${path}:${p.line}  uses ${HASH_MARKER.text}, which marks a task in ${owner.live} and ` +
            `nothing else — say the property in words here instead`,
        );
      }
    }
  }
}

// ── Nobody else states the totals ───────────────────────────────────────────
// The shapes live in lib/registers.mjs as TOTAL_CLAIMS, one per id, so that
// check-checks.mjs can require a mutation for each. Two of them matched digits
// only while three siblings beside them were built on NUMBER_WORDS — and
// PROBLEMS.md's own headline reads "zero open", so the word spelling of the very
// sentence the rule protects was the one that escaped.
const registerPath = join(root, "PROBLEMS.md");

/** Blank-line-separated blocks, joined to one string each, tagged with the line they open on. */
function paragraphs(lines) {
  const out = [];
  let buf = [];
  let start = 0;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === "") {
      if (buf.length) out.push({ line: start, text: buf.join(" ") });
      buf = [];
      continue;
    }
    if (buf.length === 0) start = i + 1;
    buf.push(lines[i].trim());
  }
  if (buf.length) out.push({ line: start, text: buf.join(" ") });
  return out;
}

// One traversal, through the shared walker that already skips .git/node_modules/
// target — rather than a third hand-rolled one. `md-files.mjs` exists to be "one
// definition of which files are ours".
{
  for (const file of allDocs) {
    const bytes = statSync(file).size;
    if (bytes > MAX_BYTES) {
      note(
        `${file}  is ${Math.ceil(bytes / 1024)} KB, over the ${MAX_BYTES / 1024} KB cap — ` +
          `split it (CLAUDE.md, "Splitting a doc"), or sweep closed entries into history`,
      );
    }
    // history/ is exempt from the totals rule: a closed record of what was true
    // on its date is not a claim about today.
    if (inHistory(file)) continue;
    // Per PARAGRAPH, not per line. This corpus wraps at about 80 columns, so
    // "Eight questions are open." was caught and the identical claim split
    // across two lines was not — the rule CLAUDE.md spends the most words on,
    // defeated by a line break. Joining a paragraph before matching also fixes
    // code spans that wrap, since markdown lets those cross a newline too.
    for (const p of paragraphs(stripCode(readFileSync(file, "utf8")))) {
      // PROBLEMS.md's derived headline is the one total the corpus states, and
      // it is recomputed against the entries above. Only that paragraph is
      // exempt: the whole FILE used to be, which made the document most likely
      // to restate a total the one place any total passed — including question
      // and task totals, which have no derived source there at all.
      if (file === registerPath && STATUS.test(p.text)) continue;
      // A doc explaining this very rule has to be able to quote the shape it
      // forbids. Backticked spans are quotation, not assertion.
      const said = p.text.replace(/`[^`]*`/g, "");
      if (TOTAL_CLAIMS.some((t) => t.re.test(said))) {
        note(
          `${file}:${p.line}  restates a register total — counts live in PROBLEMS.md only; ` +
            `cite entries, not totals (the claim may wrap; this is the paragraph it opens)`,
        );
      }
    }
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
