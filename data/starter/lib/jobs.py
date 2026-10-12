"""The starter bot's three jobs, one step at a time.

Each *_step function does one step of its job (a move, a pick, a drop, a
build or a paint) and returns True, or returns False at once when its job
has nothing to do, so a bot can try them in order:

    while True:
        if build_step():
            continue
        if mark_step():
            continue
        leg, steps = patrol_step(leg, steps)

Plans come from the mark tools in the game (B, P and O): `plans(kind)`
reads them as (x, y, value) tuples, value None for a plan to clear.
"""

from geo import same, dist2
from sense import seen_tiles, has_building, buildable, nearest, machines, ore_sources
from act import try_pick, try_drop, try_build, try_deconstruct
from act import try_paint, try_unpaint, try_overlay, try_unoverlay
from nav import approach, go_to

# Ore a bot carries at most (data/machines.toml).
CAPACITY = 10
# How far inside the seen edge the patrol loop runs.
INSET = 2
# Steps a patrol leg may take before the bot gives up on that corner.
LEG_STEPS = 40


# Building


def build_step():
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


# Painting


def mark_jobs():
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


def do_mark(job):
    """Carry out one mark job on its adjacent tile; True if it started."""
    x, y, slot, value = job
    if slot == "paint":
        return try_unpaint(x, y) if value is None else try_paint(x, y)
    return try_unoverlay(x, y) if value is None else try_overlay(x, y)


def mark_step():
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


def corners():
    """The patrol loop: the corners of the seen part of the map, inset."""
    seen = seen_tiles()
    x0 = min([t.x for t in seen])
    x1 = max([t.x for t in seen])
    y0 = min([t.y for t in seen])
    y1 = max([t.y for t in seen])
    if x1 - x0 > 2 * INSET:
        x0 += INSET
        x1 -= INSET
    if y1 - y0 > 2 * INSET:
        y0 += INSET
        y1 -= INSET
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def patrol_step(leg, steps):
    """One step round the loop, which uncovers the map as it goes.

    leg is the corner (0 to 3) being walked to and steps how many steps
    this leg has taken; returns both for the next call. A leg ends near its
    corner, or after LEG_STEPS steps, or when no route is found.
    """
    target = corners()[leg]
    if dist2(target, me().pos) <= 2 or steps >= LEG_STEPS:
        return (leg + 1) % 4, 0
    if go_to(target) is None:
        return (leg + 1) % 4, 0
    return leg, steps + 1
