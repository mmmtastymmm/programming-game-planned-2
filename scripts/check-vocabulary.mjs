#!/usr/bin/env node
// The docs that describe the register scheme must agree with the code that
// enforces it.
//
//   node scripts/check-vocabulary.mjs .
//
// WHY THIS EXISTS. In one review round the docs and the tooling had drifted
// apart in five separate places at once:
//
//   * questions-answered/README.md — the file that DEFINES the Outcome
//     vocabulary for its own directory — listed three of the four kinds the
//     checker accepts, while a file in that very directory already used the
//     fourth.
//   * INBOX.md said triage produces "exactly one of" five outcomes; the checker
//     allows several and CLAUDE.md said two is normal.
//   * design-invariants.md documented a `Q42-R2` amendment-citation form that
//     the one-file-per-entry scheme cannot express and the checker rejects.
//     Four of the five files below were guarded after that round and this one
//     was not — it sits under .claude/ rather than docs/, so every pass that
//     went looking for corpus files walked past it, while it went on restating
//     which registers exist, where they live and how their entries are spelled.
//   * history/README.md described an answered-question file as two sections,
//     omitting the `## Outcome` the checker will reject the file without.
//   * README.md — the repo's front door — advertised three registers when there
//     were four.
//
// Each is a fact about the tooling, restated by hand somewhere else. That is the
// same failure the derived-counts rule exists to prevent, one level up: the
// authority is `scripts/lib/registers.mjs`, and everything else cites it.
//
// SCOPE: this checks that the docs ENUMERATE the right things — every kind
// present, no kind invented. It cannot check that the surrounding prose is
// accurate, only that the vocabulary matches.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import {
  HASH_MARKER,
  MAX_BYTES,
  NUMBER_WORDS,
  OUTCOME_KINDS,
  REGISTERS,
} from "./lib/registers.mjs";
import { breadcrumb, TEMPLATE_PARENT } from "./lib/doc-layout.mjs";

const root = process.argv[2] ?? ".";
const problems = [];
const note = (m) => problems.push(m);

function read(rel) {
  const p = join(root, rel);
  if (!existsSync(p)) {
    note(`${p}  is missing — it documents the register scheme and cannot simply go away`);
    return null;
  }
  return { path: p, text: readFileSync(p, "utf8") };
}

const ALL_KINDS = Object.keys(OUTCOME_KINDS);
let checks = 0;

/**
 * Does this doc OFFER the kind — list it as a spelling to use — as opposed to
 * merely naming it?
 *
 * The two directions need different tests. A doc must NAME every kind its
 * register accepts, in prose or in a list; but forbidding the string outright
 * meant questions-answered/README.md could not write the one sentence a writer
 * most needs there — that there is no `- **Dropped:**` for a ruling — so the
 * inbox-only rule was explained only from the inbox side, where nobody looking
 * for it goes.
 */
const offers = (text, kind) =>
  text.split("\n").some((l) => /^\s*[-*+]\s/.test(l) && l.includes(`**${kind}:**`));

// ── Each history README enumerates exactly its register's Outcome kinds ─────
// EVERY register, not just the two with an Outcome contract. Filtering on
// `r.outcome` meant problems-fixed/README.md and tasks-completed/README.md were
// never opened at all — half the history READMEs this check exists to keep
// honest — so an invented `- **Dropped:**` in either passed clean while the
// identical line in questions-answered/ was correctly rejected. A README with no
// contract is not a README with no rule: its rule is that it offers no kinds.
for (const reg of REGISTERS) {
  const doc = read(join("docs", "history", reg.dir, "README.md"));
  if (!doc) continue;
  for (const kind of ALL_KINDS) {
    checks++;
    const mentioned = doc.text.includes(`**${kind}:**`);
    const allowed = (reg.outcome ?? []).includes(kind);
    if (allowed && !mentioned) {
      note(
        `${doc.path}  does not document the "${kind}:" outcome, which the checker ` +
          `accepts here — a writer following this README will not know it exists`,
      );
    }
    if (!allowed && offers(doc.text, kind)) {
      note(
        reg.outcome
          ? `${doc.path}  offers the "${kind}:" outcome, which the checker rejects ` +
              `for the ${reg.what} register`
          : `${doc.path}  offers the "${kind}:" outcome, but the ${reg.what} register ` +
              `has no Outcome section at all — the checker will never read it`,
      );
    }
  }
}

// ── A live register with an Outcome contract names its kinds too ────────────
// INBOX.md is where triage is explained, and it spelled the outcomes as prose —
// "a documentation fix made on the spot", "**nothing**, because it turned out to
// be a misreading" — never as the `- **Docs:**` / `- **Dropped:**` forms
// check-registers requires. A writer following it wrote a file CI rejects, which
// is the failure the history-README guard above exists to prevent, one door
// further in. INBOX.md is also one of the five files this checker's header names
// as having drifted, and it was the one nothing here opened.
for (const reg of REGISTERS.filter((r) => r.outcome)) {
  const doc = read(join("docs", reg.live));
  if (!doc) continue;
  for (const kind of ALL_KINDS) {
    checks++;
    const mentioned = doc.text.includes(`**${kind}:**`);
    const allowed = reg.outcome.includes(kind);
    if (allowed && !mentioned) {
      note(
        `${doc.path}  does not name the "${kind}:" outcome, which the checker accepts ` +
          `for ${reg.what} — a writer following this file writes a file CI rejects`,
      );
    }
    if (!allowed && offers(doc.text, kind)) {
      note(`${doc.path}  offers the "${kind}:" outcome, which the checker rejects for ${reg.what}`);
    }
  }
}

// ── CLAUDE.md names every register, both halves, and every kind ─────────────
const claude = read("CLAUDE.md");
if (claude) {
  for (const reg of REGISTERS) {
    checks += 2;
    if (!claude.text.includes(reg.live)) {
      note(`CLAUDE.md  does not name docs/${reg.live}, one of the ${REGISTERS.length} registers`);
    }
    if (!claude.text.includes(`${reg.dir}/`)) {
      note(`CLAUDE.md  does not name history/${reg.dir}/, where ${reg.what} close`);
    }
  }
  for (const kind of ALL_KINDS) {
    checks++;
    if (!claude.text.includes(`**${kind}:**`)) {
      note(`CLAUDE.md  does not document the "${kind}:" outcome kind`);
    }
  }
  checks++;
  const kb = `${MAX_BYTES / 1024} KB`;
  if (!claude.text.includes(kb)) {
    note(`CLAUDE.md  does not state the size cap as "${kb}" — the checker enforces that number`);
  }
}

// ── The front door counts what it cannot recompute ─────────────────────────
// README.md said "three registers" when there were four, and "the four doc
// checks" while scripts/ held seven — the second drifting inside the very commit
// that added three of them, two lines below the count this file already guarded.
// Both are derived numbers stated by hand, so both are derived here instead.
//
// ONE WORDS list, and one helper. There were two lists that had drifted from
// each other (one stopped at "eight", one at "ten"), so the two counts silently
// supported different ranges: a ninth register would have made `want` the string
// "9" while the alternation had no "nine", and the checker would have demanded a
// spelling it could not match — the very unsatisfiable-check bug the comment
// below says was already fixed once.
const WORDS = NUMBER_WORDS;

/**
 * One count README.md states by hand, checked against the real number.
 *
 * `after` is what follows the number, and it is the ANCHOR — it decides which
 * sentence in the file is being checked, because `exec` takes the first match in
 * the whole text. It was once loosened from `doc checks` to bare `checks`, at
 * which point any earlier sentence containing a number and the word "checks"
 * would have been read as the Layout row and reported against it. The registers
 * count kept the bare noun for a while afterwards and had exactly that hole.
 *
 * `shown` is the same tail written for a person, because the "does not say how
 * many" message has to name the phrase that SATISFIES the check. Naming the noun
 * alone produced an instruction that leaves the check red when followed: README
 * said "the eight checks", the message asked for "eight checks", and the regex
 * wanted the parenthetical neither of them mentioned.
 */
function readmeCount({ text, after, shown, noun, real, source }) {
  checks++;
  // Built from WORDS, not hand-listed: a hand-listed subset made the check
  // unsatisfiable for any count outside it — README would say "seven registers"
  // and the checker would insist it did not.
  const want = WORDS[real] ?? String(real);
  const m = new RegExp(`\\b(${WORDS.join("|")}|\\d+)\\s+${after}`, "i").exec(text);
  if (!m) {
    note(`README.md  does not say how many ${noun} there are — expected "${want} ${shown}"`);
    // `want` is the word form, but the pattern deliberately accepts digits too —
    // comparing against the word alone told an author who wrote "the 8 checks"
    // that 8 was wrong while printing 8 back at them. An unsatisfiable check is
    // the failure this file's own docstrings name twice.
  } else if (m[1].toLowerCase() !== want && m[1] !== String(real)) {
    note(`README.md  says "${m[1].toLowerCase()} ${noun}"; ${source} (${want})`);
  }
}

const readme = read("README.md");
if (readme) {
  // Anchored on the Layout row's own words, not on the bare noun: README talks
  // about registers in prose too, and `exec` takes the first match in the file.
  readmeCount({
    text: readme.text,
    after: "registers — questions",
    shown: "registers — questions, problems, tasks, inbox",
    noun: "registers",
    real: REGISTERS.length,
    source: `there are ${REGISTERS.length}`,
  });

  const scriptsDir = join(root, "scripts");
  const real = existsSync(scriptsDir)
    ? readdirSync(scriptsDir).filter((f) => /^check-.*\.mjs$/.test(f)).length
    : 0;
  if (real === 0) {
    checks++;
    note(`${scriptsDir}  holds no check-*.mjs — cannot verify README.md's count of the checks`);
  } else {
    // Anchored on the parenthetical, which is what makes the Layout row the
    // sentence being checked. "checks", not "doc checks": check-workspace-lints
    // reads Cargo manifests. It runs in the fast half because it needs no
    // toolchain, which is a scheduling fact and not a reason to call it a doc
    // check in the one place that counts them.
    readmeCount({
      text: readme.text,
      after: "checks \\(`check-\\*\\.mjs`\\)",
      shown: "checks (`check-*.mjs`)",
      noun: "checks",
      real,
      source: `scripts/ holds ${real}`,
    });
  }
}

// ── history/README.md describes the answered-question file completely ───────
const hist = read(join("docs", "history", "README.md"));
if (hist) {
  checks++;
  if (!/Outcome/i.test(hist.text)) {
    note(
      `${hist.path}  does not mention the Outcome section — a writer following its ` +
        `layout table writes a file CI rejects`,
    );
  }
  for (const reg of REGISTERS) {
    checks++;
    if (!hist.text.includes(`${reg.dir}/`)) {
      note(`${hist.path}  omits the ${reg.dir}/ directory`);
    }
  }
  // It states the cap by hand too, exactly as CLAUDE.md does, and only
  // CLAUDE.md's copy was guarded.
  checks++;
  const capKb = `${MAX_BYTES / 1024} KB`;
  if (!hist.text.includes(capKb)) {
    note(`${hist.path}  does not state the size cap as "${capKb}" — the checker enforces that number`);
  }
}

// ── design-invariants.md names the registers it is written in terms of ──────
// Every invariant here is phrased as `Q<n>`/`P<n>`/`T<n>`/`I<n>` against the four
// live docs, so a register renamed, relocated or re-prefixed leaves this file
// describing a scheme that no longer exists — in the document a reviewer reads
// to decide what to check. It is also the file that already drifted once, with a
// citation form the scheme cannot express.
const inv = read(join(".claude", "design-invariants.md"));
if (inv) {
  for (const reg of REGISTERS) {
    checks += 2;
    if (!inv.text.includes(`docs/${reg.live}`)) {
      note(`${inv.path}  does not name docs/${reg.live}, where open ${reg.what} live`);
    }
    if (!inv.text.includes(`\`${reg.prefix}<n>\``)) {
      note(
        `${inv.path}  does not name the \`${reg.prefix}<n>\` entry prefix for ${reg.what} — ` +
          `an invariant written against a prefix the registers do not use checks nothing`,
      );
    }
  }
}

// ── The ⚠HASH marker is named wherever it is defined ───────────────────────
// Three docs define it by hand — CLAUDE.md's register table, TASKS.md's
// conventions, and design-invariants' DI7 — and they went out of step: two said
// "a task" while QUESTIONS.md used it as well, and the repair widened all three
// at once into a marker that selected nearly every open question. The rule that
// only one register may carry it is enforced by check-registers; what is checked
// here is that the docs which teach it still name it, so a rename cannot leave
// three explanations of a marker the corpus no longer uses.
{
  const owner = REGISTERS.find((r) => r.what === HASH_MARKER.what);
  const defining = ["CLAUDE.md", join("docs", owner.live), join(".claude", "design-invariants.md")];
  for (const rel of defining) {
    const doc = read(rel);
    if (!doc) continue;
    checks++;
    if (!doc.text.includes(HASH_MARKER.text)) {
      note(
        `${doc.path}  does not name the ${HASH_MARKER.text} marker it explains — ` +
          `check-registers allows it in docs/${owner.live} and rejects it in every other register`,
      );
    }
  }
}

// ── CLAUDE.md teaches the breadcrumb check-doc-layout actually demands ─────
// The two disagreed, in the file that owns the convention: CLAUDE.md gave one
// formula for every part file, and the checker builds the crumb from the
// immediate parent directory — which it documents and supports at any depth. A
// nested part written to CLAUDE.md's letter fails CI. What is checked here is
// only that the string CLAUDE.md prints is one the checker would accept; the
// prose around it has to say which doorway, and no check can read that.
{
  const doc = read("CLAUDE.md");
  if (doc) {
    checks++;
    const want = breadcrumb(TEMPLATE_PARENT);
    if (!doc.text.includes(want)) {
      note(
        `CLAUDE.md  does not print the breadcrumb check-doc-layout.mjs enforces — it must ` +
          `show ${want}, or a part file written from CLAUDE.md is rejected by CI`,
      );
    }
  }
}

if (problems.length) {
  console.error(`✗ ${problems.length} vocabulary drift(s) between the docs and the tooling:\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

console.log(`✓ ${checks} vocabulary facts agree with scripts/lib/`);
