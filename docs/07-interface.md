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
  a working copy of every deployment's bundle, edited in a window over the
  map (Q39 amended Q33's panel), and deployed from there as a `Deploy`
  command carrying the working copy's files. The programs directory is
  where a match's opening bundles are read from and where the player
  exports to; the game never watches it.
- **Everything floats over the map, as windows (Q39, amending Q33 and the
  screen as first ruled).** One window per deployment's program, one each
  for the inspector, the tools and the log, dragged, resized, collapsed and
  closed by the player; the time bar is the only fixed strip and is the
  dock that opens each; the layout is remembered in `windows.toml` beside
  the programs.
- **A start screen, an end screen, and the replay always saved (Q36).**
  The map, the cards, the programs directory and play, host or join are
  picked in the game; the end names the winner or the draw, the tick, the
  hash and the replay's path; the replay is written as
  `replays/<map>-<tick>-<hash>.ron` inside the programs directory whenever
  a match ends or desyncs; every option is also a command-line flag, which
  skips the screen.
- **The mark tools are a strip, a tool stays armed, a drag marks, and a
  plan previews (Q37).** Rows of building models, paint colors and overlay
  labels each ending in a clear, `B`/`P`/`O` and `1`–`9` to pick, `Esc` to
  disarm; a drag marks every tile it crosses once, `Shift`+click withdraws
  the armed kind; the player's own plans show as a translucent version of
  their effect, and the armed tool as a ghost under the cursor.
- **A stall is a banner with a resign; a desync freezes the match with
  the report and the replay saved (Q38).** The banner names the peer, the
  tick and the wait; everything else keeps working. A desync's banner
  carries the report and the replay's path, and *leave* goes to the end
  screen.
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

The map fills the window; everything else floats over it (Q39):

| Window | Holds | Ruled in |
|---|---|---|
| **the time bar** — the one fixed strip, across the top | the tick, the speed and its `−`/`+`, the delay, the state hash, the peers, a stall or desync report, the match's end; and the dock: a toggle per window below | Q6, Q12, `06`, Q39 |
| **one per deployment** | the file being edited, its running version against its working copy, the deploy and export buttons; the deployment's fault summary | Q33, Q34, Q39 |
| **inspector** | the hovered tile as the team knows it, the selected machine's record and fault, the teams and their deployments' versions | Q34 |
| **tools** | the mark strip and resign | Q37 |
| **log** | machines' `log` lines, dropped commands, exchange events, the match's end | `02`, `06` |

Every window drags, resizes, collapses and closes, and several programs may
be open at once. The layout is saved to `windows.toml` beside the programs
whenever it changes and restored at start; nothing about it reaches a
peer. The camera orbits, pans and zooms under the windows and never
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

The tools window is a strip (Q37): a row of the building models a plan may
name (`printer` excluded, Q31), a row of the eight paint colors, a row of
the overlay labels in data (Q10, Q26, Q27), each row ending in a *clear* —
the plan to empty that slot. `B`, `P` and `O` pick a row and `1`–`9` an
entry; a click arms a tool, `Esc` or a second click disarms it.

With a tool armed, a left click on a tile is a `Mark` of that kind and
value; a left drag marks every tile the cursor crosses, once per drag,
and does not pan the camera; `Shift`+click is an `Unmark` of the armed
kind. With none armed, a click selects and a drag pans. The renderer
submits and forgets (Q15): what the player sees is the plan the sim
reports, `delay` ticks later.

A tile carrying the player's team's plan shows it as a translucent version
of its effect — the color at half alpha, the overlay's label, a ghost of
the building; a clear plan hatches the slot — and the armed tool shows the
same ghost under the cursor. Only the player's team's plans are drawn,
since plans are private (`03`), and the snapshot carries that team's plan
values per tile for it: a read, in no hash.

## The match

**Before the first tick** (Q36) the start screen lists the maps in
`data/maps` and the cards in `data/opposition` (`04`), takes the programs
directory (Q33), and offers *play* alone against the cards, *host* on an
address, or *join* one (`06`). Starting reads the programs in and builds
the driver as the command line does; every choice is also a flag, and a
flag skips the screen, so a headless run and CI need none.

**After the last** the end screen names the winner or the draw (`04`), the
ending tick, the final hash and the replay's path; *again* returns to the
start, *quit* leaves. **The replay is always saved** when a match ends or
desyncs: `(map, command log)` in `crates/replay`'s RON, as
`replays/<map>-<tick>-<hash>.ron` inside the programs directory, named by
the tick and hash so two peers that agree write the same name.

**Stalled** (Q38) — past `stall_report_ticks`, `06` — a banner over the
map names the peer awaited, the tick and the seconds waited, with a resign
button beside it; the windows, the editor and the marks keep working, and a
submission lands on the first tick still open. The banner clears when the
set arrives. **Desynced**, the match is frozen where the driver stopped it:
a red banner with the report — the tick, this peer's hash, the other's —
the replay written at once with the desync tick and this peer's hash in its
name, the path in the banner, and *leave* to the end screen, which says the
match ended in a desync. A lost peer is neither: its sets stop being
awaited and the match continues, with a line in the log.

## Costs

Every number this doc names is a tuning constant in `data/interface.toml`:
`fault_mark_ticks`, and the team ring colors. None is a sim value.

## What this doc leaves open

Nothing, for now. Dropping a late peer on a timeout, if PvP wants it, is
a `docs/06` rule under a new number, not an interface one (Q38).
