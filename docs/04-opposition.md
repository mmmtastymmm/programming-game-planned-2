# 04 — Opposition

Who the player fights, and how a match ends. The opposition in PvE is
another team — one the game controls, running programs the game ships,
subject to every rule the player's team is — and PvP is the same match with
a second player on that team. This doc is normative for what a team is, how
a match ends, and what the game's own teams are made of; two implementers
reading it must agree on the tick a match ends (design-invariant DI7).

## Decided

- **The roster and its ladder are Q29's**, elaborated in the PvE section.
- **PvE ships before PvP (Q4).** Lockstep is built now regardless, since it
  is not retrofittable, so deferring PvP costs nothing architecturally and
  buys slack on balance while the sim changes fastest.
- **The roster is the Fool, and grows one card at a time under a new
  question number each (Q29).** The ladder's shape is fixed: each card a
  directory of shipped source and one thing more than the last, named for a
  tarot card chosen when it is written; the player picks any card for any
  map, and any number of them; two scripted teams on one map fight each other
  as they would a player; a card's bundles are the reference programs and
  stay simple; no card is a handicap by data.

Everything the opposition *does* is ruled elsewhere and not repeated here:
what a machine is and how it attacks is [02-machines](02-machines.md), what
it senses and remembers is Q7 and Q21 there, what it stands on is
[03-world](03-world.md), and how its commands reach the sim is
[06-architecture](06-architecture.md).

## Teams

A **team** is a number, from the map's team list (`03`), and everything
that has a team has exactly one. A team owns its machines, its deployments,
its cap (Q24), its plans (Q27) and its memory (Q21); it shares nothing with
another team but the world.

Every team is the same kind of thing. There is no flag on a team that says
"the opposition": what makes a team the opposition is only **who submits
its commands**. A player's team has a player behind its command log; a
**scripted team** has the game behind it. The sim cannot tell the
difference, and nothing in it depends on the difference — which is what
makes PvP a second player and not a second design.

### Scripted teams

A scripted team's commands come from a **script**: a file in
[data/opposition/](../data/opposition/) that says which bundles to deploy to
which deployments on which ticks, which marks to place, and nothing else;
its commands enter the command log like any peer's, so a replay already
holds them and nothing else need be hashed. A
script is a command log written in advance. It may not do anything a player
could not: its bundles are ordinary bundles in the language, its marks are
ordinary marks, and its machines obey every rule.

- **A script is deterministic by construction**: it is a fixed list of
  `(tick, command)` pairs, applied by the driver as if a peer had sent them,
  with the same delay (Q12) as a player's. A scripted team never stalls the
  match, since its sets are always in hand.
- **A script's tiles are relative to its team's first starting tile**,
  resolved at load, so one script fits any map; nothing else in a script
  depends on the map.
- **A script may not read the world.** It reacts to nothing; its programs
  do the reacting, exactly as a player's do. A script that wanted to respond
  to the match would be a player, and the game's response lives in the
  bundles, where the player's does.
- **A scripted team's bundles are shipped source** (design-invariant DI8):
  they are printed in this doc's data, subject to the language reference,
  and the first programs a player will read to learn the game.

## The match

A match is a map, a set of teams, an opening program set for each (Q9), and
a command log that grows until the match ends.

### Ending

**A team is out** on the tick it has no printer — the moment its last
printer's `death` epilogue runs, it is deconstructed (Q28), or it is
**converted** to another team (Q30), with no other printer standing. There
is no site that could become one, since printers are never built (Q31). A
converted printer is not removed with the team that lost it; it is no
longer theirs. An out team's machines are removed, its plans cleared and its
command log closed in the tick's out-teams step (`06`, step 5), after damage
and before regrowth, whatever put it out: further commands from it are dropped at submission. `Resign` (Q10)
puts a team out at once on its agreed tick.

**A match ends** on the first tick at which at most one team is not out.
That team **wins**; if none is — two last printers dying on the same tick —
the match is a **draw**. The driver stops issuing ticks; the state hash of
the ending tick is the match's final hash, and the log to that tick is the
replay.

There is no other way for a match to end: no tick limit, no score, no
objective but the printer. A match with two passive teams runs until a
player resigns. A time limit, if PvP wants one, is a `docs/06` matter and a
new question number.

### Why the printer

The printer is the team's one irreplaceable machine: it is the only source
of bots, and nothing can make one (Q31) — the map placed every printer
there will be. A team with no printer can gain one only by converting a
rival's, and it has no bots left to do it with once its last dies, so it is
out the moment it holds none, not when its last bot dies.

## PvE

The first mode. One player's team, and one or more scripted teams, on a map
the player chose. The scripted teams' scripts and bundles are what
[data/opposition/](../data/opposition/) holds, one directory per opposition,
each named for a tarot card. The first opposition is **`fool`**, the Fool:
a team that prints bots, hauls ore, builds depots, and
never attacks — a target to learn on, wandering with its eyes on the road.
The roster is one card long (Q29): each further card — one that fights
back, one that goes for printers — is a new question number, ruled when a
sim exists to test it against, and ships as its own directory here.

Everything a scripted team does is visible to the player as it would be in
PvP: its machines are sighted, its sounds heard, its buildings remembered.
Its bundles are readable in the data directory, which is deliberate — the
opposition is a worked example.

## PvP

The same match with two players, each behind a team's command log, and any
number of scripted teams beside them. Nothing in this doc changes for it;
what PvP adds is fairness — rate-limiting, the speed control (Q6), symmetric
maps (`03`) — and every one of those is deferred with Q4 and gets a question
number when PvP is scheduled. The hooks are already in the command format
(Q10, Q12).

## What this doc leaves open

- **The next card**, under a new question number, once the game runs.
- **A time limit and a score**, for PvP, under a new number.
- **Spectating**: a peer with no team, which is a `docs/06` matter.
