*Closed record — see [../README.md](../README.md). Not spec.*

# Q41 — Amend Q33, Q39 and the hook binding: the programs are one tree, shown as a file tree, and a hook binds from the robot's file or the interrupt file

## Ruling

*Ruling (2026-09-13):* **a team's programs are one directory tree**, shown
in a file-tree panel on the right: `robots/` with one file per deployment,
`interrupts/` with `on_fault.py` and `on_dying.py`, and whatever files and
folders the player adds, shared by every robot. A deployment's bundle is
composed from the tree; a click on a file opens it as a window; the dock
leaves the top bar, which keeps the status alone. `on_fault` and
`on_dying` bind from the robot's own file when it defines them, else from
the interrupt files. Made under a new number because
[Q33](question-answered-0033.md) held a working copy per deployment,
[Q39](question-answered-0039.md) put the dock in the top bar, and `01`'s
hook rule bound only from `main.py`; all three are closed records.

### The rules

1. **The tree.** The programs directory *is* the tree. `robots/` holds
   `1.py`, `2.py`, … for the numbered deployments the team can write for
   (Q40), and `printer.py` and `depot.py`; `interrupts/` holds
   `on_fault.py` and `on_dying.py`. Those two folders and every file in
   them always exist — the editor creates a missing one empty — and cannot
   be renamed, moved or deleted. Everything else is the player's: files
   and folders added, renamed, moved and deleted in the tree, and shared by
   every robot program.
2. **Composition.** Deployment `d`'s bundle is `robots/d.py` as its
   `main.py`, plus every file under `interrupts/` and every player file,
   each by its bare name. Folders are organisational only: a module's name
   is its file's stem, which must be unique across the tree, since the
   language has no packages or dotted imports (`01`); a collision is a
   load error shown in the tree. A deploy of a file sends one `Deploy` per
   deployment whose bundle it reaches — a robot file its own, a shared or
   interrupt file every deployment's — each agreed `delay` ticks later
   (Q12).
3. **Hooks.** At load, `on_fault` binds to a top-level `def on_fault` in
   `main.py` if there is one, else to one in the bundle file
   `on_fault.py`, else to nothing — the empty handler `01` already rules;
   `on_dying` and `on_dying.py` the same. The interrupt files are otherwise
   ordinary modules. Binding stays at load and inside the bundle hash, so
   nothing about determinism moves. *(Amends `01`'s hook rule.)*
4. **The panel.** A fixed panel on the right shows the tree: `robots`,
   `interrupts`, then the player's folders and files, each file marked
   *ahead*, *error* or *open*; a click opens the file as a floating window
   (Q39's windows stay); the panel's foot opens the inspector, the tools
   and the log. **The top bar keeps the status** — the tick, the speed and
   its buttons, the delay, the hash, the peers, the reports — and nothing
   else.
5. **A window's fault summary** (Q34) counts the records naming *that
   file*, across every deployment the file reaches; a robot file's is its
   deployment's, as before.
6. **The cards use the same layout** (`04`): a script's `bundle = "1"`
   names the deployment whose bundle the card's tree composes.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided
  entries, The screen, Programs and the editor.
  [01-language/execution.md](../../01-language/execution.md) and
  [01-language/syntax.md](../../01-language/syntax.md) — the hook binding
  and the bundle. [00-overview.md](../../00-overview.md) — the Decided
  listing. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T23](../../TASKS.md) — the loaders, the binding, the panel.
  `⚠HASH`.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Question | Option | What it costs |
|---|---|---|
| the interrupt files | **Team-wide, a robot's own `def` overriding** *(chosen)* | One place for the common handler and an escape hatch per robot; one binding rule with two sources. |
| the interrupt files | Team-wide only, a robot's `def` a load error | Simpler binding, and a robot that needs its own last words has nowhere to put them. |
| the interrupt files | Per deployment | Nothing shared; the folder is `robots/` again under another name. |
| the status | Thin strip at the bottom | The same information, and a bar the eye already knows moved for no gain. |
| the status | **The top bar, status only** *(chosen)* | Nothing new to learn; the dock's toggles go where the files are. |
| opening a file | **A floating window** *(chosen)* | What Q39 built; the tree is its index. |
| opening a file | One tabbed editor | Fewer windows, and Q39's reason — two programs side by side over the match — gone again. |

### How the answer took its shape

The reference is an IDE's project tree over a game window. The one thing
it asked of the language — hooks in a file of their own — turned out to
be a second binding source rather than a new mechanism, because hooks
were already bound at load from the bundle.
