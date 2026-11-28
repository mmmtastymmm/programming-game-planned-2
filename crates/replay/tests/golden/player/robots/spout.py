# The fixture's player spout (Q67): it pours east while the tile east of
# it is a hollow — lower ground than its own — holding less than most of
# a level, and nowhere otherwise, so the hollow fills and does not run
# over.
FULL = 0.8
while True:
    here = me().pos
    mine = tile(here[0], here[1])
    east = tile(here[0] + 1, here[1])
    if east is not None and east.height < mine.height and east.water < FULL:
        if me().pouring != "e":
            pour("e")
            log("pouring", me().pouring)
    elif me().pouring is not None:
        pour(None)
        log("pouring", me().pouring)
    wait(1)
