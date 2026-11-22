*Closed record — see [../README.md](../README.md). Not spec.*

# T31 — Power in `sim`: batteries, charge costs, grids, the charger, the pylon, the panel and wire

Q62 as Q63 amended it, ruled in `08`, with `02`'s models, attributes,
senses, actions and Printing, `03`'s tile and `06`'s tick, snapshot and
hash. Needs T39.

The data, in `data/machines.toml`: every model's `battery` and
`generation`, and every building model's `drain`; the printer's
`print_drain`; `recharge_rate` on the printer and the charger; the bot's
`recharge_ticks`, `wire_cost`, `wire_ticks` and `unwire_ticks`; a `charger`,
a `solar` and a `pylon` model with every row a building model has, the
pylon's `span` besides; `[power]` with `low_power_level` and
`wake_level`, and `[power.cost]` with a row for every game builtin; the
`[deployment]` comment naming five deployments; and the load checks `08`
lists.
For every new builtin a `[game]` cost row and a hover entry in
`data/language/docs.toml` (Q57), whose test holds the two tables equal;
the hover entries Q62 makes stale brought up to date — the `Machine`
record's `charge`, `battery`, `flat`, `powered` and `grid`, `tile()`'s
`wires`, `plans()`'s `"wire"` kind, and `build()`'s and `build_nearest()`'s
models.

The bot: `charge`, `battery` and `flat`; a charge cost taken as a call
begins; out of power — flat from a call main flow or `on_low_power`
cannot pay, answered to the interpreter as not ready (T39), no costed
call begun while flat, and `on_fault`'s and `on_dying`'s calls taking
what there is; the solar in the power step, and the wake after it at
`wake_level`, which raises `low_power`; whether it is armed, and
`low_power` raised as the charge falls to the level and armed again above
it; a `low_power` prologue that does nothing to the body; `recharge()`,
re-checked on completion; a printed bot full.

The grid: the tile's per-team `wires`, on `ground` only, sensed and
remembered like paint and cleared with an out team; the wire plan as a
fourth plan kind in `Mark` and `Unmark`, only on seen ground and dropped
when it asks for what the tile already has; `wire` and `unwire`,
re-checked on completion; conductors, joins by adjacency and by a
pylon's span, labels; step 6 as `08` orders it — trace, the three passes,
solar, prints, figures — and its tick-0 form; `powered` and `grid`; a
dark building's rules, program and senses stopped and its handlers run;
a print begun only by a powered printer and held while dark, with
no spare power and no second cap, which went with Q59; `power()` and `wires()`; the
`charger` deployment in `Deploy` validation, and none for the pylon; the
snapshot's tile wires, their part-of-a-grid flags and each team's grids;
the state hash over all of it.

The starter: `robots/charger.py` and `robots/solar.py`,
`interrupts/on_low_power.py`, which calls a recharge step in `lib/jobs`,
and a wire step there, so a bot low on charge goes to a powered building
and comes back to what it was doing, and a panel planned and wired on the
default tree feeds a charger; the starter's
docstrings and comments brought up to date (design-invariant DI8).

Tests: a call the battery cannot cover leaves the bot flat until its
solar reaches the wake level, not merely the call's cost; waking runs
`on_low_power` before the call is made, and starts over a hook that was
itself flat; `on_fault`'s and `on_dying`'s calls are never held;
`low_power` comes once per fall and again only after a recharge above the
level; it waits for the action in progress; a bot with no hook is not
disturbed, and one with a hook goes on where it was;
a grid cut by destroying a pylon, and one cut by `unwire`, each darken
what the rest can no longer cover; the older building is served first and
the next is tried when one does not fit; a dark building still fills its
battery and is powered again by itself; a dark building's tiles become
remembered; a lone printer is never dark, and data that would let it be
is refused; a rival's bot recharges from a charger; two bots recharging
from one building in a tick are served lower id first; two bots wiring
one tile pay once. The golden fixtures change, and the PR says why.

Completed in `a78bade`, the merge of #122; the task's own commit is
`445eff7`. The sim has charge costs on every game builtin, a bot out of
power and its waking, `low_power` raised once per fall, grids traced and
served in the tick's step 6, dark buildings, the `solar`, `charger` and
`pylon` models, the builtins `recharge`, `wire`, `unwire`, `power` and
`wires`, and the wire plan kind in the command log and its bytes. `lang`
gained `Host::take_raised` on the way, so that a call's own charge can
raise `low_power` for the machine that made it. The golden replay hashes
changed, as the task's mark said they would, and the scenario grew to
cover the new paths: a charger, a pylon and a tile of wire that end as
one grid with the two printers, a low-power hook in the player's tree,
and 640 ticks in place of 400.

What a reader should know that the task did not say. The printer's
numbers are generous, at the user's request, for testing: it makes 6
against 2 used and keeps the ten bots of its cap charged by itself, so a
charger and a panel are not yet needed, and `05`'s "a few bots" does not
describe the data. The starter bot goes to recharge at half a battery, at
its own team's buildings: at a fifth it could not cross the first map,
and the nearest powered building was the rival's printer. The starter's
low-power hook takes at most forty cheap steps and never searches for a
route, because a route search in a queue round a printer ran its budget
out, which is a fault; the hook's budget is 50000. A building is placed
powered and on no grid until its first power step, which `08` now says.
The print held by a dark printer has no test, since no data the sim
accepts lets a printer go dark. The renderer draws the three new models
as depots and nothing else of power, which is T32. And the longer golden
scenario made CI's `test` job slow: 36 minutes on the merge, from about
5, most of it the fork tests and the seeded-defect gates replaying it.
With T39 and T32 it fixes [P8](../../PROBLEMS.md), which stays open
against T32.
