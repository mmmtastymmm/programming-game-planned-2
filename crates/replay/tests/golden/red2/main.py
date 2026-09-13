# The fixture's second red program, deployed mid-match (Q11): every bot on
# the color takes it at its next boundary. It stops hauling and stands
# guard, attacking anything of another team it can see.
class Guard:
    def __init__(self):
        self.hits = 0

    def target(self):
        mine = me().team
        for m in see():
            if m.team != mine:
                return m
        return None

guard = Guard()
log("guard online", me().name)
while True:
    t = guard.target()
    if t is None:
        try:
            move("w")
        except ValueError:
            wait(3)
        continue
    try:
        attack(t.id)
        guard.hits += 1
        log("attacked", t.id, "hits", guard.hits)
    except ValueError:
        try:
            move_to(t.pos[0], t.pos[1])
        except ValueError:
            wait(1)
