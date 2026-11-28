*Closed record — see [../README.md](../README.md). Not spec.*

# I13 — Two places the ground row does not do what `07` says, both on purpose

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

Triaged 2026-11-28. The user took both as they are built: each is what
the ground row was meant to do, and `07`'s two sentences were the ones
to change.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Q65 Decided
  entry says where a plan to raise a tile and one to lower it each draw
  their outline, and Marks says that a stroke asking for the height a
  tile already has withdraws the plan it holds.
