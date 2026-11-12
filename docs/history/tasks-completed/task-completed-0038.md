*Closed record — see [../README.md](../README.md). Not spec.*

# T38 — The vision pass visits only what a team sees or just stopped seeing

From [I9](../inbox-triaged/inbox-triaged-0009.md). Step 8 walks every
position on the map for every team that is in, so a hundred teams on a
208 by 208 map rewrite 4.3 million tile memories a tick whatever they see.
Only a tile the team sees now, or saw at the last pass, can change: the
pass visits the team's visible set and the tiles whose memory still says
`Visible`, and leaves the rest untouched, which is what the full walk
wrote to them. The tiles a team saw at the last pass are kept on the team
beside its memory, so no scan finds them. The sight cache stops comparing
every machine on every query: the world carries a generation counter that
every placement, removal, move, conversion and team change bumps, and the
cache holds the generation it was built at, so the check is one
comparison; the sites that move or change a machine are few and named,
and the existing cache test, which holds the cache to the seer-by-seer
definition across a move, a placement and a removal, gains a conversion
and a team going out. Each team's memory is written as before, and
sightings are no part of the hash, so no replay hash moves. The scaling
benchmark at 10,000 waiting bots on a hundred teams, 219 ms a tick
before, says what it is after.

Completed in `f9e3cdb`, the merge of #111. Each team keeps the tile
indices it saw at its last pass; the pass demotes those and writes what
the team sees now, and a test holds it to a walk of the whole map for
eighty ticks, across walking bots and a team that resigns. The world
counts every placement, removal and move, a conversion being a removal
and a placement and a team going out its machines' removal, through four
methods that are the only code allowed to do them, and the sight cache
holds the count it was built at. A build with debug assertions keeps the
comparison the count replaced and panics on a stale cache, and the gates
check seeds a move that bumps nothing and asserts the golden replay
fails with that message. No replay hash moved. The benchmark's step at
10,000 waiting bots on a hundred teams went from 223 ms to 67 ms, and on
two teams from 19 ms to 17 ms; what is left at a hundred teams is
[I10](../../INBOX.md).
