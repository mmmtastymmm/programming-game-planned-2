*Closed record — see [../README.md](../README.md). Not spec.*

# P8 — The code lags Q62: no batteries, charge costs, grids, chargers, pylons or wire

This entry was opened against Q59, whose power was never built; Q62
withdrew Q59 and ruled power again, and Q63 amended it, so the gap is
now theirs. They ruled that every bot has a battery its calls run down
and its own solar refills, that a bot out of power waits to be charged
back to a level, that `low_power` is a fifth interrupt, which returns to
main flow, that buildings generate
and drain and share power as grids served oldest first, that a building
a grid cannot cover goes dark, and that grids are joined by pylons or by
underground wire; `01`, `02`, `03`, `05`, `06`, `07` and `08` state it as
current. The interpreter and the sim have it, from T39 and T31, merged in
`a78bade`; what is missing is the renderer's half: it draws a solar panel,
a charger and a pylon as depots, and no wire, no pylon's line, no mark
over a flat bot or a dark building, no wire tool and no grid in the
inspector. The work left was [T32](../tasks-completed/task-completed-0032.md), and this entry closes with
the commit that finishes it.

Fixed in `054bd20`, the merge of #127, which finished T32: the renderer
draws wire, a pylon's lines, the charger, the panel and the pylon as
their own shapes, the mark over a flat bot and a dark building and the
charge bar, has the wire tool in its strip, and shows the selected
machine's charge and the player's own grids in the inspector. With T39
and T31, merged in `a78bade`, the code holds all of Q62 and Q63.
