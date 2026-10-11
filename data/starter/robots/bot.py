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
# them.

CAPACITY = 10
FAR = 1000
# How far inside the seen edge the patrol loop runs, and how many steps a
# leg may take before the bot gives up on that corner (move_to does not
# find a way round rock or water).
INSET = 2
LEG_STEPS = 40


def dist2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def nearest(points):
    here = me().pos
    best = None
    for p in points:
        d = dist2(p, here)
        if best is None or d < best[0]:
            best = (d, p)
    return None if best is None else best[1]


def adjacent(p):
    here = me().pos
    return abs(p[0] - here[0]) + abs(p[1] - here[1]) == 1


def step_off():
    for d in ["n", "e", "s", "w"]:
        try:
            move(d)
            return
        except ValueError:
            pass
    wait(1)


def approach(p):
    """One step toward standing beside p; True once beside it."""
    here = me().pos
    if (here[0], here[1]) == (p[0], p[1]):
        step_off()
        return False
    if adjacent(p):
        return True
    try:
        move_to(p[0], p[1])
    except ValueError:
        wait(1)
    return False


def buildable(x, y):
    t = tile(x, y)
    return t is None or (t.terrain == "ground" and t.deposit is None)


def has_building(x, y):
    t = tile(x, y)
    return t is None or t.building is not None


# 1. Build


def ore_sources():
    found = []
    for t in tiles(-FAR, -FAR, FAR, FAR):
        if t.deposit is not None and t.deposit["ore"] > 0:
            found.append((t.x, t.y))
    for m in see():
        if m.team == me().team and m.model == "depot" and m.store["ore"] > 0:
            found.append(m.pos)
    return found


def build_step():
    """One go at building; False when there is nothing it can do."""
    plan = plans("building")
    teardown = [(p[0], p[1]) for p in plan if p[2] is None and has_building(p[0], p[1])]
    target = nearest(teardown)
    if target is not None:
        if approach(target):
            try:
                deconstruct(target[0], target[1])
            except ValueError:
                wait(1)
        return True
    sites = [(p[0], p[1]) for p in plan if p[2] is not None and buildable(p[0], p[1])]
    for m in see():
        if m.team == me().team and m.model == "site" and m.progress == 0:
            sites.append(m.pos)
    if not sites:
        return False
    load = me().load["ore"]
    if load < CAPACITY:
        source = nearest(ore_sources())
        if source is not None:
            if approach(source):
                try:
                    pick("ore")
                except ValueError:
                    wait(1)
            return True
        if load == 0:
            return False
    target = nearest(sites)
    if approach(target):
        for p in plan:
            if (p[0], p[1]) == target and p[2] is not None:
                try:
                    build(p[2], p[0], p[1])
                except ValueError:
                    pass
        try:
            drop("ore")
        except ValueError:
            wait(1)
    return True


# 2. Paint


def mark_jobs():
    """Every paint and overlay plan there is something to do about."""
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


def mark(job):
    x, y, slot, value = job
    try:
        if slot == "paint":
            if value is None:
                unpaint(x, y)
            else:
                paint(x, y)
        elif value is None:
            unoverlay(x, y)
        else:
            overlay(x, y)
    except ValueError:
        wait(1)


def mark_step():
    """One go at painting; False when there is nothing to paint."""
    jobs = mark_jobs()
    target = nearest([(j[0], j[1]) for j in jobs])
    if target is None:
        return False
    if approach(target):
        for j in jobs:
            if (j[0], j[1]) == target:
                mark(j)
                break
    return True


# 3. Patrol


def corners():
    """The patrol loop: the corners of the seen part of the map, inset."""
    seen = tiles(-FAR, -FAR, FAR, FAR)
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
    """One step round the loop; returns the leg and the steps taken on it."""
    target = corners()[leg]
    if dist2(target, me().pos) <= 2 or steps >= LEG_STEPS:
        return (leg + 1) % 4, 0
    try:
        move_to(target[0], target[1])
    except ValueError:
        return (leg + 1) % 4, 0
    return leg, steps + 1


# Each bot starts the loop at a different corner, so they spread out.
leg = me().id % 4
steps = 0
while True:
    if build_step():
        continue
    if mark_step():
        continue
    leg, steps = patrol_step(leg, steps)
