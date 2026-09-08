#!/usr/bin/env node
// The checks, checked.
//
//   node scripts/check-checks.mjs .
//
// WHY THIS EXISTS — and it is the most load-bearing check in the repo.
//
// Every review round of this corpus has found defects, and the damaging ones
// were almost never wrong prose; they were checks that PASSED WHILE VALIDATING
// NOTHING:
//
//   * Outcome citations were scanned on the marker line only, so every citation
//     on a wrapped line went unchecked — twice, in two different revisions,
//     each shipping with a comment claiming the hole was closed.
//   * Fenced code blocks read as register entries, so a doc quoting the format
//     failed, and the failure cascaded into a false density error.
//   * Every check reported success on zero inputs.
//   * The language spike scored four identically-failing child processes as
//     DETERMINISTIC.
//
// Every one was found by hand-mutating a corpus copy and watching what the
// checks said. The round where that step was skipped is the round three of them
// shipped. So the step is no longer manual.
//
// Each mutation below is a defect that once passed CI, or one that a check
// claims to catch. The suite asserts two things: the unmutated corpus passes
// every check, and every mutation is caught by the named check with a message
// that names the actual problem.
//
// ADDING A CHECK MEANS ADDING MUTATIONS HERE. A check with no mutation is a
// check nobody has ever seen fail.

import { cpSync, existsSync, mkdtempSync, rmSync, writeFileSync, appendFileSync, readFileSync, readdirSync, renameSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
// Seeded from the walker's skip list, because the fixture's value is being
// identical to what the other checks see. They are NOT the same rule, though,
// and the distinction matters if either grows: there, SKIP means "contains no
// markdown we own"; here it means "omit from the fixture entirely". A directory
// added there for the first reason would stop being copied here — and a doc
// linking to a NON-markdown file underneath it would then fail in the fixture
// and nowhere else. Split the lists the moment that happens.
import { SKIP as WALKER_SKIP } from "./lib/md-files.mjs";

// Not extended here. The editor directories belong in the walker's list, where
// both this fixture and the real run see the same tree: skipping `.idea/` here
// alone meant a stray `.idea/notes.md` failed the real check-registers and
// passed the baseline, which is the one disagreement this fixture may not have.
const SKIP = new Set(WALKER_SKIP);
import { spawnSync } from "node:child_process";
// The member parser the `lints` check itself uses, for the manifest-only
// fixture below. Importing it rather than re-deriving the member list is the
// whole point: two parsers for one format is what made a commented-out member
// kill this run with a raw ENOENT.
import { workspaceMembers } from "./lib/workspace-lints.mjs";
import { CITATION_CLAIMS, HISTORY_BANNER, TOTAL_CLAIMS } from "./lib/registers.mjs";

const repo = process.argv[2] ?? ".";

// Each check, and how it is invoked against a corpus root. Every entry returns
// {code, out} so an in-process check sits on the same driver as a spawned one:
// the workspace-lints check was originally bolted on beside this table and
// reimplemented fixture creation, mutation-failure handling, exit checking and
// cleanup — getting each slightly wrong, and skipping `expect` entirely.
const script = (name) => {
  const run = (dir) => {
    const r = spawnSync("node", [`scripts/${name}`, dir], { encoding: "utf8", cwd: repo });
    return { code: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
  };
  // Recorded so the table can be checked against scripts/ below, rather than
  // trusted to have been updated.
  run.script = name;
  return run;
};

const CHECKS = {
  links: script("check-links.mjs"),
  registers: script("check-registers.mjs"),
  layout: Object.assign(
    (dir) => {
      const r = spawnSync("node", ["scripts/check-doc-layout.mjs", join(dir, "docs")], {
        encoding: "utf8",
        cwd: repo,
      });
      return { code: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
    },
    { script: "check-doc-layout.mjs" },
  ),
  structure: script("check-structure.mjs"),
  vocabulary: script("check-vocabulary.mjs"),
  mermaid: script("check-mermaid.mjs"),
  // Spawned, not called in-process, so this table exercises the same entry
  // point CI and the pre-commit hook run — exit code, message and all. It was
  // an in-process call to a copy of the rule that lived in this file, which is
  // how it ended up with mutations but no CI step.
  lints: script("check-workspace-lints.mjs"),
};

/** Rewrite a file, failing loudly if the edit matched nothing. */
function edit(path, from, to) {
  const before = readFileSync(path, "utf8");
  const after = before.replace(from, to);
  if (after === before) {
    throw new Error(`mutation matched nothing in ${path} — the fixture drifted from the mutation`);
  }
  writeFileSync(path, after);
}

/** Write a closed-record file, banner and all — every file under history/ has one. */
const closedFile = (dir, name, body) =>
  writeFileSync(join(dir, name), `${HISTORY_BANNER}\n\n${body}`);

/**
 * A mutation that seeds ONE named pattern from lib/registers.mjs.
 *
 * `probe` is the exact text appended, and the coverage guard at the bottom of
 * this file matches it against the whole pattern list rather than trusting the
 * tag. A mutation tagged for a rule that a SIBLING rule also catches proves
 * nothing about the rule it names: delete that rule and the suite stays green,
 * which is how the untested patterns got in.
 */
const totalClaim = (name, id, probe, file = "docs/TASKS.md") => ({
  name,
  check: "registers",
  expect: "restates a register total",
  rule: `totals:${id}`,
  probe,
  mutate: (d) => appendFileSync(join(d, file), `\n${probe}\n`),
});

/**
 * The same, for a claim ABOUT a cited entry. The seeded sentence is
 * `${before}${cite}${after}`, and the two halves are the probe, because that is
 * exactly how check-registers splits a paragraph around the citation it found.
 */
const citationClaim = ({ name, id, cite, before = "", after = "", expect, file = "docs/00-overview.md" }) => ({
  name,
  check: "registers",
  expect,
  rule: `citation:${id}`,
  probe: { before, after },
  mutate: (d) => appendFileSync(join(d, file), `\n${before}${cite}${after}\n`),
});

const qa = (d) => join(d, "docs/history/questions-answered");
const tc = (d) => join(d, "docs/history/tasks-completed");
const pf = (d) => join(d, "docs/history/problems-fixed");
const it = (d) => join(d, "docs/history/inbox-triaged");

// The claim cases below need one question that is OPEN and one that is CLOSED,
// and both were hard-coded — Q14 open, Q13 closed. The day Q14 was answered,
// "Q14 has been answered" became a true sentence, the register check rightly
// passed it, and this suite went red on a correct ruling. A fixture that breaks
// on every ruling is a fixture nobody keeps, so both are read from the corpus
// under test: the first open entry in QUESTIONS.md, the lowest-numbered file in
// questions-answered/.
const openQuestion = () => {
  const m = readFileSync(join(repo, "docs/QUESTIONS.md"), "utf8").match(/^\*\*(Q\d+) [—–-] /m);
  if (!m) throw new Error("check-checks: docs/QUESTIONS.md has no open entry to seed a claim against");
  return m[1];
};
const closedQuestion = () => {
  const m = readdirSync(qa(repo)).sort()
    .map((n) => n.match(/^question-answered-0*(\d+)\.md$/)).find(Boolean);
  if (!m) throw new Error("check-checks: questions-answered/ has no entry to seed a claim against");
  return `Q${m[1]}`;
};
const OPEN_Q = openQuestion();
const CLOSED_Q = closedQuestion();

// Same defect, other register: three cases seeded a closed `I1` into the inbox
// history on the assumption that the inbox was empty, which it was — until the
// first observation was filed and the seed collided with it as "open and closed
// at once", a different message than the case was asking about. Numbering is
// dense, so the only number a seed can safely use is the next one.
const nextInbox = () => {
  const open = [...readFileSync(join(repo, "docs/INBOX.md"), "utf8").matchAll(/^\*\*I(\d+) [—–-] /gm)]
    .map((m) => Number(m[1]));
  const closed = readdirSync(it(repo))
    .map((n) => n.match(/^inbox-triaged-0*(\d+)\.md$/)).filter(Boolean).map((m) => Number(m[1]));
  return Math.max(0, ...open, ...closed) + 1;
};
const NEXT_I = nextInbox();

// And the tasks register: the open-and-closed-at-once case seeded a closed T7
// while T7 was open, which is right until T7 closes and the seed overwrites the
// real file with a well-formed one. The first open task is the one to seed.
const openTask = () => {
  const m = readFileSync(join(repo, "docs/TASKS.md"), "utf8").match(/^\*\*T(\d+) [—–-] /m);
  if (!m) throw new Error("check-checks: docs/TASKS.md has no open entry to seed a collision against");
  return Number(m[1]);
};
const OPEN_T = openTask();

// And the problems register, whose cases hard-coded "0 opened, 0 fixed", a
// seeded P1 and a seeded P2 against a register that was empty — right until the
// first real P1 was filed, when ten cases went red on a correct commit. Every
// number below is read from the corpus: the open ids in file order (which is
// the order the checker lists them), the fixed count, and the next free number.
const problems = (() => {
  const live = readFileSync(join(repo, "docs/PROBLEMS.md"), "utf8");
  const open = [...live.matchAll(/^\*\*P(\d+) [—–-] /gm)].map((m) => Number(m[1]));
  const closed = readdirSync(pf(repo))
    .map((n) => n.match(/^problem-fixed-0*(\d+)\.md$/)).filter(Boolean).map((m) => Number(m[1]));
  const next = Math.max(0, ...open, ...closed) + 1;
  const ids = (...extra) => [...open, ...extra].map((n) => `P${n}`).join(", ");
  const file = (n) => `problem-fixed-${String(n).padStart(4, "0")}.md`;
  // the headline's counted form, as check-registers reads it
  const headline = /: (\d+) opened, (\d+) fixed — ([\w-]+) open\.\*\*/;
  return { open, closed, next, ids, file, headline };
})();
const OPEN_T_FILE = `task-completed-${String(OPEN_T).padStart(4, "0")}.md`;
const NEXT_I_FILE = `inbox-triaged-${String(NEXT_I).padStart(4, "0")}.md`;

const MUTATIONS = [
  // ── registers: entry identity ─────────────────────────────────────────────
  { name: "history file named for one entry, headed as another", check: "registers",
    expect: "the filename is the index",
    mutate: (d) => closedFile(tc(d), "task-completed-0009.md", "# T99 — mislabelled\n") },
  { name: "two entries smuggled into one file", check: "registers",
    expect: "one entry per file",
    mutate: (d) => closedFile(tc(d), "task-completed-0009.md", "# T9 — one\n\n# T10 — two\n") },
  { name: "entry numbered zero", check: "registers",
    expect: "registers number from 1",
    mutate: (d) => closedFile(tc(d), "task-completed-0000.md", "# T0 — zero\n") },
  { name: "misnamed entry file (three digits)", check: "registers",
    expect: "is misnamed",
    mutate: (d) => closedFile(tc(d), "task-completed-009.md", "# T9 — short name\n") },
  { name: "a loose file dropped straight into docs/history/", check: "registers",
    expect: "loose file in docs/history/",
    // Nothing walked docs/history/ itself, so this escaped the banner rule, the
    // naming rule and one-entry-per-file at once — the shape of the batched
    // shards this scheme replaced.
    mutate: (d) => writeFileSync(join(d, "docs/history/questions-answered-001.md"),
      "# Q1–Q6\n\nthe old shard scheme, back from the dead\n") },
  { name: "an unexpected directory under docs/history/", check: "registers",
    expect: "is not one of the register directories",
    mutate: (d) => mkdirSync(join(d, "docs/history/decisions"), { recursive: true }) },
  { name: "a renamed history directory", check: "registers",
    expect: "does not exist",
    mutate: (d) => renameSync(join(d, "docs/history/problems-fixed"), join(d, "docs/history/problems-fixd")) },

  // ── registers: the Outcome contract ───────────────────────────────────────
  // These pin "the Outcome cites T4242", not the looser "not in the tasks
  // register" tail: the inline-citation scan reports the same dangling number
  // with a different message, so the loose string let an Outcome-parser
  // regression pass on the strength of an unrelated rule catching it.
  { name: "answered question with no Outcome section", check: "registers",
    expect: 'has no "## Outcome" section',
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md", "# Q6 — no outcome\n\nbody\n") },
  { name: "Outcome citing a number that does not exist", check: "registers",
    expect: "the Outcome cites T4242",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — dangling\n\n## Outcome\n\n- **Task:** T4242 which is not real.\n") },
  { name: "Outcome citation on a wrapped continuation line", check: "registers",
    expect: "the Outcome cites T4242",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — wrapped\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md) is real, and so is\n  T4242 which is not.\n") },
  { name: "Outcome citation in a nested bullet", check: "registers",
    expect: "the Outcome cites T4242",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — nested\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md)\n  - **Task:** T4242 nested\n") },
  { name: "a nested bullet's KIND is validated, not just its citation",
    check: "registers", expect: "which is not one of",
    // The citation case above survives an anchor regression by accident: swallow
    // the nested line as prose and the parent bullet's text still contains
    // "T4242", so the same "not in the tasks register" message fires and the
    // suite stays green. Kind validation does NOT survive it — and that is the
    // path by which the inbox-only Dropped gets smuggled into a ruling. Pinning
    // it needs a forbidden kind and a different expected message.
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — nested kind\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "  - **Dropped:** smuggled in under a well-formed bullet.\n") },
  { name: "an Outcome bullet with the colon outside the bold", check: "registers",
    expect: "the Outcome cites T4242",
    // `- **Task**: …` renders the same and used to match nothing, so the
    // continuation loop absorbed it as prose belonging to the bullet above and
    // its kind went unvalidated and its citation unresolved.
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — loose colon\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "- **Task**: T4242 which is not real.\n") },
  { name: "an Outcome bullet with extra space after the list marker",
    check: "registers", expect: "the Outcome cites T4242",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — wide marker\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "-   **Task:** T4242 which is not real.\n") },
  { name: "the inbox-only Dropped, spelled with the colon outside the bold",
    check: "registers", expect: "which is not one of",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — loose dropped\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "- **Dropped**: turned out not to matter.\n") },
  { name: "a nested PROSE bullet is still continuation, not a new bullet",
    check: "registers", expect: "", skipIfClean: true,
    // The inverse of the four above: tightening the parser must not start
    // reading an ordinary explanatory sub-bullet as a malformed declaration.
    // Written into the INBOX register at its next free number — a
    // question-answered file for an open question collides as open-and-closed
    // at once and fails for an unrelated reason, which is not what this case
    // is asking.
    mutate: (d) => closedFile(it(d), NEXT_I_FILE,
      `# I${NEXT_I} — a triaged note\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md)\n` +
        "  - and some nested prose explaining it\n") },
  { name: "an Outcome citation in an indented paragraph after a blank line",
    check: "registers", expect: "the Outcome cites T4242",
    // CommonMark keeps an indented paragraph inside the same list item, so this
    // renders as part of the bullet above — and the parser stopped at the blank
    // line, which is the fourth variant of "absorbed into something nothing
    // scans" its own docstring catalogues.
    mutate: (d) => closedFile(it(d), NEXT_I_FILE,
      `# I${NEXT_I} — a triaged note\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md)\n\n` +
        "  Also T4242, which is not a real task.\n") },
  { name: "an Outcome bullet written with an em dash", check: "registers",
    expect: "which is not one of",
    // The corpus's own punctuation, rendering identically — so the kind went
    // unvalidated and the inbox-only Dropped rode into a ruling, which is the
    // worst case the two earlier spellings were added to stop.
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — dash kind\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "- **Dropped** — turned out not to matter after all.\n") },
  { name: "an Outcome section that declares nothing", check: "registers",
    expect: "declares nothing",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — empty outcome\n\n## Outcome\n\nNothing to declare, apparently.\n") },
  { name: "a Dropped bullet with no reason", check: "registers",
    expect: "Dropped without a reason",
    mutate: (d) => closedFile(it(d), NEXT_I_FILE,
      `# I${NEXT_I} — a triaged note\n\n## Outcome\n\n- **Dropped:** no.\n`) },
  { name: "a non-markdown file in a register's history directory", check: "registers",
    expect: "is not a markdown file",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-0007.txt"), "notes\n") },
  { name: "a directory README missing the closed-record banner", check: "registers",
    expect: "closed-record banner",
    mutate: (d) => {
      const f = join(tc(d), "README.md");
      writeFileSync(f, readFileSync(f, "utf8").split("\n").slice(1).join("\n"));
    } },
  { name: "a history file citing an object that is not a commit", check: "registers",
    expect: "not a commit",
    // The scan resolves every backticked hash; only the "does not exist" branch
    // had ever been fired. A tree is a real object, so this pins the kind test
    // rather than the existence test.
    mutate: (d) => {
      const tree = spawnSync("git", ["rev-parse", "HEAD^{tree}"], { cwd: repo, encoding: "utf8" });
      appendFileSync(join(tc(d), "task-completed-0001.md"), `\nAlso \`${tree.stdout.trim()}\`.\n`);
    } },
  { name: "bare Docs bullet linking nothing", check: "registers",
    expect: "links nothing",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — empty docs\n\n## Outcome\n\n- **Docs:**\n") },
  { name: "a ruling using the inbox-only Dropped outcome", check: "registers",
    expect: "which is not one of",
    mutate: (d) => closedFile(qa(d), "question-answered-0006.md",
      "# Q6 — dropped\n\n## Outcome\n\n- **Dropped:** decided it did not matter after all.\n") },

  // ── registers: totals, size, commits ──────────────────────────────────────
  totalClaim("a restated total in CLAUDE.md (outside docs/)", "count-noun-verb",
    "Eight questions are open.", "CLAUDE.md"),
  totalClaim("a restated total with the adverb AFTER the verb", "count-noun-verb",
    "Three tasks are still open."),
  // The shape the pattern's adverb slot exists for. The case above sits ENTIRELY
  // inside the base pattern ("three tasks are"), so deleting the slot left the
  // suite green — the mutation and the comment justifying the slot made the same
  // mistake about where an adverb lands.
  totalClaim("a restated total with the adverb BETWEEN noun and verb", "count-noun-verb",
    "Three tasks still remain open."),
  // PROBLEMS.md's own derived headline says "zero open", and this list could not
  // spell it.
  totalClaim("a restated total worded with zero", "count-noun-verb",
    "Zero questions are open."),
  totalClaim("a restated total with the adjective in front of the noun", "count-adjective-noun",
    "There are 8 open questions right now."),
  totalClaim("a restated total with the label first", "label-first",
    "Open: 8 questions, 2 problems."),
  // THE REGISTER HEADLINE'S OWN SHAPE, which had no mutation at all — the
  // per-check guard was satisfied by any one of the sixty register mutations, so
  // nothing noticed. It matched digits only the whole time, sitting next to three
  // siblings built on NUMBER_WORDS, and the word spelling is the one PROBLEMS.md
  // itself writes.
  totalClaim("the register headline's own shape, restated elsewhere", "opened-fixed",
    "74 opened, 65 fixed."),
  totalClaim("the register headline's own shape, worded", "opened-fixed",
    "Zero opened, zero fixed."),
  // Also unseeded, also digit-only, and also unable to spell the commonest verb
  // for it: a register `has` a count as readily as it `holds` one.
  totalClaim("a total worded as what the register holds", "register-holds",
    "The problem register holds nine entries."),
  totalClaim("a total worded as what the register has", "register-holds",
    "The questions register has 9 open."),
  { name: "a restated total WRAPPED across two lines", check: "registers",
    expect: "restates a register total",
    // The corpus wraps at about 80 columns, so a per-line scan let the rule
    // CLAUDE.md spends the most words on be defeated by a line break.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nEight questions\nare open.\n") },
  { name: "a restated total inside PROBLEMS.md itself", check: "registers",
    expect: "restates a register total",
    // The whole file was exempt, not just its derived headline — so the document
    // most likely to restate a total was the one place any total passed, and
    // question and task totals have no derived source there at all.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"), "\nEight questions are open.\n") },
  { name: "a design doc over the size cap", check: "registers",
    expect: "over the 40 KB cap",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"), "x".repeat(45000)) },
  { name: "a doc ONE BYTE over the size cap", check: "registers",
    expect: "is 41 KB, over the 40 KB cap",
    // The message rounded down, so 40,961 bytes read as "is 40 KB, over the
    // 40 KB cap" — a diagnostic contradicting itself, and the only seeded case
    // was 45 KB, which no rounding error falls between.
    mutate: (d) => writeFileSync(join(d, "docs/edge.md"), "x".repeat(40 * 1024 + 1)) },
  { name: "a doc of exactly the size cap", check: "registers",
    expect: "", skipIfClean: true,
    mutate: (d) => writeFileSync(join(d, "docs/edge.md"), "x".repeat(40 * 1024)) },
  { name: "a cited commit that is not in this repository", check: "registers",
    expect: "not a commit in this repository",
    mutate: (d) => appendFileSync(join(tc(d), "task-completed-0001.md"), "\nAlso `0bad1ce`.\n") },
  { name: "a `history` directory that is not docs/history/", check: "registers",
    expect: "", skipIfClean: true,
    // The commit-hash scan used a second, looser definition of "is this a closed
    // record" than the inHistory() every other rule in the file uses: any path
    // with a `history` segment anywhere counted. So this file's backticked hex
    // would have been validated as a commit and failed CI, while the totals rule,
    // the ⚠HASH rule and the open/closed claims all treated the same file as a
    // live doc. One definition of which files are ours.
    mutate: (d) => {
      mkdirSync(join(d, "spikes/lang-determinism/history"), { recursive: true });
      writeFileSync(join(d, "spikes/lang-determinism/history/notes.md"),
        "# Bench notes\n\nThe run before this one hashed to `0bad1ce`, which is not a commit.\n");
    } },
  { name: "stacked status blocks", check: "registers",
    expect: "states its status once",
    mutate: (d) => appendFileSync(join(d, "docs/QUESTIONS.md"), "\n**Status 2020-01-01.** stale\n") },
  { name: "a register with no status block at all", check: "registers",
    expect: "has no status block",
    // The other branch of the same rule, and it had no mutation: the per-check
    // coverage guard was satisfied by the stacked case above it. A register that
    // states its status nowhere is the commoner accident of the two — a status
    // block is deleted in a rewrite and simply not written back.
    mutate: (d) => edit(join(d, "docs/QUESTIONS.md"), /^\*\*Status \d{4}-\d{2}-\d{2}[^\n]*\n/m, "") },

  // ── structure ─────────────────────────────────────────────────────────────
  { name: "the same heading twice in one file (a splice)", check: "structure",
    expect: "duplicate heading",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"), "\n## Decided\n\nspliced\n") },
  { name: "the same heading twice at DIFFERENT levels", check: "structure",
    expect: "duplicate heading",
    // The key was "level + text", so a splice that re-indented one of the two
    // copies stopped being a duplicate. GitHub slugs `## M0 — Scaffolding` and
    // `### M0 — Scaffolding` to the same anchor, so the two are not even
    // distinguishable to a link — and the rule's own stated purpose, that the two
    // copies disagree, has nothing to do with how deep either one sits.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\n### M0 — Scaffolding\n\nspliced\n") },
  { name: "a table separator that lost its header", check: "structure",
    expect: "no header row above it",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n---|---|\n| a | b |\n") },
  { name: "a two-dash separator is still a table", check: "structure",
    expect: "columns, header has",
    // GFM wants one or more dashes per cell; demanding three meant a table
    // written `|--|--|` was not a table at all — no header check, no column
    // check, and not counted in the tick.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n| a | b |\n|--|--|\n| x | y | z |\n") },
  { name: "an alignment separator is still a table", check: "structure",
    expect: "no header row above it",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n|:-:|:-:|\n| x | y |\n") },
  { name: "a table row with the wrong column count", check: "structure",
    expect: "columns, header has",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n| a | b |\n|---|---|\n| x | y | z |\n") },
  { name: "a table that lost its SEPARATOR instead of its header", check: "structure",
    expect: "no |---|---| separator among them",
    // The mirror of the case above it, and the half nothing looked for: every
    // table rule was reached through isTableSeparator, so a table with no
    // separator was not a malformed table but no table at all — unchecked,
    // unreported, and not counted in the "N tables well formed" tick, while it
    // renders on GitHub as literal pipes.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n| a | b |\n| x | y |\n") },
  { name: "a sentence containing a pipe, directly under a table, is not a row",
    check: "structure", expect: "", skipIfClean: true,
    // The body-row loop broke on a line with NO pipe in it, so an ordinary
    // sentence that happens to contain one — with no blank line above it — was
    // column-counted and reported as a broken table row. The separator run thirty
    // lines up requires a LEADING pipe and says why; this loop did not, and the
    // complaint it produced named the one line in the block that is not a row.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n| a | b |\n|---|---|\n| x | y |\nWrite `a | b | c` when a union of three is meant.\n") },
  { name: "a single stray pipe row is not a truncated table", check: "structure",
    expect: "", skipIfClean: true,
    // GFM needs a header AND a separator before anything is a table, so one line
    // is not two-thirds of one. Flagging it would fire on ordinary prose.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n| a | b |\n") },
  { name: "a separator-less table quoted in an indented code block stays inert",
    check: "structure", expect: "", skipIfClean: true,
    // The indented half of "this is code, not content", from the table side: the
    // fenced spelling of this case has been inert since fences were consolidated,
    // and writing the same example the four-space way used to fail CI.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nA broken table reads:\n\n    | a | b |\n    | x | y |\n") },

  // ── vocabulary ────────────────────────────────────────────────────────────
  { name: "a history README dropping an Outcome kind the checker accepts", check: "vocabulary",
    expect: "does not document",
    mutate: (d) => {
      const f = join(d, "docs/history/inbox-triaged/README.md");
      writeFileSync(f, readFileSync(f, "utf8").replace(/\*\*Dropped:\*\*/g, "Dropped"));
    } },
  { name: "a history README offering a kind its register rejects", check: "vocabulary",
    expect: "which the checker rejects",
    mutate: (d) => appendFileSync(join(d, "docs/history/questions-answered/README.md"),
      "\n- `- **Dropped:**` — sure, why not.\n") },
  { name: "INBOX.md dropping an Outcome kind its own register accepts",
    check: "vocabulary", expect: 'does not name the "Dropped:" outcome',
    // The file that explains triage described the outcomes in prose, so a writer
    // following it produced a file CI rejects — and INBOX.md is one of the five
    // this checker's header names as having drifted, and the one nothing opened.
    mutate: (d) => {
      const f = join(d, "docs/INBOX.md");
      writeFileSync(f, readFileSync(f, "utf8").replace(/\*\*Dropped:\*\*/g, "Dropped"));
    } },
  { name: "README miscounting the registers", check: "vocabulary",
    // Not the bare word "registers": that is also the tail of lib/registers.mjs,
    // so any stack trace out of that module satisfied it.
    expect: 'registers"; there are',
    mutate: (d) => {
      const f = join(d, "README.md");
      writeFileSync(f, readFileSync(f, "utf8").replace(/four registers/g, "three registers"));
    } },

  // ── links, layout, mermaid ────────────────────────────────────────────────
  { name: "a link to a file that does not exist", check: "links",
    expect: "missing file",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n[nope](does-not-exist.md)\n") },
  { name: "a reference-style link, which this check cannot resolve", check: "links",
    expect: "reference-style link",
    // It left the "N relative links resolve" count unmoved, so a broken
    // reference link was invisible to both the run and the reader of its tick.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the doc][ref].\n\n[ref]: does-not-exist.md\n") },
  { name: "an angle-bracket link destination, which this check cannot resolve",
    check: "links", expect: "angle-bracket link destination",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the doc](<a b.md>).\n") },
  { name: "a link destination with a raw space in it", check: "links",
    expect: "unescaped space",
    // Neither LINK nor the angle-bracket guard can match it, so it was SKIPPED
    // rather than refused: the "N relative links resolve" count sat unmoved while
    // CommonMark rendered the line as literal text. Same class as the two shapes
    // above, and skipping it is the mislabelled-count failure they are refused
    // to avoid.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the doc](a b.md) here.\n") },
  { name: "a link with a quoted title is not a spaced destination", check: "links",
    expect: "", skipIfClean: true,
    // The inverse. A title is the one legal reason for whitespace after the
    // destination, and refusing the shape above must not start refusing it.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the register](PROBLEMS.md \"the problems register\") here.\n") },
  { name: "a link to a directory carries no citation and binds nothing",
    check: "links", expect: "", skipIfClean: true,
    // A directory has no lines. It is the one destination the citation half must
    // still skip once the `.md` filter is gone.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the closed records](history) for context.\n") },
  { name: "a code-span citation in neither supported form", check: "links",
    expect: "unverifiable citation",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\nSee `PROBLEMS.md at :24` for context.\n") },
  { name: "a misspelt file name leaving the PREVIOUS binding standing", check: "links",
    expect: "bare citation names no file",
    // A `path.md` span that resolves to nothing is skipped in silence — prose
    // says `clippy.pedantic` without meaning a path — and it used to be skipped
    // without clearing `bound`. So the `:5` below was resolved against
    // PROBLEMS.md, reported under PROBLEMS.md's path if it failed, and passed
    // silently whenever line 5 of PROBLEMS.md happened to be non-blank: the
    // citation into the misspelt file was never checked at all. Naming a file
    // ends the previous binding whether or not the new name resolves.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"),
      "\nSee [PROBLEMS.md](PROBLEMS.md), and then `PROBLMS.md`, at `:5`.\n") },
  { name: "a part file with an inverted breadcrumb", check: "layout",
    // Not the bare word "breadcrumb", which every layout message contains — this
    // case is specifically about the crumb being present but below the H1.
    expect: "breadcrumb is on line",
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language"), { recursive: true });
      writeFileSync(join(d, "docs/01-language/syntax.md"),
        "# Syntax\n\n*Part of [01-language](../01-language.md).*\n");
    } },
  // ── workspace lints ──────────────────────────────────────────────────────
  { name: "a member that omits the workspace lint opt-in", check: "lints",
    fixture: "manifests", expect: "does not opt into the workspace lints",
    mutate: (d) => edit(join(d, "crates/sim/Cargo.toml"), /^\[lints\]\n[\s\S]*?\n\n/m, "") },
  { name: "a member declaring [lints] but not the opt-in, with `workspace = true` below it",
    check: "lints", fixture: "manifests", expect: "does not opt into the workspace lints",
    // The shape that actually defeats an unanchored regex: the opt-in is gone,
    // but a `workspace = true` appears LATER in the manifest, which is the
    // ordinary way to use a workspace dependency. An earlier mutation claimed to
    // pin this and did not — the real manifest has no such line after [lints],
    // so the old regex caught it too and the section-slicing fix was untested.
    mutate: (d) => edit(join(d, "crates/sim/Cargo.toml"), /^\[lints\]\nworkspace = true\n/m,
      '[lints]\nclippy.pedantic = "warn"\n\n[dependencies]\nserde = { workspace = true }\n') },
  { name: "the root declaring no [workspace.lints.clippy] at all", check: "lints",
    fixture: "manifests", expect: "the deny is declared nowhere",
    mutate: (d) => edit(join(d, "Cargo.toml"), /^\[workspace\.lints\.clippy\]$/m, "[workspace.metadata.unused]") },
  { name: "the deny value deleted while its section header stays", check: "lints",
    fixture: "manifests", expect: "the deny is declared nowhere",
    // The check tested that `[workspace.lints.clippy]` existed, so deleting the
    // one line the determinism rules rest on left it green — while its success
    // message went on naming the deny by name. Clippy stays green either way,
    // which is the only reason this check exists at all.
    mutate: (d) => edit(join(d, "Cargo.toml"), /^arithmetic_side_effects = "deny"$/m, "") },
  { name: "the dotted [workspace.lints] spelling of the same deny", check: "lints",
    fixture: "manifests", expect: "", skipIfClean: true,
    // Valid Cargo, and the header-only test called it "declared nowhere" — a
    // correct manifest failing CI.
    mutate: (d) => edit(join(d, "Cargo.toml"),
      /\[workspace\.lints\.clippy\]\narithmetic_side_effects = "deny"/,
      '[workspace.lints]\nclippy.arithmetic_side_effects = "deny"') },
  { name: "a member glob that matches no crate", check: "lints",
    fixture: "manifests", expect: "matched no crate",
    mutate: (d) => edit(join(d, "Cargo.toml"), /members = \[[^\]]*\]/, 'members = ["crates/nonesuch-*"]') },
  { name: "a member listed but absent from the tree", check: "lints",
    fixture: "manifests", expect: "listed as a workspace member but absent",
    mutate: (d) => edit(join(d, "Cargo.toml"), /members = \[[^\]]*\]/, 'members = ["crates/sim", "crates/ghost"]') },
  { name: "default-members above members must not be read as the member list",
    check: "lints", fixture: "manifests", expect: "does not opt into the workspace lints",
    // If the anchor regresses, `default-members` matches first and the real
    // members list is never validated — so this must still catch sim's missing
    // opt-in.
    mutate: (d) => {
      // The two lists must DIFFER, or reading the wrong one still finds the
      // defect and the anchor goes unpinned — which is what a first attempt at
      // this mutation did.
      edit(join(d, "Cargo.toml"), /^\s*members = /m, 'default-members = []\nmembers = ');
      edit(join(d, "crates/sim/Cargo.toml"), /^\[lints\]\n[\s\S]*?\n\n/m, "");
    } },

  { name: "a `[lints]` header with a trailing comment is still the lints section",
    check: "lints", fixture: "manifests", expect: "", skipIfClean: true,
    // Valid TOML that Cargo accepts, and the header pattern demanded
    // end-of-line after the bracket — so a fully opted-in crate was reported as
    // opting out, and CI failed on a correct manifest. The call site's comment
    // said trailing comments were handled; that was true of the section body
    // and false of the header above it.
    mutate: (d) => edit(join(d, "crates/sim/Cargo.toml"), /^\[lints\]$/m,
      "[lints]  # inherit the workspace deny") },
  { name: "a `[workspace.lints.clippy]` header with a trailing comment still declares the deny",
    check: "lints", fixture: "manifests", expect: "", skipIfClean: true,
    mutate: (d) => edit(join(d, "Cargo.toml"), /^\[workspace\.lints\.clippy\]$/m,
      "[workspace.lints.clippy]  # every member inherits this") },

  { name: "a commented-out workspace member is read as a comment", check: "lints",
    fixture: "manifests", expect: "", skipIfClean: true,
    // Two parsers for one format: lintOptInProblems stripped comments and this
    // file's own expandAll did not, so an ordinary `# "crates/lang",` handed
    // manifests() a path that does not exist and killed the run with a raw
    // ENOENT — mid-table, taking the three mutations after it with it.
    mutate: (d) => edit(join(d, "Cargo.toml"), /^(\s*)members = \[/m,
      '$1members = [\n$1    # "crates/lang",  # not written yet\n$1    ') },

  // ── table cells follow GFM ───────────────────────────────────────────────
  { name: "an UNESCAPED pipe in a code span really does start a new cell", check: "structure",
    expect: "columns, header has",
    // Pins the GFM rule: a table row is split on unescaped pipes before inline
    // parsing. Blanking code spans made this accept a table that renders broken.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n| op | meaning |\n|---|---|\n| `a | b` | bitwise or |\n") },
  { name: "an ESCAPED pipe in a code span stays one cell", check: "structure",
    expect: "", skipIfClean: true,
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n| op | meaning |\n|---|---|\n| `a \\| b` | bitwise or |\n") },

  // ── fences are matched, not counted ──────────────────────────────────────
  { name: "an unterminated fence, which blanks the rest of the file", check: "structure",
    expect: "unterminated",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n```text\nnever closed\n") },
  { name: "a defect hidden behind a ~~~ block that quotes opening a ``` fence",
    check: "structure", expect: "duplicate heading",
    // Three markers, odd parity — no forgotten fence required. Under a bare
    // toggle everything below went blank and check-structure, check-registers
    // and check-links all printed their unchanged ✓ counts.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n~~~text\nTo open one you write ```mermaid and close it the same way.\n~~~\n" +
        "\n## Decided\n\nspliced\n") },
  { name: "a ````markdown block containing ``` stays inert", check: "structure",
    expect: "", skipIfClean: true,
    // The inverse: a longer fence legitimately encloses a shorter one, and its
    // contents must NOT be checked. Under a bare toggle the inner marker closed
    // the block early and the quoted example became live text.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n````markdown\n```mermaid\nflowchart LR\n```\n\n## Decided\n\n|---|---|\n````\n") },

  // ── line citations ───────────────────────────────────────────────────────
  { name: "a citation to line :0", check: "links",
    expect: "line numbers start at 1",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md](PROBLEMS.md) then :0 for context.\n") },
  { name: "a citation with a leading zero, which silently renumbers", check: "links",
    expect: "line numbers start at 1",
    // `Number("012")` is 12, so this used to point twelve lines from where it
    // was written — worse than the crash `:0` caused.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md](PROBLEMS.md) then `:012` as well.\n") },
  { name: "a malformed citation must not abort the scan of later files",
    check: "links", expect: "missing file",
    // `:0` indexed lines[-1] and threw, so every file sorted after the offending
    // one went unchecked. INBOX.md sorts before TASKS.md: the broken link below
    // is only reported if the scan survived the citation above it.
    mutate: (d) => {
      appendFileSync(join(d, "docs/INBOX.md"), "\nSee [PROBLEMS.md](PROBLEMS.md) then :0 here.\n");
      appendFileSync(join(d, "docs/TASKS.md"), "\n[nope](does-not-exist.md)\n");
    } },

  // ── the corpus itself going missing ──────────────────────────────────────
  // The pre-commit hook used to bail out silently when the staged tree held no
  // docs/ — which happens exactly when a commit deletes the corpus. The guard is
  // gone, so these two are what the hook now relies on to speak up.
  { name: "the whole docs/ tree deleted", check: "registers",
    expect: "missing register",
    mutate: (d) => rmSync(join(d, "docs"), { recursive: true, force: true }) },
  { name: "the whole docs/ tree deleted (layout)", check: "layout",
    expect: "no such directory",
    mutate: (d) => rmSync(join(d, "docs"), { recursive: true, force: true }) },

  // ── vocabulary: the READMEs nobody was opening ───────────────────────────
  { name: "problems-fixed README offering an outcome kind its register has none of",
    check: "vocabulary", expect: "has no Outcome section at all",
    mutate: (d) => appendFileSync(join(d, "docs/history/problems-fixed/README.md"),
      "\n- `- **Dropped:**` — sure, why not.\n") },
  { name: "tasks-completed README offering an outcome kind its register has none of",
    check: "vocabulary", expect: "has no Outcome section at all",
    mutate: (d) => appendFileSync(join(d, "docs/history/tasks-completed/README.md"),
      "\n- `- **Dropped:**` — sure, why not.\n") },
  { name: "README miscounting the checks", check: "vocabulary",
    expect: 'checks"; scripts/ holds',
    mutate: (d) => edit(join(d, "README.md"), /eight checks \(/, "four checks (") },
  { name: "a count written in digits, which the pattern accepts", check: "vocabulary",
    expect: "", skipIfClean: true,
    // The alternation takes `\\d+`, and the comparison took only the word form —
    // so "the 8 checks" was rejected with "scripts/ holds 8", telling an author
    // the number was wrong while printing it back at them.
    mutate: (d) => edit(join(d, "README.md"), /the eight checks/, "the 8 checks") },
  { name: "a number-plus-registers sentence elsewhere in README is not the anchor",
    check: "vocabulary", expect: "", skipIfClean: true,
    // The sibling of the case below. This count kept the bare noun after the
    // other one was anchored, so an ordinary sentence mentioning registers was
    // read as the Layout row and reported against it.
    mutate: (d) => edit(join(d, "README.md"), /^## Getting set up$/m,
      "Two registers matter most.\n\n## Getting set up") },
  { name: "the count message names the phrase that would satisfy the check",
    check: "vocabulary", expect: 'expected "eight checks (`check-*.mjs`)"',
    // README saying "the eight checks" was answered with "expected eight
    // checks" — an instruction that leaves the check red when followed, because
    // the regex wants a parenthetical the message never mentioned.
    mutate: (d) => edit(join(d, "README.md"), /the eight checks \(`check-\*\.mjs`\)/,
      "the eight checks") },
  { name: "a number-plus-checks sentence elsewhere in README is not the anchor",
    check: "vocabulary", expect: "", skipIfClean: true,
    // `exec` takes the FIRST match in the file, so an unanchored `(\\d+) checks`
    // would bind to this line instead of the Layout row and report a drift
    // against the wrong sentence.
    mutate: (d) => edit(join(d, "README.md"), /^## Layout$/m,
      "Three checks are worth knowing about before the rest.\n\n## Layout") },
  { name: "history/README dropping the size cap it restates by hand",
    check: "vocabulary", expect: "size cap",
    mutate: (d) => edit(join(d, "docs/history/README.md"), /40 KB/g, "a reasonable size") },
  { name: "CLAUDE.md teaching a breadcrumb check-doc-layout would reject",
    check: "vocabulary", expect: "does not print the breadcrumb",
    // The two really had drifted: CLAUDE.md gave one formula for every part file
    // while the checker builds the crumb from the immediate parent, so a nested
    // part written to CLAUDE.md's letter failed CI with "names the wrong
    // doorway". One string, in lib/doc-layout.mjs, and both sides cite it.
    mutate: (d) => edit(join(d, "CLAUDE.md"),
      /\*Part of \[NN-name\]\(\.\.\/NN-name\.md\)\.\*/, "*Part of NN-name.*") },

  // ── layout: parts nest ───────────────────────────────────────────────────
  { name: "a part file whose breadcrumb is not followed by a blank line and an H1",
    check: "layout", expect: "expected a blank line then the H1",
    // The one part-file rule with no seeded defect.
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language"), { recursive: true });
      writeFileSync(join(d, "docs/01-language/syntax.md"),
        "*Part of [01-language](../01-language.md).*\n# Syntax\n");
    } },
  { name: "an ordinary docs subdirectory is not a parts directory",
    check: "layout", expect: "", skipIfClean: true,
    // Only `NN-name/` is a split doc. Descending into every non-history
    // directory meant the first docs/assets/ holding a .md was told to grow a
    // doorway that has no reason to exist.
    mutate: (d) => {
      mkdirSync(join(d, "docs/assets"), { recursive: true });
      writeFileSync(join(d, "docs/assets/notes.md"), "# Notes\n\nnot a part file\n");
    } },
  { name: "a part file whose breadcrumb names the wrong doorway", check: "layout",
    expect: "names the wrong doorway",
    // The copy-paste a new part file actually makes, and the one shape whose
    // message contradicted itself: "breadcrumb is on line 1, not line 1".
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language"), { recursive: true });
      writeFileSync(join(d, "docs/01-language/syntax.md"),
        "*Part of [99-elsewhere](../99-elsewhere.md).*\n\n# Syntax\n");
    } },
  { name: "a README.md inside a split doc's parts directory", check: "layout",
    expect: "a second index beside the parts",
    // Every .md under docs/NN-name/ was required to be a part file, so this was
    // reported as missing a breadcrumb naming its own parent doorway — an
    // instruction that, followed, produces a file claiming to be part of the
    // directory it sits in. The doorway is already the index, so the right answer
    // is neither a lie nor a pass.
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language"), { recursive: true });
      writeFileSync(join(d, "docs/01-language.md"),
        "# The unit language\n\n| Part | Owns |\n|---|---|\n" +
          "| [syntax](01-language/syntax.md) | the grammar |\n");
      writeFileSync(join(d, "docs/01-language/syntax.md"),
        "*Part of [01-language](../01-language.md).*\n\n# Syntax\n");
      writeFileSync(join(d, "docs/01-language/README.md"), "# Parts of the language doc\n");
    } },
  { name: "a parts directory with no doorway beside it", check: "layout",
    expect: "no doorway beside it",
    // The other half of the split convention, and the half nothing looked for.
    // Every part file here is correct; what is missing is the doorway their
    // breadcrumbs name — the file CLAUDE.md says owns the invariants that cross
    // the parts. It was caught only incidentally, by check-links resolving the
    // crumb's href, so a part naming its doorway in prose was invisible to both.
    // Seeded under a number no real doc will take: this case once used
    // docs/01-language/, and the day that doc was actually written its doorway
    // existed, the seeded defect stopped being one, and the case went red on a
    // correct commit.
    mutate: (d) => {
      mkdirSync(join(d, "docs/99-seeded"), { recursive: true });
      writeFileSync(join(d, "docs/99-seeded/part.md"),
        "*Part of [99-seeded](../99-seeded.md).*\n\n# A part\n");
    } },
  { name: "a part file two levels down with no breadcrumb", check: "layout",
    expect: "no breadcrumb",
    // Discovery stopped one level down, so this file was never opened AND never
    // counted — and a doc directory holding only subdirectories reported "no
    // split docs yet", which was affirmatively false.
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language/runtime"), { recursive: true });
      writeFileSync(join(d, "docs/01-language/runtime/vm.md"), "# VM\n\nno breadcrumb\n");
    } },

  // ── registers: the derived headline is actually recomputed ───────────────
  // The rule CLAUDE.md spends the most words on, and it had no mutation at all.
  // It is also the one the live corpus cannot exercise: PROBLEMS.md reads "0
  // opened, 0 fixed — zero open" against a register holding nothing, so every
  // arm of the comparison is 0 against 0, and the entire recompute could be
  // deleted — or wired to the wrong field — with every check still green. Each
  // case below moves exactly ONE arm, so swapping two of them fails here.
  { name: "the headline's OPENED total, against a register that has entries",
    check: "registers",
    expect: `says ${problems.open.length + problems.closed.length} opened, the register has ${problems.open.length + problems.closed.length + 2}`,
    // opened counts open PLUS closed, which is the arm most easily miswired to
    // one or the other: with one more closed and one more open the three arms
    // move by 2, 1 and 1, so no two of them can be confused.
    mutate: (d) => {
      closedFile(pf(d), problems.file(problems.next), `# P${problems.next} — fixed\n\nbody\n`);
      appendFileSync(join(d, "docs/PROBLEMS.md"), `\n**P${problems.next + 1} — still open**\n\nbody\n`);
    } },
  { name: "the headline's FIXED total", check: "registers",
    expect: `says ${problems.closed.length} fixed, the register has ${problems.closed.length + 1}`,
    mutate: (d) => {
      closedFile(pf(d), problems.file(problems.next), `# P${problems.next} — fixed\n\nbody\n`);
      edit(join(d, "docs/PROBLEMS.md"), problems.headline,
        `: ${problems.open.length + problems.closed.length + 1} opened, ${problems.closed.length} fixed — $3 open.**`);
    } },
  { name: "the headline's OPEN total", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    mutate: (d) => {
      appendFileSync(join(d, "docs/PROBLEMS.md"), `\n**P${problems.next} — still open**\n\nbody\n`);
      edit(join(d, "docs/PROBLEMS.md"), problems.headline,
        `: ${problems.open.length + problems.closed.length + 1} opened, ${problems.closed.length} fixed — $3 open.**`);
    } },
  { name: "a status headline that is not in the counted form at all",
    check: "registers", expect: "malformed status headline",
    // Dated, so it is found as the register's one status block, but carrying no
    // numbers — which is how a headline stops being checked without disappearing.
    mutate: (d) => edit(join(d, "docs/PROBLEMS.md"), problems.headline, ": all totals current.**") },
  { name: "a headline count spelled as a word the checker cannot read",
    check: "registers", expect: "is not a number this checker knows",
    mutate: (d) => edit(join(d, "docs/PROBLEMS.md"), problems.headline,
      `: $1 opened, $2 fixed — twenty-one open.**`) },

  // ── registers: the entry line itself ─────────────────────────────────────
  { name: "a live entry written with a hyphen instead of an em dash", check: "registers",
    expect: 'opens with "-"',
    // Every register rule keys on that line, so a dash typo used to make the
    // entry a non-event: not counted, not deduped, not density-checked, and the
    // derived headline went on validating "zero open" against a file that
    // visibly had one open problem.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"), `\n**P${problems.next} - a real open problem**\n`) },
  { name: "an entry written as a list item", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    // Anchored at column 0, the opener treated a list marker as an opt-out: the
    // live register visibly listed an open P1 and the derived headline went on
    // validating "zero open" against it.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      `\n- **P${problems.next} — reopened, quietly**\n`) },
  { name: "an entry indented under a paragraph", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      `\n  **P${problems.next} — indented into invisibility**\n`) },
  { name: "an entry written as an ORDERED list item", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    // The bullet markers were tolerated and the ordered one was not, so this
    // spelling opted straight out of the register: uncounted, undeduped,
    // undensity-checked, with the derived headline still validating "zero open"
    // against a file that visibly listed it. Which marker a writer reaches for
    // must not decide whether the line is an entry.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      `\n1. **P${problems.next} — reopened inside a numbered list**\n`) },
  { name: "an entry written inside a blockquote", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      `\n> **P${problems.next} — reopened inside a blockquote**\n`) },
  { name: "an entry written as the first cell of a table row", check: "registers",
    expect: `the register has ${problems.open.length + 1} (${problems.ids(problems.next)})`,
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      `\n| a | b |\n|---|---|\n| **P${problems.next} — reopened in a table** | still open |\n`) },
  { name: "a register entry opened in a BLOCKQUOTE in a doc that is not a register",
    check: "registers", expect: "in a doc that is not a register",
    // The worse direction of the same widening. The number resolves, so the
    // citation scan stays silent, and this is the "a question restated in two
    // places gets answered in one of them" failure written as a contradiction.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n> **Q6 — a second, contradictory statement of the tick question**\n") },
  { name: "a LIVE entry numbered zero", check: "registers",
    expect: "is numbered 0", reject: "is missing from",
    // The history spelling of this is seeded above and takes a different path.
    // Here the entry reached the density map, so the range grew to 1 and the run
    // ALSO reported "P1 is missing" — an instruction that, followed, invents a
    // P1 beside a P0, which is what `reject` is for. Same phantom the
    // open-and-closed-at-once case carries, one entry apart.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      "\n**P0 — numbered from zero**\n") },
  { name: "the entry format quoted in a four-space indented block",
    check: "registers", expect: "", skipIfClean: true,
    // Indented code is code. Only the fenced spelling was blanked, so a doc
    // showing the format this way declared a live P1 that no reader sees as one —
    // and once the entry rule stopped reading it, the citation scan picked the
    // same text up as a dangling number instead. lib/markdown.mjs owns the rule
    // for both spellings now.
    mutate: (d) => appendFileSync(join(d, "docs/PROBLEMS.md"),
      "\nAn entry is written like this:\n\n    **P1 — a worked example**\n") },
  { name: "a register entry opened inside a numbered design doc", check: "registers",
    expect: "in a doc that is not a register",
    // The number resolves, so the citation scan says nothing; openers() read only
    // the four live registers, so a question restated in 00-overview was a
    // non-event — and every doc T8 and T9 will write was unguarded.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n**Q6 — a second, contradictory statement of the tick question**\n") },
  { name: "an entry filed into the wrong register's live doc", check: "registers",
    expect: "belongs in docs/QUESTIONS.md",
    // Each doc was scanned only for its own prefix, so an open question living
    // in TASKS.md was invisible rather than misfiled — with QUESTIONS.md still
    // claiming to be the only place an open question may be.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"),
      "\n**Q99 — an open question hiding in the tasks register**\n") },

  // ── registers: inline citations, the corpus's dominant form ──────────────
  { name: "an inline citation to a number that does not exist", check: "registers",
    expect: "neither open nor closed",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"), "\nSee Q99 for the rest.\n") },
  // The shape design-invariant DI9 calls the case to hunt: the number keeps
  // resolving while the ruling has moved to history and means the opposite.
  citationClaim({ name: "an inline citation calling an answered question open",
    id: "open-before", cite: CLOSED_Q, before: "That boundary is still open — ", after: ".",
    expect: "but it is closed" }),
  // "open — Q12" is the phrasing CLAUDE.md sanctions; "…, which is still open" is
  // the one the corpus writes, in two history files about a question that has not
  // closed yet. Matching only the first ordering meant the check went green on
  // the day it was written for.
  citationClaim({ name: "an inline citation calling an answered question open, the other way round",
    id: "open-after", cite: CLOSED_Q, after: " is still open as far as this doc is concerned.",
    expect: `calls ${CLOSED_Q} open` }),
  // The article was the hole: `is still open` was inside the pattern and `is an
  // open question` — the commonest spelling in prose — was not, because the
  // pattern wanted the word adjacent to the copula. The sibling totals pattern
  // one screen away had grown an adjective-before-noun variant for the identical
  // shape.
  citationClaim({ name: "an answered question called an open question",
    id: "open-after", cite: CLOSED_Q, after: " is an open question.",
    expect: `calls ${CLOSED_Q} open` }),
  // And the other half of that hole: `open` is not the only word for open.
  citationClaim({ name: "an answered question described as undecided",
    id: "open-after", cite: CLOSED_Q, after: " remains undecided.",
    expect: `calls ${CLOSED_Q} open` }),
  // A negated closure is a claim of openness, and it fell between both patterns
  // at once: the closed one wants the participle adjacent to the copula, and the
  // open one knew only the word `open`. Its filler now refuses a negation, so
  // this cannot be reported as its own opposite.
  citationClaim({ name: "an answered question described as not yet answered",
    id: "open-after-negated", cite: CLOSED_Q, after: " is not yet answered.",
    expect: `calls ${CLOSED_Q} open` }),
  { name: "a closed record describing what was open on its date", check: "registers",
    expect: "", skipIfClean: true,
    // history/ is append-only and is expected to contradict current design, so
    // the open/closed claim is not applied there — the same exemption the totals
    // rule has, and for the same reason.
    mutate: (d) => appendFileSync(join(qa(d), "question-answered-0003.md"),
      "\nAt the time, the language was still open — Q5.\n") },
  { name: "a backticked example number that WRAPS is still quotation",
    check: "registers", expect: "", skipIfClean: true,
    // The citation scan read code spans per line while the two scans beside it
    // read paragraphs, so one rewrap turned legal text red.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\nA ruling would read `Q73\nand friends` in the scheme.\n") },
  // The copula list stopped at `is|are|was|were`, and this is the phrasing the
  // corpus itself writes ("which has since been ruled", in the spike README). So
  // the sentence the case below calls the worst thing a Decided section can carry
  // passed clean, in the tense a writer summarising a ruling reaches for.
  citationClaim({ name: "an open question described as answered in the PERFECT tense",
    id: "closed-after", cite: OPEN_Q, after: " has been answered, so its doc can be written.",
    expect: `calls ${OPEN_Q} answered` }),
  // `resolved` and `fixed` were both absent from the participle list, and `fixed`
  // is the word the PROBLEMS register uses for its own closed state — so the
  // register with the most to lose from a false closure claim was the one that
  // could not express one.
  citationClaim({ name: "an open question described as resolved",
    id: "closed-after", cite: OPEN_Q, after: " was resolved during the language spike.",
    expect: `calls ${OPEN_Q} answered` }),
  // THE ADVERB SLOT, in the direction this check calls the worse one. Between the
  // copula and the participle the pattern allowed two hard-coded words, `already`
  // and `since`, so `is now answered` and `has finally been ruled` both went green
  // against an open question — while the totals pattern it was modelled on
  // carried a general slot for exactly this.
  citationClaim({ name: "an open question described as answered, with an adverb between",
    id: "closed-after", cite: OPEN_Q, after: " is now answered.",
    expect: `calls ${OPEN_Q} answered` }),
  citationClaim({ name: "the same, with the adverb inside the perfect tense",
    id: "closed-after", cite: OPEN_Q, after: " has finally been ruled.",
    expect: `calls ${OPEN_Q} answered` }),
  // A negated OPEN state asserts a closure as surely as `is answered` does, and
  // it is the mirror of the open-after-negated case above.
  citationClaim({ name: "an open question described as no longer open",
    id: "closed-after-negated", cite: OPEN_Q, after: " is no longer open.",
    expect: `calls ${OPEN_Q} answered` }),
  citationClaim({ name: "an answered question described as STAYING open",
    id: "open-after", cite: CLOSED_Q, after: " stays open as far as this doc is concerned.",
    expect: `calls ${CLOSED_Q} open` }),
  // The worse direction, and the one that had no rule: QUESTIONS.md is the only
  // place an undecided thing may live, so a Decided section asserting a ruling
  // nobody made is the stalest text in the most authoritative-looking place —
  // which is what the register scheme exists to prevent.
  citationClaim({ name: "an open question described as answered",
    id: "closed-before", cite: OPEN_Q, before: "That model was settled in ", after: ".",
    expect: `calls ${OPEN_Q} answered` }),
  { name: "a backticked example number is quotation, not a citation",
    check: "registers", expect: "", skipIfClean: true,
    // The docs invent numbers to show the scheme: `P29`, `Q73`, `I7`. Same
    // convention as the totals and marker scans — backticks are quotation.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\nA ruling would read `Q73` and its file `question-answered-0073.md`.\n") },

  // ── registers: the ⚠HASH marker has one owner ────────────────────────────
  { name: "the ⚠HASH marker used in a register that does not own it",
    check: "registers", expect: "which marks a task in",
    // It really was used in QUESTIONS.md, and the definitions in CLAUDE.md,
    // TASKS.md and design-invariants.md all said "task" while it was.
    mutate: (d) => appendFileSync(join(d, "docs/QUESTIONS.md"),
      "\n⚠HASH. Every option here changes the state hash.\n") },
  { name: "a backticked mention of the marker is quotation, not marking",
    check: "registers", expect: "", skipIfClean: true,
    // The docs that describe the rule have to be able to name the marker.
    mutate: (d) => appendFileSync(join(d, "docs/QUESTIONS.md"),
      "\nThe `⚠HASH` marker belongs to the tasks register.\n") },
  { name: "the ⚠HASH marker in a numbered doc, outside any register",
    check: "registers", expect: "which marks a task in",
    // The rule is "it marks nothing else", and the loop covered the three other
    // registers — not 00-overview, and not the docs/01–06 that T8 will write.
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n⚠HASH — this section changes the state hash.\n") },
  { name: "a backticked mention that WRAPS is still quotation",
    check: "registers", expect: "", skipIfClean: true,
    // This corpus wraps at about 80 columns and markdown lets a code span cross
    // a newline, so a line-at-a-time scan rejected the sentence above one line
    // break later — while the totals scan in the same file joined paragraphs
    // for exactly this reason.
    mutate: (d) => appendFileSync(join(d, "docs/QUESTIONS.md"),
      "\nThe `⚠HASH\nmarker` belongs to the tasks register.\n") },

  // ── registers: numbering ─────────────────────────────────────────────────
  // Both rules are enumerated in check-registers' own header and in CLAUDE.md's
  // register table, and neither had a mutation. The first is the commonest real
  // mistake there is: closing an entry and forgetting to delete it from the live
  // register, which leaves it open and closed at once.
  { name: "an entry that is open and closed at once", check: "registers",
    expect: "appears twice", reject: "is missing from",
    mutate: (d) => closedFile(tc(d), OPEN_T_FILE,
      `# T${OPEN_T} — closed while still listed as open\n`) },
  { name: "a history file missing the closed-record banner", check: "registers",
    expect: "closed-record banner",
    // Sixteen files carried it by imitation, documented nowhere and checked by
    // nothing. It is what tells a reader who arrived mid-file that this is
    // history rather than spec.
    mutate: (d) => {
      const f = join(qa(d), "question-answered-0002.md");
      writeFileSync(f, readFileSync(f, "utf8").split("\n").slice(1).join("\n"));
    } },
  { name: "a completed task that names no commit", check: "registers",
    expect: "records no commit",
    // "Fixing one records the commit that closed it" is stated in CLAUDE.md and
    // in two history READMEs, and was enforced nowhere: the hash scan validated
    // hashes that were present, so only the branch that fires was ever exercised.
    mutate: (d) => closedFile(tc(d), "task-completed-0016.md",
      "# T16 — done, at some point, by someone\n\nNo commit recorded.\n") },
  { name: "a hole in the middle of the numbering", check: "registers",
    expect: "numbering must be dense",
    mutate: (d) => rmSync(join(tc(d), "task-completed-0003.md")) },

  // ── links: the citation half, which the corpus cannot exercise ───────────
  // check-links devotes half its header to line citations and names these two
  // as the rot class it exists for — and the live corpus contains zero
  // citations, so the resolution logic had never once executed and the CI step
  // reported "0 line citations in range" as a success. These four are the only
  // thing that runs it.
  { name: "a citation past the end of the file it names", check: "links",
    expect: "citation past EOF",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md:9999](PROBLEMS.md) for that.\n") },
  { name: "a citation that has slid onto a blank line", check: "links",
    expect: "citation lands on a blank line",
    // Line 2 of any file opening with an H1. P32 in the predecessor was exactly
    // this: a citation that still resolved, at a line that had gone empty.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md:2](PROBLEMS.md) for that.\n") },
  { name: "a bare `:NN` with no file named before it", check: "links",
    expect: "bare citation names no file",
    // Appended below INBOX.md's "## Open" heading, which is a SCOPE_BREAK — so
    // nothing is bound and the number is unresolvable to a reader too.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nAnd the note at :24 explains it.\n") },
  { name: "a citation into a SOURCE file, past its end", check: "links",
    expect: "citation past EOF",
    // The label scan sat below a `.md`-only filter, so a citation into the one
    // non-markdown path this corpus actually links — CLAUDE.md links
    // crates/sim/src/rng.rs — was neither verified nor counted, while the CI step
    // reported "N line citations in range". Which KIND of file a writer cites
    // must not decide whether the number is checked, for the same reason which
    // SHAPE they write it in must not.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [rng.rs:9999](../crates/sim/src/rng.rs) for that.\n") },
  { name: "a backticked `path.rs:NN` citation, past the end of the file",
    check: "links", expect: "citation past EOF",
    // PATH_CITE demanded `.md`, and the unverifiable-citation backstop cannot see
    // this either: its lookbehind excludes a colon preceded by a word character.
    // So the span fell through both, unchecked and uncounted.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee `../crates/sim/src/rng.rs:9999` for that.\n") },
  { name: "a bare `:NN` binds to a source file, not to the markdown before it",
    check: "links", expect: "crates/sim/src/rng.rs",
    // The silent half, and the worst of the three: `bound` was only ever set by a
    // `.md` link, so the number was resolved against a file the writer did not
    // name. Past EOF it is merely reported against the wrong file; in range it
    // would have certified one.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md](PROBLEMS.md) and [rng](../crates/sim/src/rng.rs), at :9999.\n") },
  { name: "a citation written mid-label, not at the end of it", check: "links",
    expect: "citation past EOF",
    // The label scan was anchored to end-of-label and the prose scan's lookbehind
    // excludes a colon preceded by a word character, so this shape was neither
    // checked NOR counted while the step reported "N line citations in range".
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md:9999 for that](PROBLEMS.md) here.\n") },
  { name: "a citation inside a link label is counted once, not twice",
    check: "links", expect: "1 line citations in range", skipIfClean: true,
    // Accepted either way — the number resolves. What the duplicate showed up in
    // was the count, which is why this inverse case asserts the ✓ line.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [the note at :1](PROBLEMS.md) here.\n") },
  { name: "a clock time in a link label is not a line citation", check: "links",
    expect: "", skipIfClean: true,
    // Widening the label scan must not start reading "12:30" as line 30.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nThe [12:30 standup](PROBLEMS.md) notes.\n") },
  { name: "a bare `:NN` that binds to the file named just before it",
    check: "links", expect: "", skipIfClean: true,
    // The inverse: nearest-preceding-citation-wins is the convention the
    // register follows, so it must ACCEPT the shape it is written for. Line 1 is
    // the H1 of every file in the corpus, so this cannot rot into a false alarm.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\nSee [PROBLEMS.md](PROBLEMS.md), and its heading at :1.\n") },

  // ── vocabulary: the fifth file that drifted ──────────────────────────────
  // check-vocabulary's header names design-invariants.md as one of the five
  // originally-drifted files, and it was the one the checker never opened.
  { name: "design-invariants dropping a register's live doc", check: "vocabulary",
    expect: "does not name docs/INBOX.md",
    mutate: (d) => edit(join(d, ".claude/design-invariants.md"), /docs\/INBOX\.md/g, "docs/NOTES.md") },
  { name: "design-invariants dropping a register's entry prefix", check: "vocabulary",
    expect: "`I<n>` entry prefix",
    mutate: (d) => edit(join(d, ".claude/design-invariants.md"), /`I<n>`/g, "`N<n>`") },
  { name: "a doc that defines the ⚠HASH marker losing the marker itself",
    check: "vocabulary", expect: "does not name the ⚠HASH marker",
    // Three docs teach this marker by hand. They have already gone out of step
    // once, in both directions.
    mutate: (d) => edit(join(d, "docs/TASKS.md"), /⚠HASH/g, "HASH-AFFECTING") },

  { name: "a mermaid diagram that does not parse", check: "mermaid",
    expect: "error on line",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n```mermaid\nflowchart LR\n  a --> b\n  ]]] not valid\n```\n") },
  { name: "a broken ```mermaid example quoted inside a ````markdown block stays inert",
    check: "mermaid", expect: "", skipIfClean: true,
    // check-structure has had this exact case since fences were consolidated;
    // check-mermaid kept its own ```-prefix scan and read the quoted example as a
    // live diagram, so writing ABOUT a broken diagram failed CI. Two checks
    // disagreeing about what is content is what one shared matcher prevents.
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"),
      "\n````markdown\n```mermaid\nflowchart LR\n  ]]] not valid\n```\n````\n") },
  { name: "an unterminated ```mermaid block", check: "mermaid",
    expect: "unterminated",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n```mermaid\nflowchart LR\n") },
  { name: "the corpus losing its last mermaid block", check: "mermaid",
    expect: "the check is checking nothing",
    // Zero FILES was guarded and zero BLOCKS was not, so deleting or renaming
    // the one diagram left the step printing a tick over nothing. Every block
    // in every doc is stripped, not the overview's alone: the day a second
    // diagram landed, stripping one left one, and this case went red on a
    // correct commit.
    mutate: (d) => {
      const walk = (dir) => {
        for (const name of readdirSync(dir)) {
          const f = join(dir, name);
          if (name === "node_modules" || name.startsWith(".")) continue;
          if (name.endsWith(".md")) {
            const before = readFileSync(f, "utf8");
            const after = before.replace(/```mermaid[\s\S]*?```\n/g, "");
            if (after !== before) writeFileSync(f, after);
          } else if (!name.includes(".")) walk(f);
        }
      };
      walk(join(d, "docs"));
    } },
];

const run = (check, dir) => CHECKS[check](dir);

// The fixture is the WORKING TREE, minus build and VCS directories.
//
// It used to be `git ls-files` (tracked paths) copied from the working tree
// (current content), and the two disagree in both directions. Delete a tracked
// file without staging and cpSync threw an uncaught ENOENT, killing the run with
// a raw stack trace. Worse, the ordinary way to close a task — remove the entry
// from TASKS.md, add an untracked task-completed-NNNN.md — made the baseline
// block report "registers rejects the unmutated corpus", accusing the checks of
// being broken when the real answer was `git add`.
//
// Copying the working tree makes the fixture identical to what every other
// check in `scripts/ci.sh docs` looks at, so the baseline can only disagree with
// them for a real reason.
//
// WHAT THIS GIVES UP, stated so nobody rediscovers it: the tracked-file version
// saw exactly what CI checks out, so a green suite meant "the same inputs CI
// sees". It no longer does — a file you forgot to `git add` passes here and
// fails on a fresh clone. The pre-commit hook covers that, because it reads the
// index. The trade was made deliberately: a check that fires during ordinary
// work gets bypassed, and then it protects nothing.
function copyTree(src, dst) {
  mkdirSync(dst, { recursive: true });
  let n = 0;
  for (const entry of readdirSync(src, { withFileTypes: true })) {
    if (SKIP.has(entry.name)) continue;
    const from = join(src, entry.name);
    const to = join(dst, entry.name);
    if (entry.isDirectory()) n += copyTree(from, to);
    else if (entry.isFile()) {
      cpSync(from, to);
      n++;
    }
  }
  return n;
}

function manifests() {
  const dir = mkdtempSync(join(tmpdir(), "check-manifests-"));
  cpSync(join(repo, "Cargo.toml"), join(dir, "Cargo.toml"));
  for (const rel of MEMBER_PATHS) {
    // A member listed but absent is a defect the `lints` check REPORTS, so the
    // fixture must be able to hold one. cpSync would instead throw ENOENT out
    // of fixture construction and take the run down.
    if (!existsSync(join(repo, rel, "Cargo.toml"))) continue;
    mkdirSync(join(dir, rel), { recursive: true });
    cpSync(join(repo, rel, "Cargo.toml"), join(dir, rel, "Cargo.toml"));
  }
  return dir;
}

/**
 * Every workspace member path, for the manifest-only fixture. Read ONCE: the
 * member list cannot change between fixtures, and manifests() is built for every
 * `fixture: "manifests"` mutation in the table.
 *
 * From lib/workspace-lints.mjs, the same module the `lints` check runs on the
 * other side of the spawn. This was a second, weaker copy of it — same regex, no
 * `uncomment()` pass — and the two disagreed on the most ordinary edit
 * imaginable: a commented-out member (`# "crates/lang",  # not written yet`).
 * lintOptInProblems read the manifest correctly and saw nothing wrong, while
 * this copy handed manifests() a path that does not exist and killed the whole
 * meta-check with a raw ENOENT stack trace — mid-table, so the three mutations
 * after it never ran and nothing said they had been skipped. Two parsers for one
 * format is the defect; the ENOENT was only how it announced itself.
 */
const MEMBER_PATHS = workspaceMembers(repo).paths;

function fresh() {
  const dir = mkdtempSync(join(tmpdir(), "check-checks-"));
  if (copyTree(resolve(repo), dir) === 0) {
    console.error("✗ check-checks: copied an empty corpus — the suite would test nothing");
    process.exit(2);
  }
  return dir;
}

const failures = [];

// ── The table covers scripts/, and every check in it has a seeded defect ────
// The table is hand-written, so a new check-*.mjs got no baseline run and no
// mutations — and the only signal was an unrelated README mutation failing with
// "the fixture drifted", which names the fixture as the problem. CLAUDE.md says
// adding a check means adding mutations here; this is what makes that a failure
// rather than an omission. Derived the same way check-vocabulary derives
// README's count, so the two cannot disagree about what a check is.
{
  const SELF = "check-checks.mjs";
  const covered = new Set(Object.values(CHECKS).map((run) => run.script));
  const present = readdirSync(join(repo, "scripts")).filter((f) => /^check-.*\.mjs$/.test(f));
  // The shell checks too. The coverage guard below filtered to `.mjs`, so
  // deleting ci.sh's `check-file-size.sh repo` step — and the hook's staged one —
  // left this file fully green, because its size-gate block spawns the script
  // itself and never asks how CI reaches it. That is verbatim the regression the
  // script's own header records: "scripts/ci.sh never called it and neither CI
  // job ran it", for the one failure class this repo calls permanent.
  const shellChecks = readdirSync(join(repo, "scripts")).filter((f) => /^check-.*\.sh$/.test(f));
  for (const file of present) {
    if (file === SELF) continue; // this file drives the table; it is not in it
    if (!covered.has(file)) {
      failures.push(
        `scripts/${file} is not in this file's CHECKS table — it never runs against the ` +
          `corpus here, and nothing can seed a defect for it`,
      );
    }
  }
  // …and scripts/ci.sh must actually run each of them. The table above proves a
  // check is exercised HERE; nothing proved it runs in CI or the hook, and that
  // is not hypothetical — the workspace-lint check shipped with mutations, no
  // step in ci.sh, and no step label, reaching the real manifests only as a side
  // effect of this file's baseline loop.
  //
  // ONE loop over every check, `.mjs` and `.sh`, and THIS FILE IS IN IT. It was
  // excluded from `present` for the CHECKS-table rule, which is correct, and the
  // exclusion then silently carried into the ci.sh rule, which is not: deleting
  // `node scripts/check-checks.mjs .` from ci.sh left the whole suite green while
  // CI stopped running the file CLAUDE.md calls the most load-bearing check in
  // the repo. A check cannot notice its own step going missing unless it looks
  // for it by name, so it looks for it by name.
  //
  // COMMANDS, not the file's text. A bare `includes()` over the whole script is
  // satisfied by a COMMENT, and both files are heavily commented about the very
  // checks they run: ci.sh explains that the workspace-lint check "lived inside
  // check-checks.mjs for a while", and the hook's header names `scripts/ci.sh
  // docs` in prose. So deleting either real invocation left this block green on
  // the strength of the sentence describing it — a check passing because the repo
  // documents the thing it stopped doing.
  const commandsOf = (text) =>
    text
      .split("\n")
      .filter((l) => !/^\s*#/.test(l))
      .join("\n");
  const ciText = commandsOf(readFileSync(join(repo, "scripts/ci.sh"), "utf8"));
  const hookText = commandsOf(readFileSync(join(repo, ".githooks/pre-commit"), "utf8"));
  for (const file of [...present, ...shellChecks]) {
    if (!ciText.includes(file)) {
      failures.push(
        `scripts/${file} has no step in scripts/ci.sh — it is exercised here and never ` +
          `runs in CI or the pre-commit hook`,
      );
    }
  }
  // The staged mode is the hook's half of the size gate, and the only half that
  // can stop a blob being written at all.
  if (!hookText.includes("check-file-size.sh staged")) {
    failures.push(
      ".githooks/pre-commit does not run `check-file-size.sh staged` — the mode that " +
        "refuses the blob before it is committed exists only there",
    );
  }
  // And the hook's OTHER half. Everything above establishes that ci.sh runs each
  // check; the hook reaches all of them through one line, `scripts/ci.sh docs`,
  // and only that line was never asserted. Delete it and the hook still exits 0
  // after running the size gate alone — a gate that looks installed, passes, and
  // checks a fraction of what its own header says it does.
  if (!/scripts\/ci\.sh docs/.test(hookText)) {
    failures.push(
      '.githooks/pre-commit does not run `scripts/ci.sh docs` — that one line is how the ' +
        "hook reaches every check above, and without it the hook is the size gate and nothing else",
    );
  }
  // And how CI reaches ci.sh at all. Both rules above are satisfied by a
  // workflow that never invokes the script: the two jobs are the only thing that
  // makes a fresh clone — the only tree that sees what a reviewer sees — run any
  // of this. The workflow deliberately holds no step list, so these two strings
  // are the whole contract between it and the file above.
  const workflowPath = join(repo, ".github/workflows/ci.yml");
  if (!existsSync(workflowPath)) {
    failures.push(
      ".github/workflows/ci.yml is missing — nothing runs scripts/ci.sh on a fresh clone, " +
        "which is the only tree that sees what CI sees",
    );
  } else {
    // The `run:` values only, for the reason above one layer along: the docs job's
    // step is NAMED "Fast checks (scripts/ci.sh docs)", so a workflow that had
    // stopped invoking the script entirely still contained the string.
    const workflow = [...readFileSync(workflowPath, "utf8").matchAll(/^\s*run:\s*(.+)$/gm)]
      .map((m) => m[1])
      .join("\n");
    for (const half of ["docs", "rust"]) {
      if (!workflow.includes(`scripts/ci.sh ${half}`)) {
        failures.push(
          `.github/workflows/ci.yml never runs \`scripts/ci.sh ${half}\` — that half of the ` +
            `suite is exercised locally and gates nothing on a pull request`,
        );
      }
    }
  }

  for (const [key, run] of Object.entries(CHECKS)) {
    if (run.script && !present.includes(run.script)) {
      failures.push(`CHECKS.${key} names scripts/${run.script}, which does not exist`);
    }
    if (!MUTATIONS.some((m) => m.check === key && !m.skipIfClean)) {
      failures.push(`the ${key} check has no seeded defect — nobody has ever seen it fail`);
    }
  }

  // ── …and per RULE, not just per check ────────────────────────────────────
  // The loop above is satisfied for `registers` by any one of sixty mutations,
  // so three of its rules shipped with no seeded defect at all behind a green
  // tick — and two of those matched digits only, long after NUMBER_WORDS existed
  // for exactly the case they missed. Every pattern lib/registers.mjs exports is
  // iterated here, so ADDING one without a mutation fails, the same way the
  // table-vs-scripts guard above makes adding a check without mutations fail.
  //
  // The tag is not taken on trust either: each mutation carries the exact text it
  // seeds, and that text is matched against the WHOLE pattern list. A mutation
  // tagged for a rule that a sibling rule also catches proves nothing about the
  // rule it names — delete that rule and the suite stays green, which is the
  // state these guards exist to make impossible.
  const RULES = [
    ...TOTAL_CLAIMS.map((t) => ({
      id: `totals:${t.id}`,
      hits: (probe) => TOTAL_CLAIMS.filter((x) => x.re.test(probe)).map((x) => `totals:${x.id}`),
    })),
    ...CITATION_CLAIMS.map((c) => ({
      id: `citation:${c.id}`,
      hits: (probe) =>
        CITATION_CLAIMS.filter((x) => x.re.test(x.side === "before" ? probe.before : probe.after))
          .map((x) => `citation:${x.id}`),
    })),
  ];
  const known = new Set(RULES.map((r) => r.id));
  for (const m of MUTATIONS) {
    if (m.rule && !known.has(m.rule)) {
      failures.push(
        `${m.name}: is tagged rule "${m.rule}", which is not a pattern exported by ` +
          `lib/registers.mjs — a tag naming nothing covers nothing`,
      );
    }
  }
  for (const rule of RULES) {
    const seeded = MUTATIONS.filter((m) => m.rule === rule.id && !m.skipIfClean);
    if (seeded.length === 0) {
      failures.push(
        `the "${rule.id}" pattern has no seeded defect — the per-check guard above is ` +
          `satisfied by any other mutation of the same check, so nothing has ever seen ` +
          `this rule fire`,
      );
      continue;
    }
    for (const m of seeded) {
      const hits = rule.hits(m.probe);
      if (!hits.includes(rule.id)) {
        failures.push(
          `${m.name}: is tagged "${rule.id}", but the text it seeds does not match that ` +
            `pattern — the tag and the mutation disagree`,
        );
      } else if (hits.length > 1) {
        failures.push(
          `${m.name}: the text it seeds is also matched by ${hits.filter((h) => h !== rule.id).join(", ")} — ` +
            `delete "${rule.id}" and this mutation is still caught, so it pins nothing`,
        );
      }
    }
  }
}

// ── The file-size gate, exercised in a throwaway repository ────────────────
// It has no corpus to mutate, so it gets its own section. It needs one: shipped
// inline in the hook, it aborted EVERY commit — `set -e` plus a while-loop whose
// last command returns 1 for each file under the limit — and nothing noticed,
// because the only case ever tested was the one that is supposed to fail.
//
// BOTH MODES are exercised. `staged` is what the pre-commit hook runs and `repo`
// is what scripts/ci.sh runs, and for a while only the first existed — so the
// one gate against a failure this corpus calls permanent lived entirely in an
// opt-in hook that `--no-verify` skips. A mode nobody tests is a mode nobody has
// seen work.
{
  const dir = mkdtempSync(join(tmpdir(), "check-size-"));
  // Every setup command is checked, and a failure THROWS rather than being
  // recorded and walked past. When `git commit` failed for an unrelated reason —
  // a global `commit.gpgsign` with no key, a template hook — the fixture left
  // the blob in the index, the gate then behaved correctly, and this suite
  // reported the gate as broken. Recording the setup failure and continuing only
  // shortened that to three wrong accusations plus one right one; the run has to
  // stop, because no conclusion about the script is available after it.
  //
  // `-c commit.gpgsign=false` covers signing; hooks are covered by `--no-verify`
  // at the two commit sites, not by a config override.
  const git = (...a) => {
    const r = spawnSync("git", ["-c", "commit.gpgsign=false", ...a], { cwd: dir, encoding: "utf8" });
    if (r.status !== 0) {
      const e = new Error(
        `\`git ${a[0]}\` exited ${r.status}: ${(r.stderr ?? "").trim().split("\n")[0]}`,
      );
      e.fixture = true;
      throw e;
    }
    return r;
  };
  const gateEnv = (env, ...args) =>
    spawnSync("bash", [resolve(repo, "scripts/check-file-size.sh"), ...args], {
      cwd: dir,
      encoding: "utf8",
      env: { ...process.env, MAX_KB: "512", ...env },
    });
  const gate = (...args) => gateEnv({}, ...args);
  const write = (name, bytes) => writeFileSync(join(dir, name), "x".repeat(bytes));

  // A refusal has to be the RIGHT refusal. Exit 2 — an empty list, an unknown
  // mode — is also non-zero, so `status !== 0` scored a gate that inspected
  // nothing as a gate that caught something, which is this file's own subject
  // matter. Demand exit 1 and the offending path in the message.
  const refuses = (r, path) => r.status === 1 && `${r.stdout}${r.stderr}`.includes(path);

  try {
  git("init", "-q");
  git("config", "user.email", "t@example.com");
  git("config", "user.name", "t");

  // Zero inputs, before anything is added. `staged` must pass — a commit that
  // only deletes files stages nothing under ACMR — and `repo` must NOT, because
  // "no files are too large" over an empty repository is the green-on-nothing
  // result this whole file exists to make impossible.
  if (gate().status !== 0) {
    failures.push("file-size gate: staged mode rejects an empty index, where nothing is being added");
  }
  if (gate("repo").status !== 2) {
    failures.push(
      "file-size gate: repo mode reported on a repository with no files at all\n" +
        "  success over zero inputs is indistinguishable from success",
    );
  }

  writeFileSync(join(dir, "small.md"), "# small\n");
  git("add", "-A");
  if (gate().status !== 0) {
    failures.push(
      "file-size gate: rejects a commit containing only small files\n" +
        "  this is the shape that silently blocked every commit once already",
    );
  }
  if (gate("repo").status !== 0) {
    failures.push("file-size gate: repo mode rejects a tree of only small files");
  }

  // THE BOUNDARY. `kb=$((size / 1024))` truncates, so the gate advertised as
  // 512 KB accepted everything below 513 KB — and 0 bytes against 700 KB is a
  // pair of tests no rounding error can fall between. Exactly at the cap passes;
  // one byte over does not.
  write("edge.bin", 512 * 1024);
  git("add", "-A");
  if (gate().status !== 0) {
    failures.push("file-size gate: refused a file of exactly MAX_KB — the cap is inclusive");
  }
  write("edge.bin", 512 * 1024 + 1);
  git("add", "-A");
  if (!refuses(gate(), "edge.bin")) {
    failures.push(
      "file-size gate: accepted a file ONE BYTE over the cap\n" +
        "  a KB-truncating comparison passes everything below MAX_KB + 1 KB",
    );
  }
  rmSync(join(dir, "edge.bin"));
  git("add", "-A");

  write("big.bin", 700 * 1024);
  git("add", "-A");
  if (!refuses(gate(), "big.bin")) {
    failures.push("file-size gate: accepted a 700 KB file");
  }
  // The mode CI runs. A contributor who never installed the hook, or who used
  // --no-verify once, is caught here or nowhere.
  if (!refuses(gate("repo"), "big.bin")) {
    failures.push("file-size gate: repo mode accepted a 700 KB file");
  }

  // Staged, then removed from the working tree: the blob is still committed, so
  // a working-tree stat would wave it through.
  rmSync(join(dir, "big.bin"));
  if (!refuses(gate(), "big.bin")) {
    failures.push("file-size gate: accepted a large staged blob whose file was deleted");
  }

  // THE CASE THAT SEPARATES THE TWO MODES, and the reason everything above it
  // is not enough. Commit the oversized file and the index goes quiet: nothing
  // is staged, so the staged mode has nothing to look at and passes — correctly,
  // since nothing is being added. The repo mode must still name the file.
  //
  // Until this case existed, `repo` could be reimplemented as `git diff --cached`
  // and every assertion above stayed green: in a throwaway repo where nothing is
  // ever committed, the staged list and the tracked list are the same list. A
  // mode only ever tested where it cannot differ is untested.
  write("big.bin", 700 * 1024);
  git("add", "-A");
  git("commit", "-qm", "big", "--no-verify");
  if (gate().status !== 0) {
    failures.push("file-size gate: staged mode failed a commit that stages nothing at all");
  }
  if (!refuses(gate("repo"), "big.bin")) {
    failures.push(
      "file-size gate: repo mode missed a large file that is already committed\n" +
        "  the tracked list is not the staged list — that difference is the mode's whole point",
    );
  }

  // TWO DIFFERENT BLOBS AT ONE PATH: 700 KB committed, 800 KB staged over it.
  // De-duplicated by path rather than by blob, the history entry was dropped as
  // a repeat and the report understated what a clone would carry.
  write("big.bin", 800 * 1024);
  git("add", "-A");
  {
    const r = gate("repo");
    const out = `${r.stdout}${r.stderr}`;
    if (!out.includes("800 KB") || !out.includes("700 KB")) {
      failures.push(
        "file-size gate: reported only one of two oversized blobs at the same path\n" +
          `  the index holds 800 KB and the history holds 700 KB; it said: ${out.trim().split("\n")[1] ?? ""}`,
      );
    }
  }

  // AND THE ONE THE INDEX CANNOT SEE AT ALL: committed, then deleted in a later
  // commit. `git ls-files` no longer lists it, the working tree no longer has
  // it, and the blob is still in the history the merge would carry — which is
  // exactly the 528 MB target/ incident, and exactly what the failure message
  // ("the blob is already in the history") had been claiming to check.
  rmSync(join(dir, "big.bin"));
  git("add", "-A");
  git("commit", "-qm", "drop big", "--no-verify");
  if (!refuses(gate("repo"), "big.bin")) {
    failures.push(
      "file-size gate: repo mode passed a branch whose history carries a 700 KB blob\n" +
        "  deleting the file in a later commit does not remove it from what a clone gets",
    );
  }

  // AND THE SCOPE OF THAT WALK. Unbounded, it judges the base branch as well:
  // one oversized blob anywhere in main's past then fails every PR forever, with
  // a message telling the author to rewrite a branch that did not introduce it —
  // a confident wrong diagnosis aimed at the wrong person. SIZE_BASE_REF bounds
  // it to the commits under review, and the blob above now sits in the base.
  const base = git("rev-parse", "HEAD").stdout.trim();
  git("checkout", "-q", "-b", "feature");
  writeFileSync(join(dir, "n.txt"), "new\n");
  git("add", "-A");
  git("commit", "-qm", "feature", "--no-verify");
  if (!refuses(gate("repo"), "big.bin")) {
    failures.push("file-size gate: unbounded repo mode stopped seeing the blob in the history");
  }
  if (gateEnv({ SIZE_BASE_REF: base }, "repo").status !== 0) {
    failures.push(
      "file-size gate: SIZE_BASE_REF did not bound the history walk\n" +
        "  a blob inherited from the base branch fails a PR that did not add it, and the " +
        "author cannot rewrite the base",
    );
  }

  // A path git cannot size — an unmerged entry mid-merge, a corrupt object —
  // must be REPORTED. It used to `continue` in silence without counting toward
  // the total, so the success line named a number that quietly excluded it,
  // which is the shape the comment about `< <(...)` refuses fifteen lines above
  // in the same script.
  git("update-index", "--add", "--cacheinfo",
    "100644,1111111111111111111111111111111111111111,ghost.txt");
  if (!refuses(gate(), "ghost.txt")) {
    failures.push(
      "file-size gate: skipped a path it could not size, without saying so\n" +
        "  an unvouched-for file inside a green run is the failure this gate is about",
    );
  }

  // THE SHIPPED LIMIT, not just the mechanism. Every assertion above forces
  // MAX_KB=512 into the environment and is written against 512 * 1024, so the
  // default in check-file-size.sh could be changed to anything — or dropped —
  // with this file still reporting that the gate behaves.
  {
    const env = { ...process.env };
    delete env.MAX_KB;
    const r = spawnSync("bash", [resolve(repo, "scripts/check-file-size.sh"), "repo"], {
      cwd: dir,
      encoding: "utf8",
      env,
    });
    if (!`${r.stdout}${r.stderr}`.includes("512 KB")) {
      failures.push(
        "file-size gate: with MAX_KB unset the run does not name a 512 KB cap — the " +
          "shipped default is what a contributor without the env var actually gets",
      );
    }
  }

  // An unknown mode must be refused, not silently treated as the default. A
  // typo'd argument that falls back to `staged` would make the CI step pass
  // while checking the index, which on a fresh checkout is empty.
  if (gate("bogus").status !== 2) {
    failures.push("file-size gate: an unknown mode was not refused");
  }
  } catch (e) {
    if (!e.fixture) throw e;
    failures.push(
      `file-size gate: FIXTURE setup failed, not the gate — ${e.message}\n` +
        `  the block stopped here; nothing about check-file-size.sh was established`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// ── The corpus itself must pass every check ─────────────────────────────────
{
  const dir = fresh();
  for (const check of Object.keys(CHECKS)) {
    const { code, out } = run(check, dir);
    if (code !== 0) {
      failures.push(
        `baseline: ${check} rejects your working tree, before any defect was seeded.\n` +
          `  This is almost always a real problem in the tree rather than a broken check —\n` +
          `  the fixture is a copy of the working tree, so ${check} fails here for the same\n` +
          `  reason it would fail on its own. Untracked scratch files count.\n${out.trim()}`,
      );
    }
  }
  rmSync(dir, { recursive: true, force: true });
}

// ── Every mutation must be caught, by the right check, with a real message ──
for (const m of MUTATIONS) {
  // A typo'd check name used to die as `CHECKS[check] is not a function`, mid
  // table, taking every mutation after it with it — the truncation this loop's
  // own comment refuses one line down.
  if (!CHECKS[m.check]) {
    failures.push(`${m.name}: names check "${m.check}", which is not in the CHECKS table`);
    continue;
  }
  // Fixture construction sat outside the try, so anything it threw took the
  // whole run down with a raw stack trace — and every mutation after the
  // offending one silently never ran. A suite that can be truncated without
  // saying so reports coverage it does not have.
  let dir;
  try {
    dir = m.fixture === "manifests" ? manifests() : fresh();
  } catch (e) {
    failures.push(`${m.name}: fixture could not be built — ${e.message}`);
    continue;
  }
  try {
    m.mutate(dir);
  } catch (e) {
    failures.push(`${m.name}: mutation itself failed — ${e.message}`);
    rmSync(dir, { recursive: true, force: true });
    continue;
  }
  const { code, out } = run(m.check, dir);
  if (m.skipIfClean) {
    // An inverse case: this input is LEGAL and the check must accept it.
    if (code !== 0) {
      failures.push(`FALSE POSITIVE from ${m.check}: ${m.name}\n  ${out.trim().split("\n").slice(0, 3).join(" / ")}`);
    } else if (m.expect && !out.includes(m.expect)) {
      // An inverse case may also pin what the PASSING run says. Accepting an
      // input is not the whole claim when the check reports a count: a citation
      // seen twice is accepted twice, and only the ✓ line says so.
      failures.push(
        `WRONG REPORT from ${m.check}: ${m.name}\n  the input was accepted, but the run does ` +
          `not say ${JSON.stringify(m.expect)}\n  got: ${out.trim().split("\n").slice(0, 3).join(" / ")}`,
      );
    }
    rmSync(dir, { recursive: true, force: true });
    continue;
  }
  if (code === 0) {
    failures.push(`NOT CAUGHT by ${m.check}: ${m.name}\n  the check passed a corpus containing this defect`);
  } else if (m.expect && !out.includes(m.expect)) {
    failures.push(
      `WRONG MESSAGE from ${m.check}: ${m.name}\n  expected to see ${JSON.stringify(m.expect)}\n  got: ${out.trim().split("\n").slice(0, 3).join(" / ")}`,
    );
  } else if (m.reject && out.includes(m.reject)) {
    // Catching the defect is not the whole claim when the run also says
    // something false alongside it: a phantom "Q16 is missing" next to a real
    // "Q6 appears twice" is an instruction that, followed, invents an entry the
    // append-only rule forbids.
    failures.push(
      `EXTRA MESSAGE from ${m.check}: ${m.name}\n  the run also said ${JSON.stringify(m.reject)}, ` +
        `which is not true of this corpus\n  got: ${out.trim().split("\n").slice(0, 4).join(" / ")}`,
    );
  }
  rmSync(dir, { recursive: true, force: true });
}

if (failures.length) {
  console.error(`✗ ${failures.length} problem(s) with the checks themselves:\n`);
  for (const f of failures) console.error("  " + f.replace(/\n/g, "\n    ") + "\n");
  process.exit(1);
}

// Inverse cases are NOT seeded defects: they assert the check stays quiet on a
// legal input. Counting them in the defect total meant tightening a parser and
// pinning it with a false-positive guard raised the advertised coverage while
// catching nothing new — a success line naming a total that includes what it did
// not do, which is the shape this file exists to refuse.
const inverse = MUTATIONS.filter((m) => m.skipIfClean).length;
console.log(
  `✓ ${MUTATIONS.length - inverse} seeded defects each caught by the right check, ` +
    `${inverse} legal inputs each accepted, ` +
    `${Object.keys(CHECKS).length} checks pass the corpus clean, ` +
    `the file-size gate behaves in both modes, ` +
    `and the workspace lint opt-in is in force`,
);
