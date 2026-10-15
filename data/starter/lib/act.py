"""Actions that report failure instead of raising.

Every action builtin raises ValueError when it cannot start: the tile is
taken, nothing is adjacent, the bot is already busy. Each function here
tries the action and returns True if it ran, False if it could not, so a
program can write `if not try_pick("ore"): ...` instead of a try block.

A failed action does nothing and costs no game time, so a loop that keeps
failing spins through its budget: wait a tick (`wait(1)`) when there is
nothing better to do.
"""


def try_move(d: str) -> bool:
    """Step one tile in direction d ("n", "e", "s" or "w")."""
    try:
        move(d)
        return True
    except ValueError:
        return False


def try_move_to(x: int, y: int) -> bool:
    """Take move_to's one greedy step toward (x, y)."""
    try:
        move_to(x, y)
        return True
    except ValueError:
        return False


def try_pick(kind: str) -> bool:
    """Pick up kind ("ore") from an adjacent deposit or depot."""
    try:
        pick(kind)
        return True
    except ValueError:
        return False


def try_drop(kind: str) -> bool:
    """Drop kind into an adjacent building or site with room for it."""
    try:
        drop(kind)
        return True
    except ValueError:
        return False


def try_build(model: str, x: int, y: int) -> bool:
    """Place a site for model at (x, y), which must be adjacent."""
    try:
        build(model, x, y)
        return True
    except ValueError:
        return False


def try_deconstruct(x: int, y: int) -> bool:
    """Take down the adjacent building or site at (x, y)."""
    try:
        deconstruct(x, y)
        return True
    except ValueError:
        return False


def try_paint(x: int, y: int) -> bool:
    """Carry out the team's paint plan on the adjacent tile (x, y)."""
    try:
        paint(x, y)
        return True
    except ValueError:
        return False


def try_unpaint(x: int, y: int) -> bool:
    """Remove the paint from the adjacent tile (x, y)."""
    try:
        unpaint(x, y)
        return True
    except ValueError:
        return False


def try_overlay(x: int, y: int) -> bool:
    """Carry out the team's overlay plan on the adjacent tile (x, y)."""
    try:
        overlay(x, y)
        return True
    except ValueError:
        return False


def try_unoverlay(x: int, y: int) -> bool:
    """Remove the overlay from the adjacent tile (x, y)."""
    try:
        unoverlay(x, y)
        return True
    except ValueError:
        return False
