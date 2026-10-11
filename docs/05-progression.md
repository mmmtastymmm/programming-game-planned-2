# 05 — Progression

What a player gains as a match goes on, and what carries between matches.
This doc is short by design: progression in this game happens inside a
match, it is made of rules that [02-machines](02-machines.md) owns, and
nothing persists when the match ends.

## Decided

- **Progression is inside a match, and it is building printers (Q42,
  replacing Q30's conversion and Q31's fixed count).** A team's bots build
  printers for ore, each one dearer than the last by how many printers and
  printer sites the team has, and each one standing raises the team's cap
  by `bots_per_printer`, which its printers fill by themselves. No printer
  changes hands. A printer defends itself, dealing `defence_damage` each
  tick to every adjacent bot of another team (Q30), so taking one down is
  a fight. There is no progression between matches and none is planned;
  the ladder of opponents (Q29) is the only thing that carries from one
  match to the next, and it carries nothing but the player's own programs.

## Inside a match

A team's reach is measured in printers. Each one it holds is
`bots_per_printer` more bots its printers will print (Q24). There is one
way up and two ways down:

| Way | Costs | Ruled in |
|---|---|---|
| **build** one | a site of `cost + n × cost_step` ore, `n` the team's printers and printer sites when it is placed, hauled by bots, then `build_ticks` | Q42, [02-machines](02-machines.md) |
| **lose** one to a rival's attacks | the rival's bots, within `attack_range`, taking `defence_damage` every tick they stand adjacent | Q25, Q30 |
| **lose** one to a rival's deconstruction | a rival's bot adjacent to it for its `deconstruct_ticks`, taking the same | Q28, Q30 |

**The price rises with what a team has, not with what it ever built.** A
team that loses printers pays less for the next one, so a beaten-down team
can rebuild faster than it was knocked down — the comeback is the rule's,
on purpose. A team far ahead pays more for each printer than it did for the
last, so a lead costs more to extend than to hold.

**The last printer is not replaceable.** A team that ends a tick with no
printer is out (`04`), whatever site it was building, so a rebuild must
start while a printer still stands. Defending the last one is the one thing
a program cannot leave to later.

## Between matches

Nothing. A match is `(map, command log)` (`06`) and ends with a winner or
a draw (`04`); the next match starts from its own map with nothing carried
over but what the player learned and the programs they wrote, which live in
their programs directory, exported from the game's editor (Q33, `07`), and
not in the match. The tarot ladder (Q29) is the sequence a
player is meant to climb, and climbing it is a matter of beating each card,
not of unlocking the next.

A campaign, a profile, unlocks or any other state that outlives a match is
a **new question number**, and this doc grows a section when one is ruled.
It is deferred rather than rejected: the design is starting small, and a
profile is the first thing that would not live in the sim.

## What this doc leaves open

- **Between-match progression**, under a new number when wanted.
- **The next card** of the ladder (Q29), which is `04`'s content and this
  doc's only external structure.
