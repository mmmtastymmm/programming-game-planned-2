# 05 — Progression

What a player gains as a match goes on, and what carries between matches.
This doc is short by design: progression in this game happens inside a
match, it is made of two rules that [02-machines](02-machines.md) owns, and
nothing persists when the match ends.

## Decided

- **Progression is inside a match, and it is printers changing hands
  (Q30).** A bot may convert an adjacent printer of another team, after which
  the printer is its team's: it prints for them, and by Q24's rules its new
  team's cap rises by ten and its next color unlocks while the old team's cap
  falls and its last color locks. A printer defends itself, dealing one
  damage each tick to every adjacent bot of another team, so a conversion is
  a fight. There is no progression between matches and none is planned; the
  ladder of opponents (Q29) is the only thing that carries from one match to
  the next, and it carries nothing but the player's own programs.

## Inside a match

A team's reach is measured in printers. Each one it holds is ten more bots
it may print and one more deployment it may write for (Q24), and there are
three ways to hold more:

| Way | Costs | Ruled in |
|---|---|---|
| **build** one | ore, hauled to a site by bots, and the build time | Q9, Q22, Q23 |
| **convert** a rival's | a bot adjacent to it for `convert_ticks`, taking `defence_damage` every tick from the printer it is converting | Q30, [02-machines](02-machines.md) |
| **keep** the ones it has | bots to stand between them and the rival's, and a program that knows when to | Q25, Q30 |

Conversion is the progression rule, and it is the reason a match has an
arc: a printer taken is ten bots and a color taken, and a printer taken
back is the same in reverse. Deconstruction (Q28) removes a rival's printer
instead of taking it, which is faster and gains nothing; a program chooses.

**The arithmetic the numbers are set for.** With `convert_ticks` and bot
health both at ten and `defence_damage` at one, one full-health bot converts
a printer with nothing to spare; two bots do it comfortably; a bot already
hurt dies trying. A team that wants a rival's printer brings more than one
bot, which is a program's decision, which is the point.

**What conversion does not change.** The printer keeps its store, its
health, its name and a print in progress; it takes the new team's printer
bundle at its next boundary (Q11), and the bot it was printing is the new
team's. Its old team is out (`04`) if it was the last one.

## Between matches

Nothing. A match is `(map, command log)` (`06`) and ends with a winner or
a draw (`04`); the next match starts from its own map with nothing carried
over but what the player learned and the programs they wrote, which live in
their editor and not in the game. The tarot ladder (Q29) is the sequence a
player is meant to climb, and climbing it is a matter of beating each card,
not of unlocking the next.

A campaign, a profile, unlocks or any other state that outlives a match is
a **new question number**, and this doc grows a section when one is ruled.
It is deferred rather than rejected: the design is starting small, and a
profile is the first thing that would not live in the sim.

## What this doc leaves open

- **Between-match progression**, under a new number when wanted.
- **Q29**, the ladder itself, which is `04`'s content and this doc's only
  external structure.
