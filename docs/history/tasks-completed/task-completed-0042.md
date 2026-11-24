*Closed record — see [../README.md](../README.md). Not spec.*

# T42 — The hash reads no memory of a tile a team has never seen, and the vision pass copies nothing unchanged

From [I12](../inbox-triaged/inbox-triaged-0012.md). After T41, at
a hundred teams on a 208 by 208 map, 51 of the state hash's 60 ms are
the teams' memory tables: 4.3 million entries, 3.4 million of them
unknown, each of which the hash reads a wide record to learn and then
writes one byte for. Two changes, and neither moves a hash. Each team
keeps, beside its memory, which tiles it knows, written where the vision
pass first sees a tile, since that pass is the only writer of a tile's
state; the hash writes an unknown tile's byte from that and reads the
memory of the known ones only, and may fold a run of unknown tiles into
one step, FNV-1a over a zero byte being one multiplication. And the
vision pass, for a tile the team sees, sets `seen_at` and the state and
replaces a field of its memory only where it differs from the tile, so
a tick on which nothing changed copies no deposit, paint, overlay, wire
set or building record. A test holds the index to the memory's states
across walking bots, a fork's copy and a team that goes out, and the
existing one that holds the pass to a walk of the whole map still
passes. The PR shows the golden fixtures untouched and
`crates/replay/benches/scale.rs` at a hundred teams and at two, before
and after. Whether the hash should stop writing the known tiles in full
is `Q64`, and no part of this task.

Completed in `6df6006`, the merge of #132; the task's own commit is
`4199ef8`. Each team keeps `known`, a flag a tile, set where the vision
pass first sees one and never cleared, and the hash tells an unknown
tile from it; a run of unknown tiles is one multiplication,
`Fnv1a::write_zeros`, which a test holds to the bytes written one by
one. The vision pass replaces a field of a memory only where it differs
from the tile. The test that holds the pass to a walk of the whole map
now changes a tile's paint, overlay, wire and building in a team's sight
and back, holds the index to the memory on every tick, a team that is
out included, and has a fork opened from a saved state hash as the match
does; a debug build compares the index with the memory inside the hash,
and the Rust gates seed a pass that forgets the index. No golden fixture
moved. `crates/replay/benches/scale.rs` at 10,000 bots that only wait on
a 208 by 208 map, the two builds run alternately: at a hundred teams the
step went from 67 ms to 58 ms and the hash from 59 ms to 40 ms; at two
teams the step from 27 ms to 24 ms and the hash from 8.1 ms to 7.9 ms.
What is left of the hash at a hundred teams is the tiles the teams do
know, written in full, which `Q64` asks about and was open when this
closed; of the step's 58 ms the vision pass is about 25, since it still
sets `seen_at` on every tile in sight. It was the one task under M12,
Scale.
