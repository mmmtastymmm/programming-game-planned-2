# The fixture's low-power hook (Q62, Q63): walk toward the nearest powered
# building of the team that holds charge, recharge until nearly full, and
# let the bot's program go on from where it was. It counts its steps, so
# a bot that cannot get there gives up long before the hook's budget does.
def on_low_power():
    tries = 0
    while tries < 30 and me().charge < 90:
        tries += 1
        home = None
        for m in see():
            if m.team == me().team and m.kind == "building" and m.model != "site":
                if m.powered and m.charge >= 10:
                    home = m.pos
                    break
        if home is None:
            return
        here = me().pos
        if abs(home[0] - here[0]) + abs(home[1] - here[1]) == 1:
            try:
                recharge()
                log("recharged to", me().charge)
            except ValueError:
                return
        else:
            try:
                move_to(home[0], home[1])
            except ValueError:
                wait(1)
