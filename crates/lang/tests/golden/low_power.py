# The low-power fixture (Q62, Q63, T39): the fifth interrupt kind, whose
# hook runs with main flow set aside and returns to it, and a program held
# before a call the host is not ready for. The scripted host keeps a
# battery — one unit a tick, three for a `move` — refuses a `move` in main
# flow or in on_low_power that the battery does not cover, and raises
# low_power and a redeploy on fixed ticks (`golden.rs` holds the schedule).
done = []
t = wait()
log("main", t)
n = 0
while n < 3:
    move("n")
    n += 1
    log("moved", n, len(done))

def on_low_power():
    log("on_low_power starts", len(done))
    move("s")
    t = wait()
    if t == 15:
        x = [1, 2][t]
    done.append(t)
    log("on_low_power done", t)

def on_fault(e):
    log("on_fault", str(e), e.line, e.tick)
