*Closed record — see [../README.md](../README.md). Not spec.*

# I4 — `NoIndirectDrawing` on the camera draws no 3D on this Mac

While chasing the paint-plan flicker (#83, #84), adding Bevy's
`NoIndirectDrawing` to the `Camera3d` left the map black on macOS (Metal):
the panels drew, nothing 3D did. No error was logged. Not investigated
further, since the fix went another way (`crates/render/CLAUDE.md`). Matters
only if someone reaches for that setting later, for instance to make
see-through instance order deterministic.

## Outcome

- **Docs:** [crates/render/CLAUDE.md](../../../crates/render/CLAUDE.md) —
  its list of what does not fix the see-through flicker already named
  `NoIndirectDrawing`; it now says that it fails silently on Metal and so is
  no way to make see-through instance order deterministic either, which is
  where someone reaching for it would look.
