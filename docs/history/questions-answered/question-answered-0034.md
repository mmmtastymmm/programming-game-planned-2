*Closed record — see [../README.md](../README.md). Not spec.*

# Q34 — How does a fault reach the player?

## Ruling

*Ruling (2026-09-12):* **on every channel.** A mark over the machine while
its fault record is fresh; a per-deployment summary in the editor's panel
counting the latest records by file and line; the full record in the
inspector for the selected machine; and a line in the log for every new
record. The record is the sim's (Q8, Q18, Q20); the renderer shows it and
adds nothing to it.

### The rules

1. **The mark** shows for `fault_mark_ticks` after the record's tick, a
   tuning constant in `data/interface.toml`.
2. **The summary** groups each deployment's machines' latest records by
   file and line, newest first, with the version each names, and a line
   clears when those machines fault again or die — not when the file is
   edited, since a record survives a redeploy (Q20).
3. **The inspector** shows the whole record and the version it ran against
   the deployment's running version.
4. **The log** gets one line per new record.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Faults section
  and its Decided entry. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T17](../../TASKS.md) — the four channels.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| The log only | Cheapest; a fault in a fleet of forty is one line among hundreds, and the player learns of it late or never. |
| The mark only | Immediate on the map, but says nothing about which line, and vanishes with the machine. |
| The summary only | Names the line, but the player watching the map sees nothing happen to the bot that faulted. |
| **All four** *(chosen)* | Four views of one record, each cheap once the record is in the snapshot, and none of them a second source of truth. |

### How the answer took its shape

The record was made world state (Q20) precisely so any renderer could show
it; the question was only which surfaces. Each of the four answers a
different question a player asks — *where*, *which line*, *this one*, *when*.
