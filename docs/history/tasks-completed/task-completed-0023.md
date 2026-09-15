*Closed record — see [../README.md](../README.md). Not spec.*

# T23 — The program tree: composition, the hook files, the tree panel

Q41 built. In `lang`: `on_fault` and `on_dying` bind from `main.py`'s
top-level `def`, else from the bundle file of that name; a hook from an
interrupt file runs in that module's globals, its body first, once per
main-flow run, on the hook's budget. In `sim`: a programs tree composes
one bundle per deployment — `robots/<d>.py` as `main.py`, the two
interrupt files, the player's files by bare name, stems unique, no player
`main.py` — and a card's `bundle = "1"` names a deployment of its tree;
the starter, the Fool and the golden player moved to the layout, and the
golden fixture regenerated since every bundle carries the interrupt
files. In `render`: the editor holds the tree, composes and loads every
bundle on each edit, maps errors and fault records back to their tree
file, and deploys a file to every deployment it reaches; a fixed tree
panel on the left with a collapsible root `/`, `robots` with a `+`,
`interrupts` under a lock, and the player's folders and files, with
selection as the parent for new entries, suggested names, right-click
menus for new file and folder, rename and delete, and a click that opens
a file as a window; the top bar keeps the status alone.

Along the way the Rust check-on-the-checks gained a target directory of
its own, after sharing the repo's twice left a stale crate behind.

Completed in `ae06eec`, with the tree moved to the left and the menus
added in the same PR.
