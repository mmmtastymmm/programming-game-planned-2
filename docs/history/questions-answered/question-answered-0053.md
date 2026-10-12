*Closed record — see [../README.md](../README.md). Not spec.*

# Q53 — Should the starter programs come with a library, and where should it live?

## Ruling

*Ruling (2026-10-11):* **yes: the starter tree ships a library of
ordinary player files under `lib/`, written in the game's language, with a
docstring on every function.** Not game modules: the player can read how
each function works, change it, and replace it.

### The rules

1. **What it holds.** `geo` — positions, directions and distances; `sense`
   — what the team knows of tiles, machines and ore; `act` — each action
   wrapped to return `True` or `False` instead of raising `ValueError`;
   `nav` — `go_to` and `approach`, which use `move_to` while it makes
   progress and search for a route (breadth-first, bounded by a box and a
   tile limit) when the bot is boxed in or pacing; `share` — a bot's rank
   among its team's bots, and splitting a list of jobs by it; `jobs` — the
   starter bot's build, paint and patrol steps, moved out of `bot.py`.
2. **The starter bot uses it.** `robots/bot.py` imports `jobs` and is
   the loop alone, with a comment naming each module.
3. **Docstrings say what a call costs** where it is more than a few
   operations: a query's per-record charge, and a route search's.
4. **It is the player's.** Nothing in the sim or the renderer knows about
   `lib/`; it composes into every bundle as any player file does (Q41).
   Free docstrings (Q52) are what make documenting it cost nothing.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided
  section. [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Player files in the starter tree** *(chosen)* | A player can break their copy, and an improvement reaches new trees only; the library is metered like the player's code, so it is no faster than what they could write. Bought: readable source to learn from, no new determinism surface, no sim change. |
| Game modules (`02`) | Every function a determinism obligation, every change a replay hash, and the source out of the player's sight; better kept for a few hard primitives later. |
| Keep everything in `bot.py` | The starter bot was already over 200 lines, half of it helpers a player would want elsewhere. |

### How the answer took its shape

Asked for after the starter bot was given a docstring: a standard library
with good docstrings, so players can get started. The helpers came from
what the starter bot had built for itself. Route search was the gap a
player hits first, since `move_to` paces in front of a wall forever; a
test shows `go_to` getting round one. With the library the starter test's
jobs finish at tick 152 instead of 143, the price of the calls between
modules.
