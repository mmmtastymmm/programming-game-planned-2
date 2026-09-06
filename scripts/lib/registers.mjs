// THE authoritative description of the register scheme. `check-registers.mjs`
// enforces it; `check-vocabulary.mjs` checks that the docs describing it agree
// with it.
//
// This file exists because the docs and the tooling drifted apart five separate
// times in one review round: a directory README omitted an Outcome kind the
// checker accepts, INBOX.md said "exactly one of" where the checker allows
// several, design-invariants documented a citation form the scheme forbids,
// history/README omitted a mandatory section, and README.md advertised three
// registers when there were four. Every one of those is a fact about the
// tooling, restated by hand somewhere else — which is the same failure the
// derived-counts rule exists to prevent, one level up.

/** Roughly 10k tokens. The threshold is arbitrary; having one is not. */
export const MAX_BYTES = 40 * 1024;

export const REGISTERS = [
  {
    what: "problems",
    prefix: "P",
    live: "PROBLEMS.md",
    dir: "problems-fixed",
    filePrefix: "problem-fixed",
    status: "counted",
    outcome: null,
    closingCommit: true,
  },
  {
    what: "questions",
    prefix: "Q",
    live: "QUESTIONS.md",
    dir: "questions-answered",
    filePrefix: "question-answered",
    status: "dated",
    // A ruling closes with its Outcome, not with a commit: what it changed is
    // the record, and the commit that carried the edit is git's business.
    closingCommit: false,
    // No `Dropped`: a ruling that changes nothing anywhere is a forgotten
    // propagation, which is the failure the Outcome section exists to catch.
    outcome: ["Docs", "Question", "Problem", "Task"],
  },
  {
    what: "tasks",
    prefix: "T",
    live: "TASKS.md",
    dir: "tasks-completed",
    filePrefix: "task-completed",
    status: null,
    outcome: null,
    closingCommit: true,
  },
  {
    what: "inbox",
    prefix: "I",
    live: "INBOX.md",
    dir: "inbox-triaged",
    filePrefix: "inbox-triaged",
    status: null,
    closingCommit: false,
    // `Dropped` exists here and nowhere else: an inbox that cannot absorb a
    // false alarm stops being cheap to write to, and then it goes unused.
    outcome: ["Docs", "Question", "Problem", "Task", "Dropped"],
  },
];

/**
 * The marker for an entry that changes sim behavior, and the ONE register that
 * may carry it. It appeared in QUESTIONS.md for a while, and keeping it there
 * meant widening its definition in three docs at once — at which point it
 * selected almost nothing, since nearly every open question changes sim
 * behavior while the sim is unbuilt. A marker that marks everything is not a
 * marker.
 */
export const HASH_MARKER = { text: "⚠HASH", what: "tasks" };

/**
 * The first line of every file under `docs/history/`, except that directory's
 * own README.
 *
 * It is what tells a reader who arrived mid-file — from a grep, from a link —
 * that they are in a closed record rather than in spec, which CLAUDE.md calls
 * the load-bearing convention of the whole split. Sixteen files carried it by
 * imitation, documented nowhere and checked by nothing; deleting it from one
 * passed every check clean.
 */
export const HISTORY_BANNER = "*Closed record — see [../README.md](../README.md). Not spec.*";

/**
 * Number words, for the counts the docs state in prose.
 *
 * One list, because two of them already drifted: check-vocabulary carried a copy
 * stopping at "eight" and another stopping at "ten", which made a check
 * unsatisfiable for any count outside the shorter list. check-registers parses
 * PROBLEMS.md's headline with one and matches restated totals with another.
 */
export const NUMBER_WORDS = [
  "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
  "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
  "seventeen", "eighteen", "nineteen", "twenty",
];

/** Which register each Outcome kind cites, and how its citations are spelled. */
export const OUTCOME_KINDS = {
  Question: { what: "questions", re: /\b(Q(\d+))\b/g },
  Problem: { what: "problems", re: /\b(P(\d+))\b/g },
  Task: { what: "tasks", re: /\b(T(\d+))\b/g },
  Docs: null,
  Dropped: null,
};

// ── Every field is declared, `null` where it does not apply ─────────────────
// Both consumers gate on truthiness (`if (reg.outcome)`, `filter(r => r.status)`),
// so an OMITTED key reads exactly like a deliberately disabled check. Deleting
// `outcome` from the inbox register did not fail anything: it silently retired
// the `Dropped` rule AND check-vocabulary's agreement check for it, behind three
// green ticks and a fact count that quietly dropped by five. Requiring the key
// turns that omission into a crash on import, which is the only outcome nobody
// mistakes for a pass.
const FIELDS = ["what", "prefix", "live", "dir", "filePrefix", "status", "outcome", "closingCommit"];
for (const reg of REGISTERS) {
  for (const key of FIELDS) {
    if (!(key in reg)) {
      throw new Error(
        `lib/registers.mjs: register "${reg.what ?? "?"}" omits "${key}" — declare every ` +
          `field, using null where it does not apply; an absent key disables a check silently`,
      );
    }
  }
}

// ── The claim patterns, each with an id ─────────────────────────────────────
// These are enforcement detail, and they live here anyway, exported and NAMED,
// for one reason: check-checks.mjs requires a seeded defect per PATTERN rather
// than per check. Its coverage guard asks only whether a check has ANY mutation,
// which for `registers` is satisfied by any one of sixty — so three rules shipped
// with no mutation at all, and two of those were still digit-only long after
// NUMBER_WORDS existed for exactly the case they missed. A list the meta-check
// can iterate is what turns "adding a check means adding mutations" from a thing
// somebody remembers into a thing that fails.
//
// Add a pattern here and the meta-check demands a mutation tagged with its id.

const NUM = `\\d+|${NUMBER_WORDS.join("|")}`;
const NOUN = "questions?|tasks?|problems?|entries";

/**
 * Someone other than PROBLEMS.md's derived headline stating a register total.
 *
 * The word spellings are not decoration. PROBLEMS.md's own headline reads
 * "0 opened, 0 fixed — zero open", so a digit-only pattern waves through the
 * word spelling of the very sentence it protects — which is what the first and
 * third entries did, next to three siblings built on NUMBER_WORDS.
 */
export const TOTAL_CLAIMS = [
  { id: "opened-fixed", re: new RegExp(`\\b(?:${NUM}) opened, (?:${NUM}) fixed\\b`, "i") },
  // `carries` and `has` sit beside `holds` because a fourth entry used to spell
  // one of them out on its own — `problem register carries <word> open entr`,
  // one literal sentence from the predecessor corpus. Widening this pattern to
  // the number words subsumed every spelling of it that names a count, and no
  // probe could distinguish the two any more, so it went: a rule that cannot be
  // made to fire alone is a rule no mutation can pin.
  { id: "register-holds", re: new RegExp(`\\bregister (?:carries|holds|has) (?:${NUM})\\b`, "i") },
  // "Eight questions are open." is the same hand-maintained derived value as a
  // problem-register total, in the file the tooling is supposed to protect. It
  // drifted twice in the session that introduced it. The adverb slot is there
  // because "three questions STILL remain open" is the same claim.
  { id: "count-noun-verb",
    re: new RegExp(`\\b(?:${NUM})\\s+(?:${NOUN})\\s+(?:\\w+\\s+)?(?:are|is|remain|remains)\\b`, "i") },
  // The same total with the adjective in front of the noun instead — "there are
  // 8 open questions right now" — which the shape above cannot see, because it
  // wants the numeral adjacent to the noun.
  { id: "count-adjective-noun",
    re: new RegExp(`\\b(?:${NUM})\\s+(?:\\w+[\\s-]+)?(?:open|closed|answered|fixed|triaged|remaining)` +
      `\\s+(?:${NOUN})\\b`, "i") },
  // And with the label first — "Open: 8 questions, 2 problems."
  { id: "label-first",
    re: new RegExp(`\\b(?:open|closed|answered|fixed|triaged)\\b\\s*:\\s*(?:${NUM})\\s+(?:${NOUN})\\b`, "i") },
];

// Up to two words of adverb or determiner between the copula and the state word.
// It was two hard-coded literals, `already` and `since`, so `Q14 is now answered`
// and `Q14 has finally been ruled` both passed against an OPEN question — the
// direction this file's own comment calls the worse one — while the sibling
// TOTAL_CLAIMS pattern one screen away carried a general slot for exactly this.
// Commas are in the separators because a short parenthetical is a filler too:
// `Q14 is, finally, answered`.
//
// A negation may NOT fill the slot, and that is the whole reason the slot is not
// a plain `\w+`: "Q13 is not yet answered" is a claim that Q13 is OPEN, and a
// filler that swallowed `not` would report it as a claim of closure — inventing
// a defect, in the opposite direction, out of a sentence that is true. The two
// negated shapes get their own entries below instead, where they read as what
// they are.
const GAP = String.raw`[\s,]+`;
const FILLER = String.raw`(?:(?!(?:not|never|no|hardly|barely)\b)\w+${GAP}){0,2}`;

// The copulas are spelled out rather than left at `is|are`, because the corpus
// writes the other spellings and they were each a silent pass: `Q13 stays open
// for now` and `Q13 has been open since the spike` both went green against a
// CLOSED question, which is the citation-resolves-while-meaning-the-opposite
// case DI9 calls the one to hunt. Present tense only in the open direction —
// "Q13 was open at the time" is a true sentence about a question that has since
// closed, and history/ is not the only place someone may write one.
//
// The perfect tenses carry the filler INSIDE them (`has finally been ruled`), so
// the adverb slot appears twice: after the auxiliary and after the whole copula.
const COPULA_OPEN = String.raw`(?:is|are|stays?|remains?|stands?|(?:has|have)\s+${FILLER}been)`;
const COPULA_CLOSED = String.raw`(?:is|are|was|were|(?:has|have|had)\s+${FILLER}been|got|gets?)`;
const NEGATION = String.raw`(?:not(?:\s+yet)?|never|no\s+longer)`;
// `open` alone is how CLAUDE.md phrases it and not how prose does: `remains
// undecided`, `is unresolved`, `is still outstanding` are the same claim, and an
// article or an adjective in front of the word — "is an open question" — hid the
// commonest spelling of all behind a pattern that wanted it adjacent to the
// copula.
const OPEN_STATE = String.raw`(?:open|undecided|unanswered|unresolved|unsettled|outstanding)`;
// `resolved` and `fixed` earn their place: `fixed` is the word the PROBLEMS
// register uses for its own closed state, so the register with the most to lose
// from a false closure claim was the one that could not express it.
const CLOSED_STATE = String.raw`(?:answered|settled|decided|ruled|closed|resolved|fixed)`;
/** `^[^.]{0,80}?` — the claim has to be in the same sentence as the citation. */
const sameSentence = (body) => new RegExp(String.raw`^[^.]{0,80}?\b` + body, "i");

/**
 * A live doc claiming a cited entry is open, or claiming it is closed.
 *
 * `side` says which half of the sentence the pattern reads: `before` is the text
 * ahead of the citation ("open — Q12", the phrasing CLAUDE.md sanctions),
 * `after` is the text behind it ("Q12 is still open", the phrasing the corpus
 * actually writes). Matching only one ordering means the check goes green on the
 * day it was written for.
 *
 * One id per SHAPE, not per direction: an alternation inside a single pattern is
 * exactly the granularity the meta-check cannot see into, which is how the holes
 * these entries close got in.
 */
export const CITATION_CLAIMS = [
  { id: "open-before", claims: "open", side: "before", re: /\bopen\s*[—–-]\s*$/i },
  // "is still open", "remains undecided", "is an open question".
  { id: "open-after", claims: "open", side: "after",
    re: sameSentence(String.raw`${COPULA_OPEN}${GAP}${FILLER}${OPEN_STATE}\b`) },
  // "is not yet answered" — a negated closure is a claim of openness, and it is
  // invisible to `closed-after`, which refuses a negation in its filler
  // precisely so it does not read this sentence as its own opposite.
  { id: "open-after-negated", claims: "open", side: "after",
    re: sameSentence(String.raw`${COPULA_OPEN}${GAP}${NEGATION}${GAP}${FILLER}${CLOSED_STATE}\b`) },
  // The other direction, which is the worse one: a live doc asserting a ruling
  // that was never made. QUESTIONS.md is the only place an undecided thing may
  // live, so "settled in Q14" inside a Decided section is the stalest possible
  // text in the most authoritative-looking place — the failure the whole register
  // scheme is built around.
  { id: "closed-before", claims: "closed", side: "before",
    re: new RegExp(String.raw`\b${CLOSED_STATE}\s*(?:in|by|as)?\s*[—–:-]?\s*$`, "i") },
  { id: "closed-after", claims: "closed", side: "after",
    re: sameSentence(String.raw`${COPULA_CLOSED}${GAP}${FILLER}${CLOSED_STATE}\b`) },
  // And its mirror: "is no longer open" asserts a closure as surely as "is
  // answered" does.
  { id: "closed-after-negated", claims: "closed", side: "after",
    re: sameSentence(String.raw`${COPULA_OPEN}${GAP}${NEGATION}${GAP}${FILLER}${OPEN_STATE}\b`) },
];
