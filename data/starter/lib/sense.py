"""What the team knows about the map and its machines.

Each function here calls a sensing builtin, and those cost more for every
record they return: `tiles()` and `see()` charge one unit per tile or
machine. On a big map, call `seen_tiles()` once and reuse the list rather
than calling it in a loop.

A tile the team has never seen is "unknown": `tile(x, y)` returns None for
it, and `tiles(...)` leaves it out.
"""

from geo import dist2

# Wide enough to cover any map: tiles() clips the rectangle to the map.
FAR: int = 1000


def seen_tiles() -> list[Tile]:
    """Every tile the team sees or remembers, sorted by row then column.

    Costs one unit per tile returned, so it grows as the team explores.
    """
    return tiles(-FAR, -FAR, FAR, FAR)


def seen_bounds() -> tuple[int, int, int, int]:
    """The smallest (x0, y0, x1, y1) box around every tile the team has
    seen."""
    seen = seen_tiles()
    xs = [t.x for t in seen]
    ys = [t.y for t in seen]
    return (min(xs), min(ys), max(xs), max(ys))


def known(x: int, y: int) -> bool:
    """True if the team sees or remembers the tile at (x, y)."""
    return tile(x, y) is not None


def passable(x: int, y: int) -> bool:
    """True if a bot could stand on (x, y), as far as the team knows: ground
    with no building on it.

    An unknown tile counts as passable, so a route may lead into the fog;
    another bot standing there does not count, since it will move.
    """
    t = tile(x, y)
    return t is None or (t.terrain == "ground" and t.building is None)


def buildable(x: int, y: int) -> bool:
    """True if a building could go on (x, y): ground, no deposit and no
    building.

    An unknown tile counts as buildable, since a plan may sit there.
    """
    t = tile(x, y)
    return t is None or (
        t.terrain == "ground" and t.deposit is None and t.building is None
    )


def has_building(x: int, y: int) -> bool:
    """True if (x, y) holds a building or a site, or is unknown."""
    t = tile(x, y)
    return t is None or t.building is not None


def nearest(points: list[tuple[int, int]]) -> tuple[int, int] | None:
    """The point in points closest to this machine, or None if there are
    none.

    Ties keep the earlier point, so the result never depends on luck.
    """
    here = me().pos
    best: tuple[int, tuple[int, int]] | None = None
    for p in points:
        d = dist2(p, here)
        if best is None or d < best[0]:
            best = (d, p)
    return None if best is None else best[1]


def machines(model: str | None = None, team: int | None = None) -> list[Machine]:
    """The machines the team sees, nearest first.

    model: only this model ("bot", "printer", "depot" or "site").
    team: only this team's machines; None means this machine's own team.
    Pass team=-1 for every team.
    """
    if team is None:
        team = me().team
    found = []
    for m in see():
        if (model is None or m.model == model) and (team == -1 or m.team == team):
            found.append(m)
    return found


def ore_sources() -> list[tuple[int, int]]:
    """Every place a bot can pick ore: deposits with ore left, and the
    team's depots holding some. Costs a pass over every seen tile."""
    found = []
    for t in seen_tiles():
        if t.deposit is not None and t.deposit["ore"] > 0:
            found.append((t.x, t.y))
    for m in machines("depot"):
        if m.store["ore"] > 0:
            found.append(m.pos)
    return found
