*Closed record — see [../README.md](../README.md). Not spec.*

# Q60 — Amend Q36 and Q55: is a saved replay tied to the version of the game that wrote it?

## Ruling

*Ruling (2026-10-15):* **yes: a saved replay records the version that
wrote it and its final state hash; one from another version opens with a
warning and plays anyway, and a playback that ends on a different hash
says so.** Made under a new number because the rulings it amends,
[Q36](question-answered-0036.md) on the replay always saved and
[Q55](question-answered-0055.md) on what a replay file holds, are closed
records. Raised by [I3](../inbox-triaged/inbox-triaged-0003.md): every
hash-changing change since — T30's `while` charge, Q59's power — makes an
old replay play back differently, and nothing told the player.

### The rules

None reaches the state hash: the version and the final hash are outside a
replay's identity, as the saver's team is (Q55).

1. **A saved replay records the game's version** — the version string of
   the build that wrote it — and **its final state hash**, beside the
   saver's team, outside the replay's identity.
2. **A replay from another version** — one whose recorded version is not
   this build's, or that records none, being older than this ruling —
   opens with a warning naming the version that wrote it, and plays
   anyway. No old rules are kept: a replay plays on this build's rules.
3. **The final hash is checked.** Whatever the version, when the playback
   reaches the replay's last tick, a state hash that differs from the
   recorded one is reported in the time bar: this build played it back
   differently.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Q36 Decided
  entry and The match. [06-architecture.md](../../06-architecture.md) — the
  command log's replay identity. [00-overview.md](../../00-overview.md) —
  the Decided listing. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T33](../../TASKS.md) — the version and final hash in the
  file, the warning, and the comparison at the last tick.
- **Problem:** [P9](../../PROBLEMS.md) — the docs state Q60 as current
  until T33 lands.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Tied to its version, with a warning** *(chosen)* | One string and one hash in the file; an old replay may still play back differently, but never silently. |
| Keep every past rule set so old replays replay exactly | A versioned sim and interpreter, kept forever; every rule change carried in two forms. |
| Promised only on the build that wrote it, no warning | Nothing to build; an old replay silently shows a match that never happened. |
| Leave it undecided | The drift continues, and every hash-changing PR adds to it. |

### How the answer took its shape

Triage of I3 during the review of Q59, which changes every hash again,
asked whether a replay must survive a change of rules. The user chose a
replay tied to its version, warned about when opened elsewhere. The final
hash check is the drafting's addition: the file's name already carries
the hash, so comparing it at the last tick costs nothing and catches the
case the version cannot: a build whose rules changed while its version
string did not.
