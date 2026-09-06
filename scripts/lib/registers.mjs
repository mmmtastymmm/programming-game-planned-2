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
  },
  {
    what: "questions",
    prefix: "Q",
    live: "QUESTIONS.md",
    dir: "questions-answered",
    filePrefix: "question-answered",
    status: "dated",
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
  },
  {
    what: "inbox",
    prefix: "I",
    live: "INBOX.md",
    dir: "inbox-triaged",
    filePrefix: "inbox-triaged",
    status: null,
    // `Dropped` exists here and nowhere else: an inbox that cannot absorb a
    // false alarm stops being cheap to write to, and then it goes unused.
    outcome: ["Docs", "Question", "Problem", "Task", "Dropped"],
  },
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
const FIELDS = ["what", "prefix", "live", "dir", "filePrefix", "status", "outcome"];
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
