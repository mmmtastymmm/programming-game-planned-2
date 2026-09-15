# 07 — Interface

What the player sees and touches: the screen a match is played on, the
editor the programs are written in, how a fault reaches the player, and how
a machine on the map reads. This doc is normative for the renderer's
choices that are not sim rules — everything here is on the far side of the
one-way arrows [06-architecture](06-architecture.md) draws: the renderer
reads the completed-tick snapshot and writes only commands (Q15). Nothing
here changes a hash; a peer with a different renderer sees the same match.

## Decided

- **Programs are written in the game (Q33, as Q39 and Q41 amended).** The
  renderer holds the team's program tree — the programs directory, read at
  start and exported on request, never watched — and composes each
  deployment's bundle from it; a file is edited in a window over the map
  and deployed as one `Deploy` per deployment its bundle reaches.
- **The programs are one tree, shown as a file tree (Q41).** `robots/`
  with a file per deployment, `interrupts/` with `on_fault.py` and
  `on_dying.py`, both fixed, and the player's own files and folders,
  shared by every robot; a fixed panel on the right shows it and a click
  opens a file as a window; the top bar keeps the status alone. A hook
  binds from the robot's file if it defines one, else from the interrupt
  file (`01`).
- **Everything floats over the map, as windows (Q39, amending Q33 and the
  screen as first ruled; Q41 moved the dock into the tree).** One window
  per open file, one each for the inspector, the tools and the log,
  dragged, resized, collapsed and closed by the player; the tree panel
  opens each; the layout is remembered in `windows.toml` beside the
  programs.
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
- **A bot's body is its deployment's color, and its team is a ring (Q35,
  as Q40 amended).** Deployment `n`'s bots wear the `n`-th paint color,
  cycling through the list, and every bot wears its number over its body,
  so deployment `9` is the first color again with a `9` on it. The team
  shows as a ring under every machine, the player's own in white and each
  other team's in a color from `data/interface.toml`. Printers, depots and
  sites, which have no numbered deployment, wear the team's ring color on
  their body.

## The screen

The map fills the window; two fixed strips frame it and everything else
floats over it (Q39, Q41):

| Window | Holds | Ruled in |
|---|---|---|
| **the time bar** — fixed, across the top | the tick, the speed and its `−`/`+`, the delay, the state hash, the peers, a stall or desync report, the match's end; nothing else | Q6, Q12, `06`, Q41 |
| **the tree** — fixed, down the right | `robots`, `interrupts`, then the player's folders and files, each marked ahead, error or open; new file and folder, rename, delete on the player's; at its foot the inspector, the tools and the log | Q41 |
| **one per open file** | the file being edited, the versions it is deployed as, the deploy and export buttons; the faults naming this file | Q33, Q34, Q39, Q41 |
| **inspector** | the hovered tile as the team knows it, the selected machine's record and fault, the teams and their deployments' versions | Q34 |
| **tools** | the mark strip and resign | Q37 |
| **log** | machines' `log` lines, dropped commands, exchange events, the match's end | `02`, `06` |

Every window drags, resizes, collapses and closes, and several files may
be open at once. The layout is saved to `windows.toml` beside the programs
whenever it changes and restored at start; nothing about it reaches a
peer. The camera orbits, pans and zooms under the windows and never
matters to the sim.

## Programs and the editor

The team's **tree** is the renderer's (Q41): the programs directory as
text, edited in place. A deployment's **running version** is the sim's
(`02`): the bundle the deployment slot holds, identified by its hash (Q13).
The two meet only through `Deploy`.

- **The tree** holds `robots/` — `1.py`, `2.py`, … for the deployments the
  team can write for (Q40), `printer.py`, `depot.py` — and `interrupts/`
  with `on_fault.py` and `on_dying.py`; both folders and every file in
  them always exist and cannot be renamed, moved or deleted. Everything
  else is the player's: files and folders they add, rename, move and
  delete, shared by every robot. A file's name is `[a-z_][a-z0-9_]*.py`
  (`01`) and its stem is unique across the tree, since folders are
  organisational only and the language has no packages; a collision is a
  load error shown in the tree.
- **Composition.** Deployment `d`'s bundle is `robots/d.py` as `main.py`,
  plus every interrupt file and every player file by its bare name. A hook
  binds from `robots/d.py` if it defines one, else from the interrupt file
  (`01`, Q41).
- **Deploying** a file sends one `Deploy` per deployment whose bundle it
  reaches — a robot file its own, a shared or interrupt file every
  deployment's — each agreed `delay` ticks later (Q12). A file shows as
  *ahead* until the sim reports every reached deployment's new hash in the
  snapshot; machines take it at their next boundary by the swap rule
  (Q11), so bots on the old version are expected for a while and the
  inspector says which version each runs.
- **The opening set** (Q9) is composed from the tree at start, one bundle
  per deployment with a non-empty robot file, and becomes the opening
  `Deploy`s. The cards (`04`) use the same layout; a script's
  `bundle = "1"` names the deployment whose bundle its tree composes.
- **Export** writes the tree back to the programs directory, on the
  player's request, and never on its own: the directory is the player's
  editor-independent store, and what they wrote there is not overwritten
  by a match they lost.
- **A bundle that fails to load** (`01`) is refused at submission, as every
  command is (`06`), and the error is shown at the file and line in its
  window and marked in the tree; nothing enters the log.
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
| **the summary** | per file, in its window: how many machines' latest records name each of its lines, across every deployment the file reaches, newest first | while the records stand |
| **the inspector** | the selected machine's full record, and the version it ran against the running version | while selected |
| **the log** | one line per new record: the machine, the exception, the file and line | until the log scrolls it out |

A record survives a redeploy (Q20), so a summary line for a file and line
the player has since edited is still shown, with the version it names, and
clears only when those machines fault again or die.

## Machines on the map

- **Body:** a bot is an atlas cube in the color its deployment's number
  selects from the paint list, cycling (Q40); a printer, a depot and a site
  are their own shapes in the team's ring color.
- **Number:** every bot wears its deployment's number over its body,
  always, drawn by the renderer (Q40).
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
