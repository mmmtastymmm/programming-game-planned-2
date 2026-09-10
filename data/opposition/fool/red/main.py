# The Fool's red bots: carry ore from the nearest deposit to the nearest
# place that takes it — a planned site first, a depot otherwise — and build
# where the plans say. Printers cannot be built (Q31). Shipped source
# (design-invariant DI8): every name here is docs/01's or docs/02's.

CAPACITY = 10
FAR = 1000


def nearest(points):
    here = me().pos
    best = None
    for p in points:
        d = (p[0] - here[0]) ** 2 + (p[1] - here[1]) ** 2
        if best is None or d < best[0]:
            best = (d, p)
    return None if best is None else best[1]


def adjacent(p):
    here = me().pos
    return abs(p[0] - here[0]) + abs(p[1] - here[1]) == 1


def step_toward(p):
    try:
        move_to(p[0], p[1])
    except ValueError:
        wait(1)


def step_off():
    # Standing on the tile itself: an action wants it adjacent, so leave it.
    for d in ["n", "e", "s", "w"]:
        try:
            move(d)
            return
        except ValueError:
            pass
    wait(1)


def approach(p):
    # True once adjacent to p; otherwise one step nearer, or off it.
    here = me().pos
    if (here[0], here[1]) == (p[0], p[1]):
        step_off()
        return False
    if adjacent(p):
        return True
    step_toward(p)
    return False


def drop_targets():
    targets = [(p[0], p[1]) for p in plans("building")]
    for m in see():
        if m.team == me().team and m.kind == "building" and m.model != "printer":
            targets.append(m.pos)
    return targets


def deposits():
    found = []
    for t in tiles(-FAR, -FAR, FAR, FAR):
        if t.deposit is not None and t.deposit["ore"] > 0:
            found.append((t.x, t.y))
    return found


while True:
    if me().load["ore"] >= CAPACITY:
        target = nearest(drop_targets())
        if target is None:
            wait(5)
            continue
        if approach(target):
            for p in plans("building"):
                if (p[0], p[1]) == target:
                    try:
                        build(p[2], p[0], p[1])
                    except ValueError:
                        pass
            try:
                drop("ore")
            except ValueError:
                wait(1)
        continue
    target = nearest(deposits())
    if target is None:
        wait(5)
        continue
    if approach(target):
        try:
            pick("ore")
        except ValueError:
            wait(1)
