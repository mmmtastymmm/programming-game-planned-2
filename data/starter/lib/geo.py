"""Grid geometry: positions, directions and distances.

A position is an (x, y) pair: a tuple you write, or a machine's `pos`.
North is +y and east is +x. Nothing here asks the game anything, so each
function costs only its own handful of operations.

    from geo import step, adjacent
    ahead = step(me().pos, "n")
"""

# The four directions, in the order move_to breaks ties.
DIRS = ["n", "e", "s", "w"]

# How one step in each direction changes (x, y).
OFFSETS = {"n": (0, 1), "e": (1, 0), "s": (0, -1), "w": (-1, 0)}

# The direction that undoes each one.
OPPOSITE = {"n": "s", "e": "w", "s": "n", "w": "e"}


def at(p):
    """p as a plain (x, y) tuple, so it can be compared or used as a key."""
    return (p[0], p[1])


def same(a, b):
    """True if a and b are the same tile."""
    return a[0] == b[0] and a[1] == b[1]


def step(p, d):
    """The tile one step from p in direction d: "n", "e", "s" or "w"."""
    o = OFFSETS[d]
    return (p[0] + o[0], p[1] + o[1])


def neighbors(p):
    """The four tiles next to p, in DIRS order."""
    return [step(p, d) for d in DIRS]


def dist2(a, b):
    """The squared straight-line distance from a to b.

    Squared, because the language has no square root, and comparing squared
    distances orders tiles the same way. This is the distance `see()` sorts
    by and vision ranges use.
    """
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def manhattan(a, b):
    """How many single steps from a to b, if nothing is in the way."""
    return abs(a[0] - b[0]) + abs(a[1] - b[1])


def adjacent(a, b):
    """True if a and b share an edge: where pick, drop, build and paint
    reach."""
    return manhattan(a, b) == 1


def direction_to(a, b):
    """The direction of the first step from a toward b, or None if a is b.

    Steps along the axis with the larger gap; on a tie, north or south first.
    """
    dx = b[0] - a[0]
    dy = b[1] - a[1]
    if dx == 0 and dy == 0:
        return None
    if abs(dy) >= abs(dx):
        return "n" if dy > 0 else "s"
    return "e" if dx > 0 else "w"
