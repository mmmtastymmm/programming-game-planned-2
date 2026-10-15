"""Getting a bot from here to there.

The game's own move_to takes one greedy step: toward the target, round
nothing. It walks into dead ends behind rock and water. go_to and approach
use move_to while it makes progress, and when the bot is boxed in or starts
pacing back and forth they search for a route and follow it.

Searching is expensive. path() looks at tiles one by one, roughly 50 to 100
units for each, against a tick budget of 200, so a long search spends
several ticks of thinking (the bot still acts, a little later). The search
stays inside a box round the start and the goal (MARGIN) and gives up after
SEARCH_LIMIT tiles; raise them to find longer detours, lower them to save
budget.

Each machine keeps its own route between calls, so call go_to or approach
once per step in a loop:

    while not go_to((5, 3)):
        pass
"""

from geo import DIRS, OFFSETS, at, same, adjacent, neighbors
from act import try_move, try_move_to

# How far beyond the box round the start and the goal a route may wander.
MARGIN: int = 4
# The most tiles one search looks at before it gives up.
SEARCH_LIMIT: int = 150
# How many recent tiles to remember; standing on one again means pacing.
TRAIL: int = 6

# The route being followed. A function cannot rebind a module's name, but it
# can change a dict the name holds, so the state lives here.
route: dict = {"aim": None, "steps": [], "trail": []}


def path(
    goals: list[tuple[int, int]], limit: int = SEARCH_LIMIT
) -> list[str] | None:
    """The shortest list of directions from here to any tile in goals.

    Returns [] when already on a goal, and None when no route is found
    within the search box and limit. Ground with no building is passable;
    so is an unknown tile, so a route may run into the fog or past the
    map's edge, where a move along it fails. Other bots are ignored, since
    they move.

    Costs one `tiles()` call over the box, then roughly 50 to 100 units per
    tile searched.
    """
    start = at(me().pos)
    targets = set([at(g) for g in goals])
    if start in targets:
        return []
    if not targets:
        return None
    xs = [start[0]] + [g[0] for g in targets]
    ys = [start[1]] + [g[1] for g in targets]
    x0 = min(xs) - MARGIN
    x1 = max(xs) + MARGIN
    y0 = min(ys) - MARGIN
    y1 = max(ys) + MARGIN
    blocked = set()
    for t in tiles(x0, y0, x1, y1):
        if t.terrain != "ground" or t.building is not None:
            blocked.add((t.x, t.y))
    came = {start: None}
    frontier = [start]
    i = 0
    while i < len(frontier) and i < limit:
        p = frontier[i]
        i += 1
        for d in DIRS:
            o = OFFSETS[d]
            q = (p[0] + o[0], p[1] + o[1])
            if q in came or q in blocked:
                continue
            if q[0] < x0 or q[0] > x1 or q[1] < y0 or q[1] > y1:
                continue
            came[q] = (p, d)
            if q in targets:
                return trace(came, q)
            frontier.append(q)
    return None


def trace(came: dict, q: tuple[int, int]) -> list[str]:
    """The directions that led to q, from path()'s record of where each tile
    was reached from."""
    dirs = []
    while came[q] is not None:
        prev, d = came[q]
        dirs.append(d)
        q = prev
    dirs.reverse()
    return dirs


def forget() -> None:
    """Drop the route being followed, so the next call starts fresh."""
    route["aim"] = None
    route["steps"] = []
    route["trail"] = []


def follow(goals: list[tuple[int, int]], aim: tuple[int, int]) -> bool | None:
    """One step toward standing on any tile in goals, heading for aim.

    Does not check whether the bot is already on a goal; go_to and approach
    do that first, and are the usual way in. Returns False after a step (or
    a wait), and None when it searched and found no route.
    """
    here = at(me().pos)
    aim = at(aim)
    if route["aim"] != aim:
        forget()
        route["aim"] = aim
    steps = route["steps"]
    if steps:
        if try_move(steps.pop(0)):
            return False
        route["steps"] = []
    trail = route["trail"]
    if here not in trail:
        trail.append(here)
        if len(trail) > TRAIL:
            trail.pop(0)
        if try_move_to(aim[0], aim[1]):
            return False
    # Pacing, or boxed in: search.
    route["trail"] = []
    found = path(goals)
    if found is None:
        wait(1)
        return None
    route["steps"] = found
    if found and try_move(found.pop(0)):
        return False
    route["steps"] = []
    wait(1)
    return False


def go_to(p: tuple[int, int]) -> bool | None:
    """One step toward standing on p; True once there.

    False after a step, None when no route was found (the bot waits a tick,
    and the next call tries again).
    """
    if same(me().pos, p):
        forget()
        return True
    return follow([p], p)


def step_off() -> bool:
    """Move to any free neighboring tile, or wait a tick if there is none."""
    for d in DIRS:
        if try_move(d):
            return True
    wait(1)
    return False


def approach(p: tuple[int, int]) -> bool | None:
    """One step toward standing beside p; True once beside it.

    For working on a tile: pick, drop, build, paint and the rest reach only
    an adjacent tile. Standing on p itself counts as not beside, and steps
    off. False after a step, None when no route was found.
    """
    here = me().pos
    if same(here, p):
        step_off()
        return False
    if adjacent(here, p):
        forget()
        return True
    return follow(neighbors(p), p)
