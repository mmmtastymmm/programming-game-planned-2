*Closed record — see [../README.md](../README.md). Not spec.*

# Q68 — Should ground on a line of sight block it when it is no higher than the seer?

## Ruling

*Ruling (2026-11-28):* **no. A tile on the ray blocks only if it is both
higher than the ground the seer stands on and above the straight line
between the two grounds.** Made under a new number because
[Q65](question-answered-0065.md), which it amends, is a closed record,
and opened and answered in one commit, as T44 built Q65's rule and
showed what it hid.

### The rule

It is the sim's and is hash-affecting. With the walk from `a` to `b`
taking `n` steps and the tile at step `k` one the ray passes through,
that tile blocks if `height(tile) > height(a)` and
`height(tile) × n > height(a) × (n − k) + height(b) × k`. Q65 had the
second test alone. Range is as Q65 left it: vision plus the seer's
height less the seen tile's.

### Consequences

- **Lower ground is seen from a distance.** A machine on level ground
  sees the bed of a lake or a river two tiles off, past the bank, where
  under Q65 only a machine on the bank did.
- **Looking up is as it was.** From in a hollow the bank is higher than
  the seer and hides what is beyond it, so a machine in a pit still sees
  only the tiles round it.
- **Sight is less symmetric.** What stands on a plain sees a bot in a
  hollow that cannot see it.
- **A ridge still hides.** Ground higher than the seer hides what is
  lower behind it.

## Outcome

- **Docs:** [03-world.md](../../03-world.md) — a Q68 Decided entry and
  Height on the line. [00-overview.md](../../00-overview.md) — the
  Decided listing. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T44](../../TASKS.md), amended — sight over height is built
  with this test, in the commit that brings this ruling.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| ground at the seer's height on a line down | Blocks, being above the line *(Q65)* | Low ground is seen only from its rim: a printer cannot see the river beside it. |
| ground at the seer's height on a line down | **Never blocks** *(chosen)* | A second test on every tile of every ray; a machine in a hollow is seen by what it cannot see. |
| ground at the seer's height on a line down | An eye height above the ground | A number to tune, and it lets a machine see over a rise of a level as well. |

### How the answer took its shape

T44 built Q65's test to the letter and reported what it did on the
maps: on `falls` each printer lost sight of the river bed in front of
it. The user kept the task and changed the rule in a sentence: a
machine should always see ground lower than itself within its range,
with less sight up and more down. That ground higher than the seer
still blocks what is lower behind it is the writer's reading of
"within a range", and the one the tests hold.
