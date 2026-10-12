# 07 — Interface

What the player sees and touches: the screen a match is played on, the
editor the programs are written in, how a fault reaches the player, and how
a machine on the map reads. This doc is normative for the renderer's
choices that are not sim rules — everything here is on the far side of the
one-way arrows [06-architecture](06-architecture.md) draws: the renderer
reads the completed-tick snapshot and writes only commands (Q15). The
debugger alone also passes the match's sim breakpoint watches, reads its
hit reports and saved states, and drives forks, private copies that never
reach the match (Q52). Nothing here changes a hash; a peer with a
different renderer sees the same match.

## Decided

- **Programs are written in the game (Q33, as Q39 and Q41 amended).** The
  renderer holds the team's program tree — the programs directory, read at
  start and exported on request, never watched — and composes each
  deployment's bundle from it; a file is edited in a window over the map
  and deployed as one `Deploy` per deployment its bundle reaches.
- **The programs are one tree, shown as a file tree (Q41).** `robots/`
  with a file per deployment, `interrupts/` with `on_fault.py` and
  `on_dying.py`, both fixed, and the player's own files and folders,
  shared by every robot; a fixed panel down the left shows it and a click
  opens a file as a window; the top bar keeps the status alone, and on a
  replay its seek (Q52). A hook
  binds from the robot's file if it defines one, else from the interrupt
  file (`01`).
- **Everything floats over the map, as windows (Q39, amending Q33 and the
  screen as first ruled; Q41 moved the dock into the tree; Q52 added the
  debugger's and the version windows).** One window per open file, one
  each for the inspector, the tools and the log, the debugger's while a
  fork is open, and a read-only **version window** per version of a
  program the player opens to read or debug, dragged,
  resized, collapsed and closed by the player; the tree panel opens each;
  the layout is remembered in `windows.toml` beside the programs.
- **A start screen, an end screen, and the replay always saved (Q36, as
  Q52 amended).** The map, the cards, the programs directory and play,
  host, join or open a replay are picked in the game; the end names the
  winner or the draw, the tick, the hash and the replay's path; the replay
  is written as `replays/<map>-<tick>-<hash>-<team>.ron` inside the
  programs directory whenever a match ends or desyncs, `<team>` the
  saver's; every option is also a command-line flag, which skips the
  screen.
- **The mark tools are a strip, a tool stays armed, a drag marks, and a
  plan previews (Q37, amended by Q44, Q50 and Q51).** Rows of building
  models, paint colors and overlay labels each ending in a clear,
  `B`/`P`/`O` and `1`–`9` to pick, `Esc` to disarm; a drag marks every tile
  it crosses once, and a `Shift`+drag fills the rectangle between its
  corners on release; *clear* withdraws the team's plan of its kind where
  one stands, and plans the slot emptied elsewhere; the player's own plans
  show as a smaller, see-through version of their effect, and the armed
  tool as the same ghost under the cursor or over the rectangle, except on
  a tile already carrying exactly that plan.
- **A stall is a banner with a resign; a desync freezes the match with
  the report and the replay saved (Q38).** The banner names the peer, the
  tick and the wait; everything else keeps working. A desync's banner
  carries the report and the replay's path, and *leave* goes to the end
  screen.
- **A fault reaches the player on every channel (Q34, as Q39 and Q41
  moved the panel into each file's window).** A mark over the machine
  while its fault record is fresh, a summary in each file's window counting
  its faults by line across every deployment the file reaches, the full
  record in the inspector, and a line in the log. The record itself is the sim's (Q20);
  the renderer only shows it.
- **A bot's body is the color its program set, and every team is a color
  the player chooses (Q43, amending Q35 and Q42; amended by Q45, Q47 and
  Q48).** A bot is drawn in its `color` (`02`), or a neutral body color
  while it has none, and its lights — the lens, the marker lights, the
  tail light, the dot on top — glow in its team's color over that paint; a
  printer's and a depot's body is the neutral color with its lights
  glowing in its team's color and a printer's trim painted in it, and a
  site, which has no lights, is outlined in it. The player's own team is
  green by default, each other team's the next of a list in
  `data/interface.toml` in team order, skipping the player's — blue first,
  so a default one-on-one is green against blue. The player may change any
  team's color, their own included, from the inspector; the choice is
  saved in `colors.toml` beside the programs and never reaches a peer.
- **Every bot wears its entity id on top (Q46, amending Q42 and Q43).**
  The id's last three digits, in a seven-segment display set into the top
  face: fixed segments, lit in the team's color in whichever quarter turn
  reads upright from the camera, so it redraws rather than spins; the
  chevron, the trim and the light around it keep the heading, the paint
  and the team readable from above.
- **A program is debugged on a fork (Q52, amending Q15, Q36 and Q39).**
  Line breakpoints are the editor's; a hit opens a private copy of the sim
  at that point, where the player steps by line, over a call, by tick, on
  to the next breakpoint or runs freely, steps back, inspects the selected
  machine's execution and may edit its variables, and a **what-if** — its
  own button, beside deploy — sends the editor's text into the copy to try
  it before deploying it. A breakpoint sits on a line of the text its
  window shows — a file window's, which moves with its line as the text is
  edited, or a read-only version window's, showing what a machine runs —
  and watches whatever runs exactly that text. While the map draws a fork,
  the mark tools rest. The same tool opens on a live match or on a replay
  saved with its team. In a live PvP match a fork never passes the live
  tick and is never diverged — no edits, no what-if. Debugging, below.

## The screen

The map fills the window; two fixed strips frame it and everything else
floats over it (Q39, Q41):

| Window | Holds | Ruled in |
|---|---|---|
| **the time bar** — fixed, across the top | the tick, the speed and its `−`/`+`, the delay, the state hash, the peers, a stall or desync report, the match's end, and on a replay its seek; nothing else | Q6, Q12, `06`, Q41, Q52 |
| **the tree** — fixed, down the left | `robots`, `interrupts`, then the player's folders and files, each marked ahead, error or open; new file and folder, rename, delete on the player's; at its foot the inspector, the tools, the log, and the debugger while a fork is open | Q41, Q52 |
| **one per open file** | the file being edited, the versions it is deployed as, the deploy and export buttons; the faults naming this file | Q33, Q34, Q39, Q41 |
| **inspector** | the hovered tile as the team knows it, the selected machine's record and fault, the teams and their deployments' versions, each team's color to change | Q34, Q43, Q45 |
| **tools** | the mark strip and resign | Q37 |
| **log** | machines' `log` lines, dropped commands, exchange events, the match's end | `02`, `06` |
| **debugger** | while a fork is open: its tick, state and controls, why the last step stopped, the hit list, the selected machine's stack, locals, globals, action, budget and interrupts | Q52 |
| **one per version window** | a version of a deployment's program a machine runs or a replay ran, read-only, apart from the tree; its breakpoints | Q52 |

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

- **The tree** holds `robots/` — `bot.py`, `printer.py`, `depot.py`, one
  per deployment (Q42) — and `interrupts/` with `on_fault.py` and
  `on_dying.py`; both folders, every deployment's file and both interrupt
  files always exist and cannot be renamed, moved or deleted, and the
  player cannot add a file to either folder. Everything outside them is the
  player's: files and folders they add, rename, move and delete, shared by
  every robot. A file's name is `[a-z_][a-z0-9_]*.py` (`01`) and its stem
  is unique among the files outside `robots/`, since folders are
  organisational only and the language has no packages; a collision is a
  load error shown in the tree. A robot file is its bundle's `main.py`,
  never a module, so `lib/printer.py` beside `robots/printer.py` is no
  collision. A file found in `robots/` for no deployment — `robots/1.py`
  from a tree written before Q42 — composes nothing, is marked as an error
  in the tree and its window, and the player may delete it; the match
  starts without it. A file that calls a builtin Q42 removed — `print` or
  `convert` — without defining, importing or assigning that name itself is
  marked the same way at that line, since such a call loads but faults
  every time it runs.
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
  `Deploy`s; a deployment whose file is empty gets none and runs the empty
  program (`06`). The cards (`04`) use the same layout; a script's `bundle
  = "bot"` names the deployment whose bundle its tree composes.
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

- **Body:** a bot is an atlas cube in its `color`, or the neutral body
  color while it is `None`; a printer, a depot and a site are their own
  shapes in the neutral body color (Q43).
- **Team:** a bot's lights glow in its team's color (Q45), and so do a
  printer's — its panel light, side markers and status light, made larger
  (Q48) — and a depot's lamp where its braces cross (Q47); a printer's
  trim — its bands and the rails of its top — is painted in it (Q48); a
  site, which has no lights, is outlined in it (Q43).
- **Number:** a bot's entity id, its last three digits, in fixed seven
  segments on its top face, lit in its team's color in the quarter turn
  that reads upright from the camera (Q46). The player's own team is green unless changed,
  each other team's from the list in data unless changed, so friend and
  foe read at a glance and three teams stay three.
- **Health:** a bar over the machine for a few seconds after any change.
- **Fault:** the mark above.
- **Fog:** as Q21 rules — a machine on a remembered tile is drawn as last
  seen, dimmed; on an unknown tile, not at all.

## Marks

The tools window is a strip (Q37): a row of the building models a plan may
name (`printer` among them, Q42), a row of the eight paint colors, a row of
the overlay labels in data (Q10, Q26, Q27), each row ending in a *clear* —
the plan to empty that slot. `B`, `P` and `O` pick a row and `1`–`9` an
entry; a click arms a tool, `Esc` or a second click disarms it.

With a tool armed, a left click on a tile is a `Mark` of that kind and
value; a left drag marks every tile the cursor crosses, once per drag,
and does not pan the camera. A left press with `Shift` held anchors a
rectangle instead (Q44): the drag moves its other corner, stopping at the
map's edge, the ghost covers every tile in it, and release marks them all
— `Esc` first fills nothing. *Clear* on a tile where the team already
holds a plan of that kind is an `Unmark` of it, and elsewhere the plan to
empty the slot, decided per tile against the plans the sim last reported.
With none armed, a click selects and a drag pans. The renderer submits
and forgets (Q15): what the player sees is the plan the sim reports,
`delay` ticks later.

A tile carrying the player's team's plan shows it as a smaller,
see-through version of its effect (Q50) — a square of the new color inset
from the tile's edge, with any paint already there showing round it; a
smaller overlay marker under its label (Q51); a scaled-down translucent
ghost of the building, a printer for a printer plan; a clear plan hatches
the slot — and the armed tool shows the same ghost under the cursor, or
over a `Shift`+drag's rectangle, on the tiles it can plan and that do not
already carry exactly that plan (Q51): paint and overlays go only on
ground the team has seen (`03`, Q49), so a single mark elsewhere is
refused with the reason and a stroke marks the rest and says how many it
skipped. Only the player's team's plans are drawn, since plans are private
(`03`), and the snapshot carries that team's plan values per tile for it:
a read, in no hash.

## The match

**Before the first tick** (Q36) the start screen lists the maps in
`data/maps` and the cards in `data/opposition` (`04`), takes the programs
directory (Q33), and offers *play* alone against the cards, *host* on an
address, or *join* one (`06`), or *open a replay* from `replays/` (Q52).
Starting reads the programs in and builds the driver as the command line
does; every choice is also a flag, and a flag skips the screen, so a
headless run and CI need none — `--play`, which chooses nothing, starts the
defaults at once, and `--replay <path>` opens a replay.

**A replay** plays back in the match's screen with no peer, and sends no
command: the mark tools, resign and deploy are not offered. Its pace and
pause are the viewer's and its time bar also seeks, as `06`'s replay
driver rules: the `SetSpeed` commands in its log set nothing, a paused
replay stays paused, and a seek restores the latest state the driver has
saved at or before the tick asked for and runs on to it. A replay that
records its team is drawn in that team's view and can be debugged
(Debugging), a breakpoint hit pausing the playback directly; one that
does not is drawn in the first team's view and can only be watched.

**After the last** the end screen names the winner or the draw (`04`), the
ending tick, the final hash and the replay's path; *again* returns to the
start, *quit* leaves. **The replay is always saved** when a match ends or
desyncs: `(map, command log)` and the saver's team (Q52), in
`crates/replay`'s RON, as `replays/<map>-<tick>-<hash>-<team>.ron` inside
the programs directory, named by the tick and hash so two peers that agree
write names that differ only in the team, and two peers sharing one
directory keep both.

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

## Debugging

A program is debugged on a **fork** (Q52): a private copy of the sim, as
[06-architecture](06-architecture.md)'s Forks section defines it, which
never feeds back into the match. Everything here is the renderer's; the
match's sim is only read — its snapshot, its hit reports and its saved
states — and given breakpoint watches, which change nothing.

- **Breakpoints.** A click in a window's gutter sets a breakpoint on that
  line of the text the window shows, scoped to the selected machine if
  its bundle holds the file, and otherwise to every machine of the team
  whose bundle does; on a line where no statement begins (`06`) it moves
  to the next one. It becomes `06`'s watch on that text's content hash, so
  it hits wherever that exact text runs, and is greyed while nothing runs
  it. In a **file window** the text is the tree's, which the player edits:
  editing carries breakpoints with their lines, as it carries the cursor,
  and they hit once the edited text runs — in the match once deployed and
  taken (Q11), in a fork once what-if has sent it. In a **version window**
  the text is a version a machine runs, read-only, so breakpoints there hit
  the code already running even while the tree's file is ahead of it.
  Breakpoints are kept with the tree, never sent to a peer, and only the
  team's machines hit them.
- **Version windows.** A file window offers each version of its file that
  a machine of the team runs, and the inspector the version the selected
  machine runs; each opens a read-only version window, apart from the
  tree, never exported or deployed. On a replay, the versions are those
  the team's own deploys in its log carried, never another team's.
- **Saved states.** While a breakpoint is set or a fork is open, the
  driver saves the match's state at once — at the latest completed tick,
  so a hit or a fork opened by hand always has one to start from — and at
  every later tick that is a multiple of `debug_checkpoint_ticks`, keeping
  the last `debug_checkpoints_kept`; a fork saves its own on the same
  terms. A saved state a fork was opened from, or that a listed hit needs,
  is kept for as long as that fork or that hit is, beyond the count. With
  no usable state a fork re-runs from the start (`06`). A replay's driver
  keeps its own for seeking (`06`, The driver).
- **A hit** opens a fork at the earliest position among the tick's hits
  (`06` orders them): from the latest saved state before the hit's tick,
  run to that position; the tick's other hits are listed as further hits.
  The map draws the fork under a banner naming its tick, the machine, and
  its state — faithful, or each of ahead, stale and diverged that holds
  (`06`) — and the debugger window opens on that machine. In single-player
  and PvE the match is also paused by a `SetSpeed(0)`, which lands `delay`
  ticks later (Q12), unless the last speed the player submitted is
  already `0` — so a resume still in flight is overtaken by the pause; in
  PvP it runs on, and the time bar keeps the live tick. On a replay, which
  sends no command, the playback pauses at once.
- **Further hits** while a fork is open leave it as it is: the debugger
  window lists them by position, the latest first, keeping the last
  `debug_hits_kept`, each with the count `06` reports of the beginnings it
  stands for, and reopens the fork at any listed one. They submit no
  further pause.
- **A fork can also be opened by hand** at the match's latest tick, or at
  any tick of a replay.
- **The map of a fork** is `06`'s snapshot of it at its position, mid-tick
  ones included: machines as they stand there, fog as of the fork's last
  vision pass, in the team's view (Q21). Only the team's machines can be
  selected or inspected.
- **The match while a fork is open** keeps its own controls: resign, the
  speed, and every file window's deploy button act on the match, never
  the fork. The mark tools rest while the map draws a fork, since they act
  on a map the player is not looking at; *show the match* in the banner
  draws the match again, with the tools, and the fork stays open. The
  match's later commands can make the fork stale (`06`).
- **Stepping**: *line*, *over*, *tick*, *continue*, *run* and *back*, as
  `06` defines each, with the keys the build chooses; *run* goes at the
  speed the debugger window sets, with a pause. The whole world advances;
  *line* and *over* need a selected machine. The window says why a step
  stopped: a statement, a watched line and whose, `dying` delivered,
  death, a restart, the end of the tick, the step limit, the live tick, or
  the match ending.
- **Inspection**: the selected machine's call stack, each frame with its
  file, line, function and locals, the frame's line marked in a version
  window of the text the machine runs; the globals of each module it has
  run; its record, action and progress; its tick budget and deficit; its
  pending interrupts; and whether it is in a handler, which, and the hook
  budget spent — all from the fork at the stop, not from a snapshot.
- **Editing a variable**: a local of a frame the stack shows, or a global
  of a module the selected machine has run, typed as `06` allows, on a
  fork paused at a statement, a watched line, a restart or a tick's end —
  not at a death in step 4, where the machine is being removed. The fork
  is marked *diverged* in its banner from then on.
- **What-if**: while a fork is open, paused where an edit could be made,
  each file window offers *try in the fork*, which sends that file's
  deploys into the fork, not the match, for its next tick; the fork is
  *diverged*, and the file's breakpoints now hit in it. Beside it in a
  match is the deploy button, and *take it*, which deploys the same text
  to the match, `delay` ticks after the match's own tick. On a replay
  there is no match: file windows offer *try in the fork* alone.
- **PvP**: in a live PvP match a fork stops at the match's latest completed
  tick and waits there, and never diverges: neither edits nor what-if are
  offered, and deploy acts on the match as always. A replay of a PvP match
  is a replay: its match is over, and every rule above applies.
- **Replays**: a replay that records its team (The match) is debugged as
  that team, its programs in version windows. A replay with no recorded
  team — a match of cards only, or one saved before Q52 — can be watched
  but not debugged.
- **Closing** the fork returns the map and the windows to the match.

## Costs

Every number this doc names is a tuning constant in `data/interface.toml`:
`fault_mark_ticks`, `debug_checkpoint_ticks`, `debug_checkpoints_kept`,
`debug_step_max_ticks` and `debug_hits_kept` (Q52) — the step limit passed
to a fork's steps as an argument, so the sim reads none of them — the
neutral body color, the player's default team
color and the list for the other teams. None is a sim value; the
player's own choices live in `colors.toml` beside the programs.

## What this doc leaves open

Nothing, for now. Dropping a late peer on a timeout, if PvP wants it, is
a `docs/06` rule under a new number, not an interface one (Q38).
