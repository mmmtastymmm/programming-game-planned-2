"""The starter bot's jobs, one step at a time.

Each *_step function does one step of its job (a move, a pick, a drop, a
build, a paint or a wire) and returns True, or returns False at once when
its job has nothing to do, so a bot can try them in order:

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

Plans come from the mark tools in the game (B, P, O and C): `plans(kind)`
reads them as (x, y, value) tuples, value None for a plan to clear.

A bot goes to recharge when its battery is half empty (low()), so that it
gets there on what it has left. The low-power hook
(interrupts/on_low_power.py) runs recharge_step too, at a fifth, for a bot
that did not make it.
"""

from geo import DIRS, same, step, adjacent
from sense import known, passable, has_building, buildable, nearest, machines
from sense import ore_sources, charge_sources
from act import try_move, try_move_to, try_pick, try_drop, try_build, try_deconstruct
from act import try_paint, try_unpaint, try_overlay, try_unoverlay
from act import try_wire, try_unwire, try_recharge
from nav import approach

# Ore a bot carries at most (data/machines.toml).
CAPACITY: int = 10
# Steps a patrol leg may take before the bot turns anyway.
LEG_STEPS: int = 40
# Ore one tile of wire costs (data/machines.toml).
WIRE_COST: int = 3
# The share of its battery a bot recharges to before going back to work.
FULL = 0.9
# The share below which a bot goes to recharge before it takes another job.
# Half a battery is enough to cross the first map; the low-power hook, at a
# fifth, is the fallback for a bot that could not get home.
RESERVE = 0.5


# Building


def build_step() -> bool:
    """One step of building; False when there is nothing it can do.

    Takes down a building under a plan to clear it first. Otherwise fetches
    ore, carries it to the nearest planned or unfinished site, places the
    site beside a plan, and drops the ore in.
    """
    plan = plans("building")
    teardown = [(p[0], p[1]) for p in plan if p[2] is None and has_building(p[0], p[1])]
    target = nearest(teardown)
    if target is not None:
        if approach(target) and not try_deconstruct(target[0], target[1]):
            wait(1)
        return True
    sites = [(p[0], p[1]) for p in plan if p[2] is not None and buildable(p[0], p[1])]
    for m in machines("site"):
        if m.progress == 0:
            sites.append(m.pos)
    if not sites:
        return False
    load = me().load["ore"]
    if load < CAPACITY:
        source = nearest(ore_sources())
        if source is not None:
            if approach(source) and not try_pick("ore"):
                wait(1)
            return True
        if load == 0:
            return False
    target = nearest(sites)
    if approach(target):
        for p in plan:
            if same(p, target) and p[2] is not None:
                try_build(p[2], p[0], p[1])
        if not try_drop("ore"):
            wait(1)
    return True


# Wiring


def wire_step() -> bool:
    """One step of wiring; False when there is no wire plan to act on.

    Takes up wire planned for removal first. Otherwise fetches ore, since a
    tile of wire costs WIRE_COST, and lays the nearest planned wire.
    """
    plan = plans("wire")
    target = nearest([(p[0], p[1]) for p in plan if p[2] is None])
    if target is not None:
        if approach(target) and not try_unwire(target[0], target[1]):
            wait(1)
        return True
    lay = [(p[0], p[1]) for p in plan if p[2] is not None]
    if not lay:
        return False
    if me().load["ore"] < WIRE_COST:
        source = nearest(ore_sources())
        if source is None:
            return False
        if approach(source) and not try_pick("ore"):
            wait(1)
        return True
    target = nearest(lay)
    if approach(target) and not try_wire(target[0], target[1]):
        wait(1)
    return True


# Recharging


def low() -> bool:
    """True once the battery is below RESERVE of what it holds: time to go
    and recharge, while there is charge enough to get there."""
    m = me()
    return m.charge < m.battery * RESERVE


def recharge_step(search: bool = False) -> bool:
    """One step toward a nearly full battery; False once it is, or when
    there is nothing more this step can do about it.

    Heads for the team's nearest powered building holding charge and
    recharges there; a recharge moves only so much at a time, so call it
    until it returns False. It gives up rather than wait beside a building
    that has run low.

    search: find a way round rock and water, as the other jobs do. Leave it
    False in the low-power hook, which has a budget a route search could
    use up: the bot then takes move_to's one greedy step and searches for
    nothing.
    """
    m = me()
    if m.charge >= m.battery * FULL:
        return False
    target = nearest(charge_sources())
    if target is None:
        return False
    if adjacent(m.pos, target):
        return try_recharge()
    if search:
        return approach(target) is not None
    if not try_move_to(target[0], target[1]):
        wait(1)
    return True


# Painting


def mark_jobs() -> list[tuple[int, int, str, str | None]]:
    """Every paint and overlay plan there is something to do about, as
    (x, y, slot, value) tuples, slot "paint" or "overlay"."""
    jobs = []
    for slot in ["paint", "overlay"]:
        for p in plans(slot):
            t = tile(p[0], p[1])
            # A clear on a tile with nothing to clear can never be done.
            if p[2] is None and t is not None:
                now = t.paint if slot == "paint" else t.overlay
                if now is None:
                    continue
            jobs.append((p[0], p[1], slot, p[2]))
    return jobs


def do_mark(job: tuple[int, int, str, str | None]) -> bool:
    """Carry out one mark job on its adjacent tile; True if it started."""
    x, y, slot, value = job
    if slot == "paint":
        return try_unpaint(x, y) if value is None else try_paint(x, y)
    return try_unoverlay(x, y) if value is None else try_overlay(x, y)


def mark_step() -> bool:
    """One step of painting; False when there is nothing to paint."""
    jobs = mark_jobs()
    target = nearest([(j[0], j[1]) for j in jobs])
    if target is None:
        return False
    if approach(target):
        for j in jobs:
            if same(j, target):
                if not do_mark(j):
                    wait(1)
                break
    return True


# Patrolling


def patrol_step(leg: int, steps: int) -> tuple[int, int]:
    """One step of the patrol, which uncovers the map as it goes.

    leg is the direction being walked (an index into DIRS) and steps how
    many steps this leg has taken; returns both for the next call. The bot
    walks straight until the leg has taken LEG_STEPS steps, or the tile
    ahead is off the map, rock, water or a building, or another bot is in
    the way; then it turns right and starts the next leg. Walking to each
    edge in turn and turning rounds the map, so the loop grows with the map
    and the patrol costs the same on any: one tile looked at and one step.
    It used to measure the part of the map the team had seen first, which
    cost a unit per seen tile every step and on a big map was most of the
    tick budget.

    A tile next to the bot that the team does not know is off the map: a
    bot always sees the tiles around it.
    """
    d = DIRS[leg]
    ahead = step(me().pos, d)
    if (
        steps >= LEG_STEPS
        or not known(ahead[0], ahead[1])
        or not passable(ahead[0], ahead[1])
        or not try_move(d)
    ):
        return (leg + 1) % 4, 0
    return leg, steps + 1
