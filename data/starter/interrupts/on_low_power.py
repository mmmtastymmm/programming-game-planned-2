# The team-wide low-power hook (docs/01, Q41, Q63). It runs when a bot's
# battery falls to a fifth, and again when a bot that ran out has charged
# back up; the bot's own program then goes on from where it was. This one
# walks the bot toward its team's nearest powered building holding charge —
# a printer or a charger — and recharges until the battery is nearly full.
# The starter bot goes to recharge by itself at half a battery
# (robots/bot.py), so this is the fallback for one that did not get there.
#
# A hook has a budget, and one that runs out of it is a fault. So this one
# takes at most TRIES steps, each a cheap one: recharge_step heads straight
# for the building and does not search for a way round. A bot that cannot
# get there, because of a wall or a queue, goes back to work on what it
# has, and tries again the next time the hook runs.

from jobs import recharge_step
from nav import forget

TRIES = 40


def on_low_power():
    tries = 0
    while tries < TRIES and recharge_step():
        tries += 1
    # The bot has moved: the route its program was following is stale.
    forget()
