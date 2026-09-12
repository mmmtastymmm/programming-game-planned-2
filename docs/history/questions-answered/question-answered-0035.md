*Closed record — see [../README.md](../README.md). Not spec.*

# Q35 — What does a bot's body color mean?

## Ruling

*Ruling (2026-09-12):* **its deployment.** A `red` bot is red on every
team, from the same eight-color list the deployments are named from
(`02`, Q24). The team is a ring under every machine — the player's own
white, each other team's a color from `data/interface.toml` — and a
printer, a depot or a site, which has no color deployment, wears the
team's ring color on its body.

### The rules

1. **Body = deployment color** for bots, one atlas per color.
2. **Ring = team**, on every machine, the player's white.
3. **Buildings and printers = team color** on the body, and the ring too.
4. **The team palette is the renderer's**, in `data/interface.toml`, and
   distinct from the deployment colors only by being a ring rather than a
   body: the eye reads a body first and a base second, and a match has
   more deployments than teams.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — Machines on the map
  and its Decided entry. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.
- **Task:** [T17](../../TASKS.md) — the atlases per color and the ring.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Body is the deployment** *(chosen)* | The map shows which program each bot runs, which is what the player edits; team needs a second cue, the ring. The predecessor did the same, coloring bodies by role. |
| Body is the team | Friend and foe at a glance, but every bot of a team looks alike and the player cannot see which of their programs a bot runs without clicking it. |
| Body is the team, a badge is the deployment | The badge is small at the distances the camera sits; the same information, harder to read. |

### How the answer took its shape

A deployment is *named* by a color (Q24), so drawing a bot in any other
color would be a contradiction the player meets in their first minute.
