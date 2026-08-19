# docs/history — closed records

Everything in this directory is **history, not spec**. It records what was
decided, built, or found *at a point in time*. Entries here are expected to
contradict the current design where the design has since moved on — that is not
a defect, and it is not something to fix.

The authority on current design is always the live corpus: the numbered docs for
rules, [QUESTIONS.md](../QUESTIONS.md) for what is still open,
[PROBLEMS.md](../PROBLEMS.md) for known-wrong text, [TASKS.md](../TASKS.md) for
what is left to build.

**A normal doc pass does not read this directory.** That is the whole point of
the split: a closed ruling is read once, by whoever needed it, and never again.
Keeping it inline meant every reader of the live register paid for every ruling
ever made — the predecessor project's `QUESTIONS.md` reached 166 KB, almost all
of it answered. Grepping here is fine, and a hit for a retired term is usually
correct history to leave alone.

## Layout: one file per entry

| Directory | Files | Holds |
|---|---|---|
| [questions-answered/](questions-answered/README.md) | `question-answered-0013.md` | The ruling for Q13, its **Outcome**, then the worksheet |
| [problems-fixed/](problems-fixed/README.md) | `problem-fixed-NNNN.md` | the entry as recorded, and the commit that closed it |
| [tasks-completed/](tasks-completed/README.md) | `task-completed-0003.md` | T3 and the commit that finished it |
| [inbox-triaged/](inbox-triaged/README.md) | `inbox-triaged-NNNN.md` | the observation as written, and where it went |

There is deliberately **no status log**. An earlier version archived the dated
status blocks displaced from `QUESTIONS.md`, and they turned out to restate what
was already authoritative elsewhere — which rulings existed (`ls` answers that),
what they said (each has its own file), and what was still open (`QUESTIONS.md`
itself). `git log -p docs/QUESTIONS.md` records every status block ever written,
dated and attached to its commit, which is a stricter record than a
hand-maintained archive and cannot be forgotten.

**The filename is the index.** Finding Q73 means opening
`questions-answered/question-answered-0073.md` and nothing else — no table to
consult, and no table to go stale. `ls` is the table of contents:

```sh
ls docs/history/questions-answered/
```

This replaced an earlier scheme that batched entries into numbered shards. That
scheme needed three rules to stay straight — shards fill in order, each shard's
H1 states the range it holds, an empty shard must be the last one — and all three
existed only because a shard held many entries. One file per entry deletes the
category. It also removed a failure already approaching: the first shard reached
24 KB at six entries and would have needed splitting at about ten.

`scripts/check-registers.mjs` enforces what the scheme depends on: a file's name
and its heading must agree, a file holds exactly one entry, numbering is dense,
and nothing exceeds 40 KB.

## Working rules

- **Answering a question writes here.** The ruling and its worksheet become one
  new file in `questions-answered/`. Then the question leaves `QUESTIONS.md`
  entirely — it is answered, so it is not open. Its status block is simply
  rewritten; nothing is displaced.
- **Fixing a problem writes here.** The entry becomes a file in
  `problems-fixed/` with its closing commit. The register's `fixed` count is
  derived from that directory, so an entry left behind in the live register is
  still open as far as CI is concerned.
- **Relative links gain a `../` on the way in.** This directory sits a level
  below `docs/`, and its entry files another level below that. Moving text here
  silently deepens every relative link it contains; `scripts/check-links.mjs`
  exists because that rot is invisible in review.
- **Numbers are never reused.** A file here is append-only once written; an
  amended ruling is a new number and a new file, not an edit.
