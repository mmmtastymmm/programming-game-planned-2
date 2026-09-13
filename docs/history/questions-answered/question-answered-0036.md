*Closed record — see [../README.md](../README.md). Not spec.*

# Q36 — What does the interface show before the first tick and after the last?

## Ruling

*Ruling (2026-09-12):* **a start screen and an end screen, in the game,
and the replay always saved.** Before the first tick the player picks the
map, the opposition cards, the programs directory, and whether to play
alone, host or join; after the last the screen names the winner or the
draw, the ending tick, the final hash and where the replay went. The
command line keeps every option so a headless run and CI need no screen.

### The rules

1. **The start screen** lists the maps in `data/maps` and the cards in
   `data/opposition` (`04`; any number, seated in team order after the
   player's), takes the programs directory (Q33; `data/starter` by default),
   and offers *play* (alone, the cards as the other teams), *host* (an
   address to listen on) and *join* (an address). Starting reads the
   programs in and builds the driver exactly as the command line does
   today; a flag given on the command line skips the screen.
2. **The end screen** shows the winner or the draw (`04`), the ending
   tick, the final state hash, and the replay's path; *again* returns to
   the start screen, *quit* leaves. A desync (Q38) ends there too, saying
   so, with no winner.
3. **The replay is always saved** when a match ends or desyncs:
   `(map, command log)` in `crates/replay`'s RON, as
   `replays/<map>-<tick>-<hash>.ron` inside the programs directory — the
   ending tick and the final hash name it, so two peers' files of one
   match collide only when they agree. It is the report `06` promises and
   what the headless driver replays.
4. **Nothing here reaches a peer**: the screens are the renderer's, and the
   match starts, for every peer, at the handshake `06` rules.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — The match, and the
  Decided entry. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T19](../../TASKS.md) — the screens and the saved replay.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Both screens in the game** *(chosen)* | Two more screens to build; bought: a match a player can start without a terminal, and an end that says what happened and where the evidence is. |
| The end screen only | Half a game: the start is where the cards are chosen (`04`), and a player who cannot choose one cannot climb the ladder (Q29). |
| Neither, command line only | Fine for the developer, and the game stays a developer's tool. |
| Save the replay only on request | A desync report without its replay is nothing (`06`); the one time it matters is the time nobody thought to ask. |

### How the answer took its shape

The replay's file name was the only real choice: a wall-clock name would
be the renderer's to take, but the ending tick and final hash name a match
the way the sim knows it, and two peers that agree write the same name.
