*Closed record — see [../README.md](../README.md). Not spec.*

# T24 — Build printers at a rising price, print by turns, drop conversion, one deployment per model

Q42 built. In `sim`: `build` and building plans accept `printer`; a
site's store capacity is `cost + n × cost_step`, `n` the team's printers
and other printer sites just before it is placed. Printing stopped being
an action: a new tick step 6 has each team's ready printers begin prints
by turns past the team's print turn until its bots and prints in progress
reach its cap, a printer's `printing` counts down in step 3 and places the
bot on the first on-map, passable, unoccupied neighbour, and a printer at
or below zero health loses its print. `print`, `convert`, pending deploys
and the conversion tie set are gone; deployments are `bot`, `printer` and
`depot`; every machine is named `<model><id>`; `printing` and the print
turn are hashed. In data: the printer's `cost`, `cost_step` and
`build_ticks` as playtest placeholders, the depot's `cost_step` of zero,
and the conversion rows and the `print` and `convert` cost rows removed.
The starter and Fool trees moved `robots/1.py` to `robots/bot.py` and
their printers only wait. The golden replay fixture regenerated: printing
is automatic from tick 1, the bot deployment and names changed, two new
values entered the hash, and the scenario's locked deploy became a
printer plan. In `render`: one robot file per deployment, no number
badge, bots in the neutral body color until T25, and a printer plan in
the tools.

Completed in `bf067b2`, with the renderer's clippy fix in `8a87d09`.
