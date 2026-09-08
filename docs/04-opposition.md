# 04 — Opposition

Who the player fights, and how a match ends. The opposition in PvE is
another team — one the game controls, running programs the game ships,
subject to every rule the player's team is — and PvP is the same match with
a second player on that team. This doc is normative for what a team is, how
a match ends, and what the game's own teams are made of; two implementers
reading it must agree on the tick a match ends (design-invariant DI7).

## Decided

- **PvE ships before PvP (Q4).** Lockstep is built now regardless, since it
  is not retrofittable, so deferring PvP costs nothing architecturally and
  buys slack on balance while the sim changes fastest.

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
[data/opposition/](../data/opposition/), part of a replay's inputs and
hashed into its identity like the map, that says which bundles to deploy to
which deployments on which ticks, which marks to place, and nothing else. A
script is a command log written in advance. It may not do anything a player
could not: its bundles are ordinary bundles in the language, its marks are
ordinary marks, and its machines obey every rule.

- **A script is deterministic by construction**: it is a fixed list of
  `(tick, command)` pairs, applied by the driver as if a peer had sent them,
  with the same delay (Q12) as a player's. A scripted team never stalls the
  match, since its sets are always in hand.
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

**A team is out** on the tick it has no printer and no site that would
become one — the moment its last printer's `death` epilogue runs, or its
last printer-site is deconstructed, with no other printer or printer-site
standing. An out team's machines are removed at the end of that tick's
damage step (`06`, step 4), its plans are cleared, and its command log is
closed: further commands from it are dropped at submission. `Resign` (Q10)
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
of bots, and a bot is the only thing that can build a printer. A team with
no printer and no printer-site cannot ever have one again, so it is out the
moment that is true, not when its last bot dies. Sites count because a
site is a printer in the making, and a team with bots hauling to one is
alive.

## PvE

The first mode. One player's team, and one or more scripted teams, on a map
the player chose. The scripted teams' scripts and bundles are what
[data/opposition/](../data/opposition/) holds, one directory per opposition,
each named for what it does. The first opposition is **`hauler`**: a team
that prints bots, hauls ore, builds a depot and a second printer, and never
attacks — a target to learn on. What the others are, and how they escalate
into an opposition that fights back, is Q29.

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

- **Q29**: the opposition roster beyond `hauler` — what the shipped teams
  do, how they escalate, and whether the player picks one or the map does.
- **A time limit and a score**, for PvP, under a new number.
- **Spectating**: a peer with no team, which is a `docs/06` matter.
