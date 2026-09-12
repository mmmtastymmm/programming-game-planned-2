# 07 — Interface

What the player sees and touches: the screen a match is played on, the
editor the programs are written in, how a fault reaches the player, and how
a machine on the map reads. This doc is normative for the renderer's
choices that are not sim rules — everything here is on the far side of the
one-way arrows [06-architecture](06-architecture.md) draws: the renderer
reads the completed-tick snapshot and writes only commands (Q15). Nothing
here changes a hash; a peer with a different renderer sees the same match.

## Decided

- **Programs are written in the game (Q33).** The renderer holds an editor:
  a working copy of every deployment's bundle, edited in a panel beside the
  map, and deployed from there as a `Deploy` command carrying the working
  copy's files. The programs directory is where a match's opening bundles
  are read from and where the player exports to; the game never watches it.
- **A fault reaches the player on every channel (Q34).** A mark over the
  machine while its fault record is fresh, a line per deployment counting
  faults by file and line in the programs panel, the full record in the
  inspector, and a line in the log. The record itself is the sim's (Q20);
  the renderer only shows it.
- **A bot's body is its deployment's color, and its team is a ring (Q35).**
  A `red` bot is red on every team. The team shows as a ring under every
  machine, the player's own in white and each other team's in a color from
  `data/interface.toml`. Printers, depots and sites, which have no color
  deployment, wear the team's ring color on their body.

## The screen

One window, four regions around the map:

| Region | Holds | Ruled in |
|---|---|---|
| **top bar** | the tick, the speed and its `−`/`+`, the delay, the state hash, the peers, a stall or desync report, the match's end | Q6, Q12, `06` |
| **left** | the editor: a tab per deployment, the file being edited, its running version against its working copy, and the deploy button; the fault summary per deployment | Q33, Q34 |
| **right** | the inspector: the hovered tile as the team knows it, the selected machine's record and fault, the teams and their deployments' versions; the mark tools | Q34, open — `Q37` |
| **bottom** | the log: machines' `log` lines, dropped commands, exchange events, the match's end | `02`, `06` |

The map is what remains. The camera orbits, pans and zooms and never
matters to the sim.

## Programs and the editor

A deployment's **working copy** is the renderer's: the files of its bundle,
as text, edited in place. A deployment's **running version** is the sim's
(`02`): the bundle the deployment slot holds, identified by its hash (Q13).
The two meet only through `Deploy`.

- **Deploying** sends the working copy as the bundle of one `Deploy`
  command, agreed `delay` ticks later like any other (Q12). The editor
  shows the working copy as *ahead* of the running version until the
  sim reports the new hash in the snapshot, and machines take it at their
  next boundary by the swap rule (Q11), so bots on the old version are
  expected for a while and the inspector says which version each runs.
- **The opening set** (Q9) is read from the programs directory at start,
  one bundle per deployment directory, and becomes both the opening
  `Deploy`s and the initial working copies. A deployment with no directory
  starts with an empty working copy and no opening deploy.
- **Export** writes every working copy back to the programs directory, on
  the player's request, and never on its own: the directory is the player's
  editor-independent store, and what they wrote there is not overwritten
  by a match they lost.
- **A bundle that fails to load** (`01`) is refused at submission, as every
  command is (`06`), and the error is shown at the file and line in the
  editor; nothing enters the log.
- Highlighting, completion and the editor's keys are the build's choices
  and not rules; they change nothing a peer sees.

## Faults

A machine's fault record (`02`, Q20) carries the exception, the file and
line, the tick, and the version it ran. The renderer shows it four ways,
so a player watching the map, the code, one machine or the log each learns
of it:

| Channel | Shows | For how long |
|---|---|---|
| **the mark** | a scribble over the machine | `fault_mark_ticks` after the record's tick, from `data/interface.toml` |
| **the summary** | per deployment, in the editor's panel: how many of its machines' latest records name each file and line, newest first | while the records stand |
| **the inspector** | the selected machine's full record, and the version it ran against the running version | while selected |
| **the log** | one line per new record: the machine, the exception, the file and line | until the log scrolls it out |

A record survives a redeploy (Q20), so a summary line for a file and line
the player has since edited is still shown, with the version it names, and
clears only when those machines fault again or die.

## Machines on the map

- **Body:** a bot is an atlas cube in its deployment's color, from the same
  eight-color list the deployments are named from (`02`); a printer, a
  depot and a site are their own shapes in the team's ring color.
- **Ring:** every machine stands on a ring in its team's color, the
  player's own team white.
- **Health:** a bar over the machine for a few seconds after any change.
- **Fault:** the mark above.
- **Fog:** as Q21 rules — a machine on a remembered tile is drawn as last
  seen, dimmed; on an unknown tile, not at all.

## Marks

The mark tools — a building model, one of the eight paint colors, one of
the overlay labels in data, a clear, a withdrawal — become `Mark` and
`Unmark` commands on the tile clicked (Q10, Q26, Q27). How the tools are
picked and how the team's own plans read on the map is open — `Q37`.

## The match

What the screen shows before the first tick and after the last — picking a
map and an opposition, hosting or joining, the winner, saving the replay —
is open — `Q36`. What the player may do while stalled on a peer or after a
desync is open — `Q38`.

## Costs

Every number this doc names is a tuning constant in `data/interface.toml`:
`fault_mark_ticks`, and the team ring colors. None is a sim value.

## What this doc leaves open

- **The match's start and end screens** — `Q36`.
- **The mark tools' ergonomics** — `Q37`.
- **The stall and the desync in the interface** — `Q38`.
