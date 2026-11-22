# A starter bot. Each time round it takes the first job it finds:
#
#   1. Build. Carry ore to the team's building plans and unfinished sites,
#      place the site once beside a plan, and take down a building under a
#      plan to clear it.
#   2. Wire. Lay the team's planned wire, which costs a little ore a tile,
#      and take up wire planned for removal.
#   3. Paint. Carry out the team's paint and overlay plans: a color or a
#      label, or a clear.
#   4. Patrol. Walk straight until the map's edge or something in the way,
#      turn, and go on, which uncovers the map as it goes.
#
# Everything a bot does runs its battery down, so before each job a bot
# whose battery is half empty goes to a printer or a charger of its team to
# recharge. If it runs low anyway, the hook in interrupts/on_low_power.py
# takes it there, and the bot then goes on with what it was doing.
#
# Place plans with the mark tools in the game (B, P, O, C) and the bots act
# on them. The jobs are in lib/jobs.py, built from the rest of lib/:
#
#   geo    positions, directions, distances
#   sense  what the team knows: tiles, machines, ore
#   act    actions that return True or False instead of raising
#   nav    go_to and approach, which find a way round rock and water
#   share  splitting a list of jobs between bots
#   jobs   build, wire, paint, patrol and recharge, one step at a time

from jobs import low, recharge_step, build_step, wire_step, mark_step, patrol_step

# Each bot sets off in a different direction, so they spread out.
leg = me().id % 4
steps = 0
while True:
    if low() and recharge_step(True):
        continue
    if build_step():
        continue
    if wire_step():
        continue
    if mark_step():
        continue
    leg, steps = patrol_step(leg, steps)
