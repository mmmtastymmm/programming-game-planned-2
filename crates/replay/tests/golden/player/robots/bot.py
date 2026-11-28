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
# The team's first bot hauls nothing: it paces a lane beside the deposit,
# which empties a battery about as fast as a bot can, so that inside the
# fixture's few hundred ticks one bot falls to low power, runs the hook,
# walks back to recharge and returns. The id is the first a printer on
# this map gives a bot, and the lane is two tiles of its ground.
RUNNER = 3
LANE = (-8, 6)
# Before it paces, the runner works the ground east of the lane (Q65), a
# step of this list at a time: it digs a tile down a level and fills the
# one beside it up a level with that earth, climbs onto the filled tile
# and moves off it onto the dug one, a drop of two levels and so a fall,
# and then mines a rock away. And once, pacing, it bonks a bot that has
# stopped to pick at the lane's west end.
CREW = [
    ("go", -3, 6),
    ("dig", -3, 7),
    ("go", -2, 6),
    ("fill", -2, 7),
    ("move", "n"),
    ("move", "w"),
    ("go", -1, 6),
    ("mine", 0, 6),
]
runner = {"stage": 0, "bonked": False}
# The ninth bot the team's first printer prints digs (Q65): it fills up
# with ore like any other, and from then on carries out the
# team's ground plans, standing west of each, and places and fills the
# site of a building plan beside it — all of it south of the base, by the
# lake, so that it leaves the base's own sites to the others.
DIGGER = 25


def crew():
    step = CREW[runner["stage"]]
    here = me().pos
    try:
        if step[0] == "go":
            if (here[0], here[1]) != (step[1], step[2]):
                move_to(step[1], step[2])
                return
        elif step[0] == "dig":
            dig(step[1], step[2])
        elif step[0] == "fill":
            fill(step[1], step[2])
        elif step[0] == "mine":
            mine(step[1], step[2])
        else:
            move(step[1])
            log("crew moved", step[1], "to height", tile(me().pos[0], me().pos[1]).height)
    except ValueError:
        wait(1)
        return
    runner["stage"] += 1
    log("crew", step[0], "done, health", me().health)


def bonk():
    # At the lane's west end with pace: a bot standing on the tile beyond,
    # picking, is bonked a tile onward, once.
    west = (LANE[0] - 1, LANE[1])
    for m in see():
        if m.kind == "bot" and m.pos == west and m.busy == "pick":
            try:
                move("w")
            except ValueError:
                return False
            # It worked if the tile is empty now: a bot that moved off
            # before the bonk landed is tried again.
            runner["bonked"] = True
            for after in see():
                if after.pos == west:
                    runner["bonked"] = False
            log("bonked", m.id, runner["bonked"])
            return True
    return False


def pace():
    here = me().pos
    if runner["stage"] < len(CREW):
        crew()
        return
    if not runner["bonked"] and (here[0], here[1]) == LANE and me().pace > 0:
        if bonk():
            return
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


def lakeside(p):
    return p[1] < 0 and adjacent(p)


def dig_step():
    for p in plans("building"):
        if lakeside((p[0], p[1])):
            try:
                build(p[2], p[0], p[1])
                log("lakeside site")
            except ValueError:
                wait(1)
            return
    for m in see():
        if m.team == me().team and m.model == "site" and lakeside(m.pos):
            try:
                drop("ore")
                log("lakeside filled")
            except ValueError:
                wait(1)
            return
    for p in plans("ground"):
        here = me().pos
        if (here[0], here[1]) == (p[0] - 1, p[1]):
            try:
                dig(p[0], p[1])
                log("dug", p[0], p[1], "to", tile(p[0], p[1]).height)
            except ValueError:
                wait(1)
        else:
            try:
                move_to(p[0] - 1, p[1])
            except ValueError:
                wait(1)
        return
    wait(1)


while True:
    if me().id == RUNNER:
        pace()
        continue
    if me().id == DIGGER and (me().load["ore"] >= CAPACITY or me().load["earth"] > 0):
        dig_step()
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
