*Closed record — see [../README.md](../README.md). Not spec.*

# I3 — A saved replay from before Q52 and Q53 probably no longer plays back the same

`data/starter/replays/first-993-ec57f7d121fdc338.ron` was saved by a match
played before docstrings became free (Q52) and before the starter bot was
rewritten onto `lib/` (Q53). It is gitignored, so only this checkout has it,
and nothing in CI replays it. A replay that carries its bundles' bytes may
still replay, since the bot's source is in the file, but the interpreter now
charges that source less, so its state hash would likely differ from the one
in its name. Not tried. Worth knowing whether a replay is expected to survive
a language change at all, or is only promised to replay on the build that
wrote it.

## Outcome

- **Question:** [Q60](../questions-answered/question-answered-0060.md) —
  opened and answered: a replay is tied to the version that wrote it. It
  records that version and its final state hash; one from another version
  opens with a warning and plays anyway, and a playback whose final hash
  differs says so. No old rules are kept.
- **Docs:** [07-interface.md](../../07-interface.md) — the Q36 entry and
  The match; [06-architecture.md](../../06-architecture.md) — the command
  log's replay identity.
- **Task:** [T33](../../TASKS.md) — the version and hash in the file, the
  warning and the comparison.
