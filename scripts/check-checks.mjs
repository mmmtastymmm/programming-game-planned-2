#!/usr/bin/env node
// The checks, checked.
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

const SKIP = new Set(WALKER_SKIP);
import { spawnSync } from "node:child_process";
// The member parser the `lints` check itself uses, for the manifest-only
// fixture below. Importing it rather than re-deriving the member list is the
// whole point: two parsers for one format is what made a commented-out member
// kill this run with a raw ENOENT.
import { workspaceMembers } from "./lib/workspace-lints.mjs";

const repo = process.argv[2] ?? ".";

// Each check, and how it is invoked against a corpus root. Every entry returns
// {code, out} so an in-process check sits on the same driver as a spawned one:
// the workspace-lints check was originally bolted on beside this table and
// reimplemented fixture creation, mutation-failure handling, exit checking and
// cleanup — getting each slightly wrong, and skipping `expect` entirely.
const script = (name) => (dir) => {
  const r = spawnSync("node", [`scripts/${name}`, dir], { encoding: "utf8", cwd: repo });
  return { code: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
};

const CHECKS = {
  links: script("check-links.mjs"),
  registers: script("check-registers.mjs"),
  layout: (dir) => {
    const r = spawnSync("node", ["scripts/check-doc-layout.mjs", join(dir, "docs")], {
      encoding: "utf8",
      cwd: repo,
    });
    return { code: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
  },
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

const qa = (d) => join(d, "docs/history/questions-answered");
const tc = (d) => join(d, "docs/history/tasks-completed");
const pf = (d) => join(d, "docs/history/problems-fixed");

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
  { name: "a nested bullet's KIND is validated, not just its citation",
    check: "registers", expect: "which is not one of",
    // The citation case above survives an anchor regression by accident: swallow
    // the nested line as prose and the parent bullet's text still contains
    // "T4242", so the same "not in the tasks register" message fires and the
    // suite stays green. Kind validation does NOT survive it — and that is the
    // path by which the inbox-only Dropped gets smuggled into a ruling. Pinning
    // it needs a forbidden kind and a different expected message.
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — nested kind\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "  - **Dropped:** smuggled in under a well-formed bullet.\n") },
  { name: "an Outcome bullet with the colon outside the bold", check: "registers",
    expect: "not in the tasks register",
    // `- **Task**: …` renders the same and used to match nothing, so the
    // continuation loop absorbed it as prose belonging to the bullet above and
    // its kind went unvalidated and its citation unresolved.
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — loose colon\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "- **Task**: T4242 which is not real.\n") },
  { name: "an Outcome bullet with extra space after the list marker",
    check: "registers", expect: "not in the tasks register",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — wide marker\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "-   **Task:** T4242 which is not real.\n") },
  { name: "the inbox-only Dropped, spelled with the colon outside the bold",
    check: "registers", expect: "which is not one of",
    mutate: (d) => writeFileSync(join(qa(d), "question-answered-0006.md"),
      "# Q6 — loose dropped\n\n## Outcome\n\n- **Docs:** [x](../../00-overview.md)\n" +
        "- **Dropped**: turned out not to matter.\n") },
  { name: "a nested PROSE bullet is still continuation, not a new bullet",
    check: "registers", expect: "", skipIfClean: true,
    // The inverse of the four above: tightening the parser must not start
    // reading an ordinary explanatory sub-bullet as a malformed declaration.
    // Written into the INBOX register, which has no open entries — a
    // question-answered file would collide with the still-open Q6 and fail for
    // an unrelated reason, which is not what this case is asking.
    mutate: (d) => writeFileSync(join(d, "docs/history/inbox-triaged/inbox-triaged-0001.md"),
      "# I1 — a triaged note\n\n## Outcome\n\n- **Task:** [T7](../../TASKS.md)\n" +
        "  - and some nested prose explaining it\n") },
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
  { name: "a restated total with the adverb AFTER the verb", check: "registers",
    expect: "restates a register total",
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nThree tasks are still open.\n") },
  { name: "a restated total with the adverb BETWEEN noun and verb", check: "registers",
    expect: "restates a register total",
    // This is the shape the regex's adverb slot exists for. The case above sits
    // ENTIRELY inside the base pattern ("three tasks are"), so deleting the slot
    // left the suite green — the mutation and the comment justifying the slot
    // made the same mistake about where an adverb lands.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nThree tasks still remain open.\n") },
  { name: "a restated total worded with zero", check: "registers",
    expect: "restates a register total",
    // PROBLEMS.md's own derived headline says "zero open", and this list could
    // not spell it.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nZero questions are open.\n") },
  { name: "a restated total with the adjective in front of the noun",
    check: "registers", expect: "restates a register total",
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nThere are 8 open questions right now.\n") },
  { name: "a restated total with the label first", check: "registers",
    expect: "restates a register total",
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nOpen: 8 questions, 2 problems.\n") },
  { name: "a restated total WRAPPED across two lines", check: "registers",
    expect: "restates a register total",
    // The corpus wraps at about 80 columns, so a per-line scan let the rule
    // CLAUDE.md spends the most words on be defeated by a line break.
    mutate: (d) => appendFileSync(join(d, "docs/TASKS.md"), "\nEight questions\nare open.\n") },
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
  { name: "a code-span citation in neither supported form", check: "links",
    expect: "unverifiable citation",
    mutate: (d) => appendFileSync(join(d, "docs/INBOX.md"), "\nSee `PROBLEMS.md at :24` for context.\n") },
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

  // ── layout: parts nest ───────────────────────────────────────────────────
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
    check: "registers", expect: "says 0 opened, the register has 2",
    // opened counts open PLUS closed, which is the arm most easily miswired to
    // one or the other: with a closed P1 and an open P2 the three arms read 2,
    // 1 and 1, so no two of them can be confused.
    mutate: (d) => {
      writeFileSync(join(pf(d), "problem-fixed-0001.md"), "# P1 — fixed\n\nbody\n");
      appendFileSync(join(d, "docs/PROBLEMS.md"), "\n**P2 — still open**\n\nbody\n");
    } },
  { name: "the headline's FIXED total", check: "registers",
    expect: "says 0 fixed, the register has 1",
    mutate: (d) => {
      writeFileSync(join(pf(d), "problem-fixed-0001.md"), "# P1 — fixed\n\nbody\n");
      edit(join(d, "docs/PROBLEMS.md"), /0 opened, 0 fixed/, "1 opened, 0 fixed");
    } },
  { name: "the headline's OPEN total", check: "registers",
    expect: "the register has 1 (P1)",
    mutate: (d) => {
      appendFileSync(join(d, "docs/PROBLEMS.md"), "\n**P1 — still open**\n\nbody\n");
      edit(join(d, "docs/PROBLEMS.md"), /0 opened, 0 fixed/, "1 opened, 0 fixed");
    } },
  { name: "a status headline that is not in the counted form at all",
    check: "registers", expect: "malformed status headline",
    // Dated, so it is found as the register's one status block, but carrying no
    // numbers — which is how a headline stops being checked without disappearing.
    mutate: (d) => edit(join(d, "docs/PROBLEMS.md"),
      /: 0 opened, 0 fixed — zero open\.\*\*/, ": all totals current.**") },
  { name: "a headline count spelled as a word the checker cannot read",
    check: "registers", expect: "is not a number this checker knows",
    mutate: (d) => edit(join(d, "docs/PROBLEMS.md"), /— zero open/, "— twenty-one open") },

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

  // ── registers: numbering ─────────────────────────────────────────────────
  // Both rules are enumerated in check-registers' own header and in CLAUDE.md's
  // register table, and neither had a mutation. The first is the commonest real
  // mistake there is: closing an entry and forgetting to delete it from the live
  // register, which leaves it open and closed at once.
  { name: "an entry that is open and closed at once", check: "registers",
    expect: "appears twice",
    mutate: (d) => writeFileSync(join(tc(d), "task-completed-0007.md"),
      "# T7 — closed while still listed as open\n") },
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
 * member list cannot change between fixtures, and manifests() is built eight
 * times.
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
  // Every setup command is checked. When `git commit` failed for an unrelated
  // reason — a global `commit.gpgsign` with no key, a global `core.hooksPath` —
  // the fixture left the blob in the index, the gate then behaved correctly, and
  // this suite reported the gate as broken: a confident wrong diagnosis pointing
  // at the script under test, which is the failure mode it exists to prevent.
  // The two config overrides remove the commonest causes; the status check
  // catches the rest and names the fixture.
  const git = (...a) => {
    const r = spawnSync("git", ["-c", "commit.gpgsign=false", ...a], { cwd: dir, encoding: "utf8" });
    if (r.status !== 0) {
      failures.push(
        `file-size gate: FIXTURE setup failed, not the gate — \`git ${a[0]}\` exited ${r.status}\n` +
          `  ${(r.stderr ?? "").trim().split("\n")[0]}`,
      );
    }
    return r;
  };
  const gate = (...args) =>
    spawnSync("bash", [resolve(repo, "scripts/check-file-size.sh"), ...args], {
      cwd: dir,
      encoding: "utf8",
      env: { ...process.env, MAX_KB: "512" },
    });
  const write = (name, bytes) => writeFileSync(join(dir, name), "x".repeat(bytes));

  // A refusal has to be the RIGHT refusal. Exit 2 — an empty list, an unknown
  // mode — is also non-zero, so `status !== 0` scored a gate that inspected
  // nothing as a gate that caught something, which is this file's own subject
  // matter. Demand exit 1 and the offending path in the message.
  const refuses = (r, path) => r.status === 1 && `${r.stdout}${r.stderr}`.includes(path);

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

  // AND THE ONE THE INDEX CANNOT SEE AT ALL: committed, then deleted in a later
  // commit. `git ls-files` no longer lists it, the working tree no longer has
  // it, and the blob is still in the history the merge would carry — which is
  // exactly the 528 MB target/ incident, and exactly what the failure message
  // ("the blob is already in the history") had been claiming to check.
  rmSync(join(dir, "big.bin"));
  git("add", "-A");
  git("commit", "-qm", "drop big", "--no-verify");
  if (gate("repo").status === 0) {
    failures.push(
      "file-size gate: repo mode passed a branch whose history carries a 700 KB blob\n" +
        "  deleting the file in a later commit does not remove it from what a clone gets",
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

  // An unknown mode must be refused, not silently treated as the default. A
  // typo'd argument that falls back to `staged` would make the CI step pass
  // while checking the index, which on a fresh checkout is empty.
  if (gate("bogus").status !== 2) {
    failures.push("file-size gate: an unknown mode was not refused");
  }

  rmSync(dir, { recursive: true, force: true });
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
    `the file-size gate behaves in both modes, ` +
    `and the workspace lint opt-in is in force`,
);
