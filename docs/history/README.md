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

## Sharding

Closed records are **split into numbered shards, and the shards are bounded**.
`scripts/check-registers.mjs` fails any file over 40 KB, so filling one is what
tells you to start the next — nobody has to notice.

| Family | Shards | Holds |
|---|---|---|
| Answered questions | `questions-answered-NNN.md` | One ruling per answered question. The thing that gets cited. |
| Worksheets | `questions-worksheets-NNN.md` | The argument behind a ruling — options, numbers, paths not taken. Largest files here; split hardest. |
| Fixed problems | `problems-fixed-NNN.md` | Register entries closed out of `PROBLEMS.md`, with the commit that closed them. |
| Status blocks | `questions-status-log-YYYY.md` | Dated board states displaced from `QUESTIONS.md`. Sharded by year — blocks are dated, not numbered. |
| Milestones | [tasks-completed.md](tasks-completed.md) | Milestones with no open items. One file; milestones are few. |

**There is no index table, deliberately** — one would be a hand-maintained
derived value, which is the thing this corpus keeps getting wrong. Each shard's
H1 states the range it actually holds, derived and machine-checked. The headings
*are* the index:

```sh
grep -H '^# ' docs/history/*.md
```

Shards fill in order: every number in shard `002` is above every number in
`001`. So finding `Q73` means opening the one shard whose heading covers it, and
nothing else gets read.

## Working rules

- **Answering a question writes here.** The ruling goes to the current
  `questions-answered-NNN.md`; the question's worksheet body goes to the current
  `questions-worksheets-NNN.md`; the status block it displaces from
  `QUESTIONS.md` goes to the top of this year's status log. Then the question
  leaves `QUESTIONS.md` entirely — it is answered, so it is not open.
- **Fixing a problem writes here.** The entry moves out of `PROBLEMS.md` into
  the current `problems-fixed-NNN.md` with its closing commit. The register's
  `fixed` count is derived from these shards, so an entry left behind in the
  live register is still open as far as CI is concerned.
- **Blocks arrive as they were written.** Moving a block here changes exactly
  two things: it loses any `(latest)` marker, and its relative links gain a
  `../` because the archive sits a directory deeper. Not the prose, not the
  counts, not the ordering.
- **Numbers are never reused.** A shard is append-only once closed; an amended
  ruling is a new number, not an edit.
