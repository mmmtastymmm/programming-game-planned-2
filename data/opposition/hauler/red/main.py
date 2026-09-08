# The hauler's red bots: find a deposit, carry ore to the nearest site the
# team has planned, and build a depot and then a second printer where the
# plans say. Shipped source (design-invariant DI8).

def nearest(points):
    here = me().pos
    best = None
    for p in points:
        d = (p[0] - here[0]) ** 2 + (p[1] - here[1]) ** 2
        if best is None or d < best[0]:
            best = (d, p)
    return None if best is None else best[1]

def step_toward(x, y):
    try:
        move_to(x, y)
    except ValueError:
        wait(1)

while True:
    plan = nearest(plans("building"))
    if plan is not None and me().load["ore"] > 0:
        step_toward(plan[0], plan[1])
        try:
            drop("ore")
        except ValueError:
            pass
        continue
    deposits = [t for t in tiles(-12, -8, 11, 7) if t.deposit is not None]
    target = nearest([(t.x, t.y) for t in deposits])
    if target is None:
        wait(5)
        continue
    if me().load["ore"] < 10:
        step_toward(target[0], target[1])
        try:
            pick("ore")
        except ValueError:
            pass
    else:
        if plan is not None:
            try:
                build(plan[2], plan[0], plan[1])
            except ValueError:
                pass
        wait(1)
