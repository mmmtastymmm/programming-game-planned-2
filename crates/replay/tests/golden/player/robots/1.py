# The fixture's first red program: walk east, look around, pick from the
# first deposit found, and keep a tally that a redeploy will throw away.
steps = 0
while True:
    found = None
    for t in tiles(-30, -30, 30, 30):
        if t.deposit is not None and t.deposit["ore"] > 0:
            found = (t.x, t.y)
            break
    here = me().pos
    if found is not None and abs(found[0] - here[0]) + abs(found[1] - here[1]) == 1:
        try:
            pick("ore")
            log("picked", me().load["ore"])
        except ValueError as e:
            wait(1)
        continue
    try:
        if found is None:
            move("e")
        else:
            move_to(found[0], found[1])
        steps += 1
    except ValueError:
        wait(2)
    if steps % 4 == 0:
        log("steps", steps, "sees", len(see()), "hears", len(hear()))
