# The interrupt fixture: the paths a match replay is least likely to cover.
# A scripted host raises dying, death and redeploy on fixed ticks
# (`golden.rs` holds the schedules) and answers every wait with the tick.
# Each run of main flow faults; which way depends on the tick it started.
t = wait()
log("main", t)
try:
    if t % 4 == 0:
        raise KeyError("unwound")
    x = [1, 2][t]
finally:
    i = 0
    limit = 300 if t % 4 == 0 else 40
    while i < limit:
        i += 1
    log("finally", t)

def on_fault(e):
    log("on_fault", str(e), e.line, e.tick)
    i = 0
    while i < e.line * 60:
        i += 1
    log("on_fault done")

def on_dying():
    log("on_dying starts")
    t = wait()
    log("on_dying resumed", t)
