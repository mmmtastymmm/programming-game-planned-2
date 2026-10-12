"""Splitting work between bots.

Every bot runs the same program, so left alone they all chase the same
nearest job. These functions give each bot a different answer from the same
list, using its rank: its place among the team's bots it can see, ordered
by id. The rank is the same in every bot's eyes only while they all see the
same bots, which holds for a team's own machines (see() always includes
them).
"""

from sense import machines


def rank():
    """(this bot's place among the team's bots by id, how many there are)."""
    mine = me().id
    ids = sorted([m.id for m in machines("bot")])
    return (ids.index(mine), len(ids))


def claim(items):
    """The item this bot should take: items[rank % len(items)], or None if
    items is empty.

    With fewer items than bots, several bots share an item; with more, the
    rest wait for the next round. Sort items the same way in every bot (a
    query's own order does) or the bots will disagree.
    """
    if not items:
        return None
    r = rank()
    return items[r[0] % len(items)]


def mine(items):
    """Every item this bot should take: one in every n, n the number of
    bots."""
    r = rank()
    return [x for i, x in enumerate(items) if i % r[1] == r[0]]
