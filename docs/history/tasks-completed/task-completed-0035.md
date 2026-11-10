*Closed record — see [../README.md](../README.md). Not spec.*

# T35 — The sim's sight queries and vision pass stop scanning every machine

From [I8](../inbox-triaged/inbox-triaged-0008.md). `sense::team_sees`
and `team_can_see` recomputed a team's whole field of view — every seer
against every machine, with a line of sight for each pair — on every
`see()` and attack, so a thousand bots each calling `see()` once a tick
was the tick; and the vision pass searched every machine for a building
on every visible tile. A `SightCache` on the world holds each team's
visible tiles between queries, checked against every machine's id,
position, model and team on each use so any move, placement or removal
invalidates it without a hook; `team_sees` is the machines standing on a
visible tile, which is the same test; the vision pass indexes buildings by
tile once. Sightings are no part of the hash, and the queries answer as
before, so no replay hash moves; a test holds the cache to the seer-by-
seer definition across a move, a placement and a removal.

Completed in `0665332`, the merge of #101, which did the work and triaged the
inbox entry that asked for it in one PR; this file closes the task.
