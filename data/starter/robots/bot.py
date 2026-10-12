# A starter bot. Each time round it takes the first job it finds:
#
#   1. Build. Carry ore to the team's building plans and unfinished sites,
#      place the site once beside a plan, and take down a building under a
#      plan to clear it.
#   2. Paint. Carry out the team's paint and overlay plans: a color or a
#      label, or a clear.
#   3. Patrol. Walk a loop around the part of the map the team has seen,
#      which uncovers more of it as it goes.
#
# Place plans with the mark tools in the game (B, P, O) and the bots act on
# them. The jobs are in lib/jobs.py, built from the rest of lib/:
#
#   geo    positions, directions, distances
#   sense  what the team knows: tiles, machines, ore
#   act    actions that return True or False instead of raising
#   nav    go_to and approach, which find a way round rock and water
#   share  splitting a list of jobs between bots
#   jobs   build, paint and patrol, one step at a time

from jobs import build_step, mark_step, patrol_step

# Each bot starts the loop at a different corner, so they spread out.
leg = me().id % 4
steps = 0
while True:
    if build_step():
        continue
    if mark_step():
        continue
    leg, steps = patrol_step(leg, steps)
