# The fixture's first bot program: haul ore from the nearest deposit into
# the team's building plan — a printer, whose price rises with the team's
# printers (Q42) — building the site when there is none, and keep a tally
# that a redeploy will throw away.
steps = 0
CAPACITY = 10


def deposit():
    for t in tiles(-30, -30, 30, 30):
        if t.deposit is not None and t.deposit["ore"] > 0:
            return (t.x, t.y)
    return None


def site():
    for p in plans("building"):
        return (p[0], p[1])
    for m in see():
        if m.team == me().team and m.model == "site":
            return (m.pos[0], m.pos[1])
    return None


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


while True:
    full = me().load["ore"] >= CAPACITY
    target = site() if full else deposit()
    if target is None:
        wait(5)
        continue
    here = me().pos
    if (here[0], here[1]) == target:
        step_off()
        continue
    if adjacent(target):
        if full:
            for p in plans("building"):
                if (p[0], p[1]) == target:
                    try:
                        build(p[2], p[0], p[1])
                        log("site placed")
                    except ValueError:
                        pass
            try:
                drop("ore")
                log("dropped")
            except ValueError:
                wait(1)
        else:
            try:
                pick("ore")
                log("picked", me().load["ore"])
            except ValueError:
                wait(1)
        continue
    try:
        move_to(target[0], target[1])
        steps += 1
    except ValueError:
        wait(2)
    if steps % 4 == 0:
        log("steps", steps, "sees", len(see()), "hears", len(hear()))
