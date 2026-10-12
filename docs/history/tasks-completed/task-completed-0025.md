*Closed record — see [../README.md](../README.md). Not spec.*

# T25 — A bot's `set_color`, and the team outline the player colors

Q43 built, as amended by Q45, Q46 and Q47. In `sim`: a bot's `color`
attribute, `None` when printed, kept across a fault's restart and a
redeploy, carried by sightings and `me()`, absent on buildings, and in
the state hash; the zero-tick `set_color(color)` action, a paint color or
`None`, refused while busy and allowed in a `dying` hook, with its row in
`data/language/costs.toml`. The golden fixtures were regenerated: every
machine record hashes a color slot, and the fixture's first bot wears its
job as its color.

In `render`: a bot drawn in its color's atlas or the neutral body, its
lights — the lens, the side markers, the tail light, the top dot —
glowing in its team's color through a mask the bake makes from art that
marks its lights (Q45); a printer's panel light, side markers and status
light, and a depot's lamp where its braces cross, glowing the same way
(Q47); a site outlined in its team's color, replacing the ring. Each bot
wears its entity id's last three digits on its top face in fixed seven
segments, lit in the turn that reads upright from the camera (Q46). The
player's own team is green by default and the others from the list in
`data/interface.toml`; any team's color is changed from the inspector,
saved in `colors.toml` beside the programs and restored at start.
Smoke-tested in a running window.

Completed in `e8eb0fc`, the merge of #73; finishing it fixed
[P2](../problems-fixed/problem-fixed-0002.md).
