#!/usr/bin/env node
// The doc checks, checked.
//
//   node scripts/check-checks.mjs .
//
// WHY THIS EXISTS — and it is the most load-bearing check in the repo.
//
// Three review rounds of this corpus found 45 issues. The damaging ones were
// almost never wrong prose; they were checks that PASSED WHILE VALIDATING
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

import { cpSync, mkdtempSync, rmSync, writeFileSync, appendFileSync, readFileSync, readdirSync, renameSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";

const repo = process.argv[2] ?? ".";

// Each check, and how it is invoked against a corpus root.
const CHECKS = {
  links: (dir) => ["scripts/check-links.mjs", dir],
  registers: (dir) => ["scripts/check-registers.mjs", dir],
  layout: (dir) => ["scripts/check-doc-layout.mjs", join(dir, "docs")],
  structure: (dir) => ["scripts/check-structure.mjs", dir],
  vocabulary: (dir) => ["scripts/check-vocabulary.mjs", dir],
  mermaid: (dir) => ["scripts/check-mermaid.mjs", dir],
};

const qa = (d) => join(d, "docs/history/questions-answered");
const tc = (d) => join(d, "docs/history/tasks-completed");

const MUTATIONS = [
  // ── registers: entry identity ─────────────────────────────────────────────
  { name: "history file named for one entry, headed as another", check: "registers",
    expect: "the filename is the index",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-0009.md"), "# T99 — mislabelled\n") },
  { name: "two entries smuggled into one file", check: "registers",
    expect: "one entry per file",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-0009.md"), "# T9 — one\n\n# T10 — two\n") },
  { name: "entry numbered zero", check: "registers",
    expect: "registers number from 1",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-0000.md"), "# T0 — zero\n") },
  { name: "misnamed entry file (three digits)", check: "registers",
    expect: "is misnamed",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-009.md"), "# T9 — short name\n") },
  { name: "a renamed history directory", check: "registers",
    expect: "does not exist",
    mutate: (d) => renameSync(join(d, "docs/history/problems-fixed"), join(d, "docs/history/problems-fixd")) },

  // ── registers: the Outcome contract ───────────────────────────────────────
  { name: "answered question with no Outcome section", check: "registers",
    expect: 'has no "## Outcome" section',
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"), "# Q6 — no outcome\n\nbody\n") },
  { name: "Outcome citing a number that does not exist", check: "registers",
    expect: "not in the tasks register",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — dangling\n\n## Outcome\n\n- **Task:** T4242 which is not real.\n") },
  { name: "Outcome citation on a wrapped continuation line", check: "registers",
    expect: "not in the tasks register",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — wrapped\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md) is real, and so is\n  T4242 which is not.\n") },
  { name: "Outcome citation in a nested bullet", check: "registers",
    expect: "not in the tasks register",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — nested\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md)\n  - **Task:** T4242 nested\n") },
  { name: "bare Docs bullet linking nothing", check: "registers",
    expect: "links nothing",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — empty docs\n\n## Outcome\n\n- **Docs:**\n") },
  { name: "a ruling using the inbox-only Dropped outcome", check: "registers",
    expect: "which is not one of",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — dropped\n\n## Outcome\n\n- **Dropped:** decided it did not matter after all.\n") },

  // ── registers: totals, size, commits ──────────────────────────────────────
  { name: "a restated total in CLAUDE.md (outside docs/)", check: "registers",
    expect: "restates a register total",
    mutate: (d) => appendFileSync(join(d, "CLAUDE.md"), "\nEight questions are open.\n") },
  { name: "a restated total evading the rule with an adverb", check: "registers",
    expect: "restates a register total",
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nThree tasks are still open.\n") },
  { name: "a design doc over the size cap", check: "registers",
    expect: "over the 40 KB cap",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"), "x".repeat(45000)) },
  { name: "a cited commit that is not in this repository", check: "registers",
    expect: "not a commit in this repository",
    mutate: (d) => appendFileSync(join(tc(d), "task-completed-0001.md"), "\nAlso `0bad1ce`.\n") },
  { name: "stacked status blocks", check: "registers",
    expect: "states its status once",
    mutate: (d) => appendFileSync(join(d, "docs/QUESTIONS.md"), "\n**Status 2020-01-01.** stale\n") },

  // ── structure ─────────────────────────────────────────────────────────────
  { name: "the same heading twice in one file (a splice)", check: "structure",
    expect: "duplicate heading",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"), "\n## Decided\n\nspliced\n") },
  { name: "a table separator that lost its header", check: "structure",
    expect: "no header row above it",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n---|---|\n| a | b |\n") },
  { name: "a table row with the wrong column count", check: "structure",
    expect: "columns, header has",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n| a | b |\n|---|---|\n| x | y | z |\n") },

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
  { name: "README miscounting the registers", check: "vocabulary",
    expect: "registers",
    mutate: (d) => {
      const f = join(d, "README.md");
      writeFileSync(f, readFileSync(f, "utf8").replace(/four registers/g, "three registers"));
    } },

  // ── links, layout, mermaid ────────────────────────────────────────────────
  { name: "a link to a file that does not exist", check: "links",
    expect: "missing file",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\n[nope](does-not-exist.md)\n") },
  { name: "a code-span citation in neither supported form", check: "links",
    expect: "unverifiable citation",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\nSee `PROBLEMS.md at :24` for context.\n") },
  { name: "a part file with an inverted breadcrumb", check: "layout",
    expect: "breadcrumb",
    mutate: (d) => {
      mkdirSync(join(d, "docs/01-language"), { recursive: true });
      writeFileSync(join(d, "docs/01-language/syntax.md"),
        "# Syntax\n\n*Part of [01-language](../01-language.md).*\n");
    } },
  { name: "a mermaid diagram that does not parse", check: "mermaid",
    expect: "error on line",
    mutate: (d) => appendFileSync(join(d, "docs/00-overview.md"),
      "\n```mermaid\nflowchart LR\n  a --> b\n  ]]] not valid\n```\n") },
];

function run(check, dir) {
  const r = spawnSync("node", CHECKS[check](dir), { encoding: "utf8", cwd: repo });
  return { code: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
}

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
const SKIP = new Set([".git", "node_modules", "target"]);

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

function fresh() {
  const dir = mkdtempSync(join(tmpdir(), "check-checks-"));
  if (copyTree(resolve(repo), dir) === 0) {
    console.error("✗ check-checks: copied an empty corpus — the suite would test nothing");
    process.exit(2);
  }
  return dir;
}

const failures = [];

// ── Workspace lints are actually in force ──────────────────────────────────
// `[workspace.lints]` does nothing for a crate that omits `[lints] workspace =
// true`, and clippy stays green either way. The deny exists so the coming
// language crate is covered; a silent opt-out would defeat it exactly when it
// starts to matter.
{
  const rootManifest = readFileSync(join(repo, "Cargo.toml"), "utf8");
  if (!/\[workspace\.lints\.clippy\]/.test(rootManifest)) {
    failures.push("Cargo.toml: no [workspace.lints.clippy] section");
  }
  const members = /members\s*=\s*\[([^\]]*)\]/.exec(rootManifest);
  const paths = members ? [...members[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]) : [];
  if (paths.length === 0) failures.push("Cargo.toml: could not read workspace members");
  for (const rel of paths) {
    const manifest = readFileSync(join(repo, rel, "Cargo.toml"), "utf8");
    if (!/\[lints\][\s\S]*?workspace\s*=\s*true/.test(manifest)) {
      failures.push(
        `${rel}/Cargo.toml: does not opt into the workspace lints ` +
          `([lints] workspace = true) — the arithmetic_side_effects deny is inert here`,
      );
    }
  }
}

// ── The staged-size gate, exercised in a throwaway repository ───────────────
// It has no corpus to mutate, so it gets its own section. It needs one: shipped
// inline in the hook, it aborted EVERY commit — `set -e` plus a while-loop whose
// last command returns 1 for each file under the limit — and nothing noticed,
// because the only case ever tested was the one that is supposed to fail.
{
  const dir = mkdtempSync(join(tmpdir(), "check-size-"));
  const git = (...a) => spawnSync("git", a, { cwd: dir, encoding: "utf8" });
  const gate = () =>
    spawnSync("bash", [resolve(repo, "scripts/check-staged-size.sh")], {
      cwd: dir,
      encoding: "utf8",
      env: { ...process.env, MAX_KB: "512" },
    });

  git("init", "-q");
  git("config", "user.email", "t@example.com");
  git("config", "user.name", "t");

  writeFileSync(join(dir, "small.md"), "# small\n");
  git("add", "-A");
  if (gate().status !== 0) {
    failures.push(
      "staged-size gate: rejects a commit containing only small files\n" +
        "  this is the shape that silently blocked every commit once already",
    );
  }

  writeFileSync(join(dir, "big.bin"), "x".repeat(700 * 1024));
  git("add", "-A");
  if (gate().status === 0) {
    failures.push("staged-size gate: accepted a 700 KB file");
  }

  // Staged, then removed from the working tree: the blob is still committed, so
  // a working-tree stat would wave it through.
  rmSync(join(dir, "big.bin"));
  if (gate().status === 0) {
    failures.push("staged-size gate: accepted a large staged blob whose file was deleted");
  }

  rmSync(dir, { recursive: true, force: true });
}

// ── The corpus itself must pass every check ─────────────────────────────────
{
  const dir = fresh();
  for (const check of Object.keys(CHECKS)) {
    const { code, out } = run(check, dir);
    if (code !== 0) {
      failures.push(`baseline: ${check} rejects the unmutated corpus\n${out.trim()}`);
    }
  }
  rmSync(dir, { recursive: true, force: true });
}

// ── Every mutation must be caught, by the right check, with a real message ──
for (const m of MUTATIONS) {
  const dir = fresh();
  try {
    m.mutate(dir);
  } catch (e) {
    failures.push(`${m.name}: mutation itself failed — ${e.message}`);
    rmSync(dir, { recursive: true, force: true });
    continue;
  }
  const { code, out } = run(m.check, dir);
  if (code === 0) {
    failures.push(`NOT CAUGHT by ${m.check}: ${m.name}\n  the check passed a corpus containing this defect`);
  } else if (m.expect && !out.includes(m.expect)) {
    failures.push(
      `WRONG MESSAGE from ${m.check}: ${m.name}\n  expected to see ${JSON.stringify(m.expect)}\n  got: ${out.trim().split("\n").slice(0, 3).join(" / ")}`,
    );
  }
  rmSync(dir, { recursive: true, force: true });
}

if (failures.length) {
  console.error(`✗ ${failures.length} problem(s) with the checks themselves:\n`);
  for (const f of failures) console.error("  " + f.replace(/\n/g, "\n    ") + "\n");
  process.exit(1);
}

console.log(
  `✓ ${MUTATIONS.length} seeded defects each caught by the right check, ` +
    `${Object.keys(CHECKS).length} checks pass the corpus clean, ` +
    `and the staged-size gate behaves on all three of its cases`,
);
