# The fixture's first bot program: haul ore from the nearest deposit into
# the team's building plans, one at a time — a charger, a pylon and a
# printer, whose price rises with the team's printers (Q42) — building the
# site when there is none, lay the team's planned wire on the way (Q62),
# and keep a tally that a redeploy will throw away. It wears its job as its
# color (Q43): orange while hauling to the site, cyan while fetching ore.
# The low-power hook beside it (interrupts/on_low_power.py) takes a bot
# off to recharge, and this loop then goes on where it was (Q63).
steps = 0
CAPACITY = 10
# The team's first bot hauls nothing: it paces a lane away from the
# deposits, which empties a battery about as fast as a bot can, so that
# inside the fixture's few hundred ticks one bot falls to low power, runs
# the hook, walks back to recharge and returns. The id is the first a
# printer on this map gives a bot, and the lane is two tiles of its ground.
RUNNER = 3
LANE = (-6, 6)


def pace():
    here = me().pos
    if here[1] != LANE[1] or here[0] < LANE[0] or here[0] > LANE[0] + 1:
        try:
            move_to(LANE[0], LANE[1])
        except ValueError:
            wait(1)
        return
    try:
        move("e" if here[0] == LANE[0] else "w")
    except ValueError:
        wait(1)


def deposit():
    for t in tiles(-30, -30, 30, 30):
        if t.deposit is not None and t.deposit["ore"] > 0:
            return (t.x, t.y)
    return None


def site():
    # The printer first: it is the dearest, and the fixture wants it up
    # and printing while there are still ticks to see it take its turn.
    planned = plans("building")
    for p in planned:
        if p[2] == "printer":
            return (p[0], p[1])
    for p in planned:
        return (p[0], p[1])
    for m in see():
        if m.team == me().team and m.model == "site":
            return (m.pos[0], m.pos[1])
    return None


def wire_plan():
    for p in plans("wire"):
        if p[2]:
            return (p[0], p[1])
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
    if me().id == RUNNER:
        pace()
        continue
    full = me().load["ore"] >= CAPACITY
    job = "orange" if full else "cyan"
    if me().color != job:
        set_color(job)
    wiring = wire_plan() if full else None
    target = wiring or (site() if full else deposit())
    if target is None:
        wait(5)
        continue
    here = me().pos
    if (here[0], here[1]) == target:
        step_off()
        continue
    if adjacent(target):
        if wiring is not None:
            try:
                wire(target[0], target[1])
                log("wired")
            except ValueError:
                wait(1)
        elif full:
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
