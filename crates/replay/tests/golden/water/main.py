# The fixture's waterworks program (Q67), deployed to `bot` once the
# planned printer's site is full and the haulers have nothing left to
# haul to. In the ticks left before the guards take over, the bots lay a
# line of wire south from the base, build a tank at its end, a pump west
# of the tank, beside the flooded hollow, and a spout two tiles east of
# it, join the spout to the tank with a tile of wire and of pipe, and dig
# the hollow the spout pours into. Each bot has one of the three sites as
# its own and helps with the others once its own is full. The digger
# goes on as it was, and the low-power hook is the one bot.py had.
#
# A bot has two hundred cost units a tick (docs/01), so a hauler's turn
# of the loop below is written to fit one step in them: what a bot is and
# where it works are read once, and it walks no list of machines or
# tiles.
CAPACITY = 10
ORE = (-10, 5)
# Each site, and the tile north of it a bot stands on to build and fill
# it: a row of tiles a tile north of the sites', so that no bot waits on
# a tile something is planned for.
TANK = (-9, -2)
PUMP = (-10, -2)
SPOUT = (-7, -2)
SITES = [TANK, PUMP, SPOUT]
# The tile between the tank and the spout, which takes a wire and a pipe,
# and the hollow east of the spout.
LINE = (-8, -2)
HOLLOW = (-6, -2)
# The way south: the planned printer and the rocks east of it stand
# between the base and the sites, so a bot north of them makes first for
# the gap to their west.
GAP = (-10, 0)
ME = me().id
# The ninth bot the first printer printed digs, as in bot.py; the bot
# that lays the wire down the column from the base; and the one that
# lays the line's wire and pipe and then digs the hollow.
DIGGER = ME == 25
WIRER = ME == 5
LINER = ME == 11
HOME = {3: SPOUT, 5: PUMP, 8: PUMP, 11: SPOUT, 14: SPOUT, 23: PUMP}.get(ME, TANK)
state = {"deliver": me().load["ore"] >= CAPACITY, "lines": WIRER or LINER}


def wants(p):
    # The model planned on `p`, or "site" for its site while that is
    # short of ore, or None once it is full or built.
    t = tile(p[0], p[1])
    kind = t.plans["building"]
    if kind is not None:
        return kind
    b = t.building
    if b is not None and b.model == "site" and b.busy != "build":
        return "site"
    return None


def toward(pos, x, y):
    # A step toward (x, y); southbound, by way of the gap from north of it.
    try:
        if y <= 0 and pos[1] > 0 and pos[0] < -6:
            move_to(GAP[0], GAP[1])
        else:
            move_to(x, y)
    except ValueError:
        wait(1)


def haul(pos, ore):
    if ore >= CAPACITY:
        state["deliver"] = True
    elif ore <= 0:
        state["deliver"] = False
    target = HOME
    kind = wants(target)
    if kind is None:
        for p in SITES:
            kind = wants(p)
            if kind is not None:
                target = p
                break
    if kind is None:
        wait(5)
        return
    if not state["deliver"]:
        if abs(ORE[0] - pos[0]) + abs(ORE[1] - pos[1]) == 1:
            try:
                pick("ore")
            except ValueError:
                wait(1)
        else:
            toward(pos, ORE[0], ORE[1])
        return
    if pos[0] != target[0] or pos[1] != target[1] + 1:
        toward(pos, target[0], target[1] + 1)
        return
    try:
        if kind == "site":
            drop("ore")
            log("water dropped, left", me().load["ore"])
        else:
            build(kind, target[0], target[1])
            log("water site", kind)
    except ValueError:
        wait(1)


def wire_column(pos):
    # The wire plans in the tank's column, north to south, each laid from
    # the tile west of it.
    for p in plans("wire"):
        if p[0] == TANK[0]:
            if pos[0] == p[0] - 1 and pos[1] == p[1]:
                try:
                    wire(p[0], p[1])
                    log("water wired", p[1])
                except ValueError:
                    wait(1)
            else:
                toward(pos, p[0] - 1, p[1])
            return
    state["lines"] = False


def line(pos):
    # The line's wire, then its pipe, from the tile north of it; then
    # what is left of the load to a site; then the hollow, from the tile
    # north of that.
    plan = tile(LINE[0], LINE[1]).plans
    if plan["wire"] or plan["pipe"]:
        if pos[0] != LINE[0] or pos[1] != LINE[1] + 1:
            toward(pos, LINE[0], LINE[1] + 1)
            return
        try:
            if plan["wire"]:
                wire(LINE[0], LINE[1])
            else:
                pipe(LINE[0], LINE[1])
            log("water line", tile(LINE[0], LINE[1]).wires, tile(LINE[0], LINE[1]).pipes)
        except ValueError:
            wait(1)
        return
    if me().load["ore"] > 0:
        haul(pos, me().load["ore"])
        return
    if tile(HOLLOW[0], HOLLOW[1]).height < tile(HOLLOW[0], HOLLOW[1] + 1).height:
        state["lines"] = False
        return
    if pos[0] != HOLLOW[0] or pos[1] != HOLLOW[1] + 1:
        toward(pos, HOLLOW[0], HOLLOW[1] + 1)
        return
    try:
        dig(HOLLOW[0], HOLLOW[1])
        log("water dug", tile(HOLLOW[0], HOLLOW[1]).height)
    except ValueError:
        wait(1)


def lakeside(p):
    h = me().pos
    return p[1] < 0 and abs(p[0] - h[0]) + abs(p[1] - h[1]) == 1


def dig_step():
    # bot.py's, as it was.
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
        h = me().pos
        if (h[0], h[1]) == (p[0] - 1, p[1]):
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
    if DIGGER:
        dig_step()
        continue
    m = me()
    if state["lines"]:
        if WIRER:
            wire_column(m.pos)
        else:
            line(m.pos)
        continue
    haul(m.pos, m.load["ore"])
