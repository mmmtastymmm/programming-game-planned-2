*Closed record — see [../README.md](../README.md). Not spec.*

# Q58 — How does a hover box take the player to the definition it names?

## Ruling

*Ruling (2026-10-15):* **by a link in the box, or a command-click on the
name; either opens the file's window in front and shows the line.**
Amends Q57, whose box showed where a name is defined but went nowhere.

### The rules

All are the renderer's and reach no hash.

1. **The link.** A box for a name defined in a file of the tree ends in
   `go to <path>:<line>`. A builtin's box has none: its text lives in
   `data/language/docs.toml`, not in the tree.
2. **The box stays reachable.** It sits under the name, or over it near
   the bottom of the screen, and stays open while the pointer is over it
   or between it and the name. A click on the text closes it.
3. **Command-click.** `⌘` on macOS and `Ctrl` elsewhere, on a name
   defined in a file, goes there without the box.
4. **Arriving.** The file's window opens, or if it is open is expanded and
   brought in front; the line scrolls into view, is lit briefly, and takes
   the cursor at its start. Nothing is selected, so a keystroke replaces
   nothing.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided
  section and Programs and the editor.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **A box of the editor's own, under the name** *(chosen)* | Its own rules for opening, closing and the delay. Bought: the link can be reached. |
| egui's tooltip, as Q57 drew it | It follows the pointer and closes the moment the pointer leaves the name, so a link in it can never be clicked. |
| Command-click only | The standard gesture, but invisible: nothing on screen says it exists. The link's hover text names it. |
| Select the line on arrival | The most visible mark, but the next keystroke would replace the line. A fading highlight shows it without the risk. |

### How the answer took its shape

The second of the three editor features the user asked for alongside
Q57: docstrings on hover, a way from the box to the file that defines
the function, and red underlines on incompatible types.
