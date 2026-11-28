# Inbox

Raw observations, in whatever words came out. **This file is never spec and holds
no rulings.**

The point of an inbox is that writing something down must be cheaper than
deciding where it goes. Dump it here; triage it later. An entry left here is a
review finding, not a backlog item — a doc pass that walks past this file has
missed something.

**This file holds untriaged entries only.** Triaging one moves it to
[history/inbox-triaged/](history/inbox-triaged/README.md) as its own file,
declaring where it went. Numbering is stable — **append new entries, never
renumber**, and never reuse a number. Entries are written as
`**I<n> — <short title>**` on their own line, which is how
`scripts/check-registers.mjs` finds them.

Triage means turning an entry into at least one of the outcomes below — two is
common, since an observation that opens a task often gets a doc fix at the same
time. The triaged file's `## Outcome` declares which, in these spellings and no
others:

- `- **Question:**` — a numbered question in [QUESTIONS.md](QUESTIONS.md),
  something undecided;
- `- **Problem:**` — a numbered entry in [PROBLEMS.md](PROBLEMS.md), decided
  text that is wrong;
- `- **Task:**` — a numbered task in [TASKS.md](TASKS.md), work to do;
- `- **Docs:**` — a documentation fix made on the spot, with the edits linked;
- `- **Dropped:**` — nothing, because it turned out to be a misreading.

That last outcome is the one this register has and the others do not, and it is
deliberate: an inbox that cannot absorb a false alarm stops being cheap to write
to, and then people stop writing to it. A dropped entry still gets a file and
still has to say why.

## Open

**I13 — Two places the ground row does not do what `07` says, both on purpose**

Building `07`'s ground row (T46) turned up two sentences of Marks that
cannot be followed to the letter. *A ground plan shows as the tile's
outline at the planned height*: for a plan to lower a tile that height
is under the ground, where no outline can be seen, so the renderer
draws the outline on the tile and hatches the ground to be taken away,
as a clear plan's slot is hatched; a plan to raise does float its
outline at the planned height, on a see-through block of the earth to
add. And *none is sent for a tile already known to be ground at that
height*: where such a tile holds a plan for some other height, sending
nothing leaves that plan standing, so a second stroke of *raise* over a
tile planned a level down, or *level* over one, could never bring it
back; the renderer sends the `Unmark` there. Both read as what Q65
meant, and neither is what `07` says.
