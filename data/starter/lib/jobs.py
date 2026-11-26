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
        if ground_step():
            continue
        if mark_step():
            continue
        leg, steps = patrol_step(leg, steps)

Plans come from the mark tools in the game (B, P, O, C and G):
`plans(kind)` reads them as (x, y, value) tuples, value None for a plan to
clear. A ground plan's value is the height the tile should be brought to.

A bot goes to recharge when its battery is half empty (low()), so that it
gets there on what it has left. The low-power hook
(interrupts/on_low_power.py) runs recharge_step too, at a fifth, for a bot
that did not make it.
"""

from geo import DIRS, same, step, adjacent
from sense import known, passable, has_building, buildable, nearest, machines
from sense import ore_sources, charge_sources, earth_sources
from act import try_move, try_move_to, try_pick, try_drop, try_build, try_deconstruct
from act import try_paint, try_unpaint, try_overlay, try_unoverlay
from act import try_wire, try_unwire, try_recharge
from act import try_dig, try_fill, try_mine
from nav import approach
from share import rank

# Ore a bot carries at most (data/machines.toml).
CAPACITY: int = 10
# Steps a patrol leg may take before the bot turns anyway.
LEG_STEPS: int = 40
# Ore one tile of wire costs (data/machines.toml).
WIRE_COST: int = 3
# Earth a bot carries at most (data/machines.toml).
EARTH_ROOM: int = 4
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


# Shaping the ground


def ground_step() -> bool:
    """One step of shaping the ground; False when there is no ground plan
    to act on.

    A ground plan is a height. Rock under one is mined; a tile above its
    plan is dug down, a level a dig, and its earth goes into the load; a
    tile below its plan is filled, a level a fill, from the load. A bot
    with earth fills first. One with a full load and nothing to fill
    takes the earth to a depot, and one with a fill to do and no earth
    fetches some from a depot: with no depot, digging stops at a full
    load and filling waits for a dig.

    A dig or a fill is a whole level whoever does it, so two bots on one
    tile would take it a level too far. Each tile is therefore one bot's:
    the plans are dealt out by rank, and a bot leaves alone a tile that
    another is already at work on.
    """
    mines, digs, fills = [], [], []
    for p in plans("ground"):
        t = tile(p[0], p[1])
        if t is None:
            continue
        if t.terrain == "rock":
            mines.append((p[0], p[1]))
        elif t.height > p[2]:
            digs.append((p[0], p[1]))
        elif t.height < p[2]:
            fills.append((p[0], p[1]))
    if not mines and not digs and not fills:
        return False
    place, bots = rank()
    digs = [p for i, p in enumerate(digs) if i % bots == place and not at_work(p)]
    fills = [p for i, p in enumerate(fills) if i % bots == place and not at_work(p)]
    earth = me().load["earth"]
    if fills and earth >= 1:
        target = nearest(fills)
        if approach(target) and not try_fill(target[0], target[1]):
            wait(1)
        return True
    if mines:
        target = nearest(mines)
        if approach(target) and not try_mine(target[0], target[1]):
            wait(1)
        return True
    if digs and earth < EARTH_ROOM:
        target = nearest(digs)
        if approach(target) and not try_dig(target[0], target[1]):
            wait(1)
        return True
    if digs:
        # A full load: somewhere has to take the earth.
        depot = nearest([m.pos for m in machines("depot", me().team)])
        if depot is None:
            return False
        if approach(depot) and not try_drop("earth"):
            wait(1)
        return True
    if fills:
        source = nearest(earth_sources())
        if source is None:
            return False
        if approach(source) and not try_pick("earth"):
            wait(1)
        return True
    return False


def at_work(p: tuple[int, int]) -> bool:
    """Whether another of the team's bots beside p is digging or filling."""
    for m in machines("bot", me().team):
        if m.id != me().id and m.busy in ("dig", "fill") and adjacent(m.pos, p):
            return True
    return False


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
