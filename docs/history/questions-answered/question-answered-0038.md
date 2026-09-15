*Closed record — see [../README.md](../README.md). Not spec.*

# Q38 — What may the player do while stalled on a peer, or after a desync?

## Ruling

*Ruling (2026-09-12):* **wait or resign; on a desync, the match freezes
with the report and the replay saved, and the player leaves.** A stall
past `stall_report_ticks` (`06`) is a banner over the map naming the peer
awaited, the tick, and the seconds waited, with a resign button beside it;
everything else stays usable. A desync is a red banner with the report —
the tick, this peer's hash, the other's — the replay saved as Q36 names it,
and a *leave* button to the end screen, which says the match ended in a
desync.

### The rules

1. **A stall changes nothing but the banner.** The driver already issues
   no tick (Q12); the windows, the editor and the marks keep working, and a
   submission lands on the first tick still open, as the driver rules. The
   banner shows the peer, the tick and the wait, and clears when the set
   arrives; the resign beside it is the ordinary `Resign` (Q10), agreed
   `delay` ticks after the stall ends — which the player is told.
2. **A desync freezes the match** where the driver stopped it (`06`): no
   further tick, the windows still readable, the report in a red banner,
   the replay written at once to Q36's path with the desync tick and this
   peer's hash in its name, and the path in the banner. *Leave* goes to
   the end screen; nothing else leaves it.
3. **A lost peer is neither**: its sets are no longer awaited and the
   match continues, with a line in the log, as the driver already does.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — The match, and the
  Decided entry. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T21](../../TASKS.md) — the banners, the resign, the freeze.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Banner with resign; freeze on desync** *(chosen)* | A banner and two buttons; the stall stays what `06` made it and the desync leaves its evidence behind. |
| Wait only | The resign is a window away in the tools; the banner would only be information. Cheap, and the player's one real choice during a stall is hidden. |
| Drop a late peer after a timeout | Every peer must drop it on the same tick, which is a new lockstep command and a `06` rule — a widening PvP may want (Q12's rate-limiting hook is where it would go), not an interface answer. |

### How the answer took its shape

The only temptation was the timeout, and it is not the renderer's to take:
whatever the interface shows, the sim's rule is that a missing set stops
every peer, and changing that is a question for `06` under its own number.
