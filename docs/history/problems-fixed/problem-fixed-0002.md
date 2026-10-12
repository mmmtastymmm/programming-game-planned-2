*Closed record — see [../README.md](../README.md). Not spec.*

# P2 — The code lags Q43: no `set_color`, no `color`, no outlines

Q43 gave bots a `set_color(color)` builtin and a `color` attribute, and
every team an outline the player colors. `docs/02` states the builtin, the
attribute and the `dying` hook's use of it as current, its Costs section
says every builtin above has a row in `data/language/costs.toml`, and
`docs/07` names the neutral body color, the default outline color and the
outline list as constants in `data/interface.toml`. None exists yet: the
sim has no `set_color` or `color`, the cost table has no row for it, the
renderer draws bots white and teams as rings, and `data/interface.toml`
holds only the ring list. The work is
[T25](../tasks-completed/task-completed-0025.md); this entry closes with
the commit that finishes it.

Fixed in `e8eb0fc`, the merge of #73, which finished T25: the sim has
`color` and `set_color` with its cost row, `data/interface.toml` holds
the neutral body, the player's default color and the list for the other
teams, and the renderer draws a team as Q43 amended by Q45 and Q47 rule:
lights on bots, printers and depots, an outline on sites.
