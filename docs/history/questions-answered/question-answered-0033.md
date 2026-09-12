*Closed record — see [../README.md](../README.md). Not spec.*

# Q33 — Where does the player write programs during a match?

## Ruling

*Ruling (2026-09-12):* **in the game.** The renderer holds an editor: a
working copy of every deployment's bundle, edited in a panel beside the
map, and deployed from there as a `Deploy` carrying the working copy's
files. The programs directory is where the opening bundles are read from
and where the player exports to on request; the game never watches it.

### The rules

1. **One working copy per deployment**, the renderer's, initialised from
   the programs directory at start and edited in place.
2. **Deploy sends the working copy** as one `Deploy` command's bundle,
   validated at submission like any command (`06`); a bundle that fails to
   load is refused and shown at its file and line.
3. **The running version is the sim's** and is shown against the working
   copy; nothing the editor does reaches a machine except through the log.
4. **Export is explicit** and writes every working copy to the programs
   directory; import is the start of a match.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — written for this
  and Q34 and Q35, with its Decided section. [00-overview.md](../../00-overview.md)
  — the docs table and the Decided listing. [QUESTIONS.md](../../QUESTIONS.md)
  — the status block.
- **Task:** [T17](../../TASKS.md) — the editor.
- **Question:** [Q36](../../QUESTIONS.md) — the match's screens, which
  the editor's start (the import) touches.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **An in-game editor** *(chosen)* | An editor to build and maintain — the predecessor's ran to 3,500 lines with highlighting and completion. Bought: the code and the match on one screen, which is the game's premise (Q3), and a deploy that is one click from the line just changed. |
| An external editor and a watched directory | Smallest build, and any editor the player already knows. Lost: the player looks away from the match to edit, and a deploy on save turns every intermediate save into a lockstep command every peer applies; a deploy on a key needs the game window focused, which is the editor's window anyway. |
| Both from the start | Two sources of truth for one bundle, reconciled on every save in either; the day they disagree the player cannot tell which version a bot runs. |

### How the answer took its shape

`docs/05` says programs "live in their editor and not in the game", written
when the editor was assumed external; the ruling keeps its sense — the
programs directory is the player's store and survives a match — while
putting the editing on the match's screen. The predecessor's editor is the
reference for what the build needs, not a ruling.
