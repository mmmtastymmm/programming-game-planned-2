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

import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { MAX_BYTES, OUTCOME_KINDS, REGISTERS } from "./lib/registers.mjs";

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

// ── Each history README enumerates exactly its register's Outcome kinds ─────
for (const reg of REGISTERS.filter((r) => r.outcome)) {
  const doc = read(join("docs", "history", reg.dir, "README.md"));
  if (!doc) continue;
  for (const kind of ALL_KINDS) {
    checks++;
    const mentioned = doc.text.includes(`**${kind}:**`);
    const allowed = reg.outcome.includes(kind);
    if (allowed && !mentioned) {
      note(
        `${doc.path}  does not document the "${kind}:" outcome, which the checker ` +
          `accepts here — a writer following this README will not know it exists`,
      );
    }
    if (!allowed && mentioned) {
      note(
        `${doc.path}  documents the "${kind}:" outcome, which the checker rejects ` +
          `for the ${reg.what} register`,
      );
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

// ── The front door counts the registers correctly ───────────────────────────
const readme = read("README.md");
if (readme) {
  const WORDS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight"];
  const want = WORDS[REGISTERS.length] ?? String(REGISTERS.length);
  checks++;
  // Built from WORDS, not hand-listed: a hand-listed subset made the check
  // unsatisfiable for any count outside it — README would say "seven registers"
  // and the checker would insist it did not.
  const COUNT = new RegExp(`\\b(${WORDS.join("|")}|\\d+)\\s+registers\\b`, "i");
  if (COUNT.test(readme.text)) {
    const said = COUNT.exec(readme.text)[1].toLowerCase();
    if (said !== want) {
      note(`README.md  says "${said} registers"; there are ${REGISTERS.length} (${want})`);
    }
  } else {
    note(`README.md  does not say how many registers there are — expected "${want} registers"`);
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
}

if (problems.length) {
  console.error(`✗ ${problems.length} vocabulary drift(s) between the docs and the tooling:\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

console.log(`✓ ${checks} vocabulary facts agree with scripts/lib/registers.mjs`);
