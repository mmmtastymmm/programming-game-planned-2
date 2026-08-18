*Closed record — see [../README.md](../README.md). Not spec.*

# T6 — Answer Q13, the subset boundary

Ruled: a broad subset — procedural Python plus `class`, `set`, `match` and
`import`; no generators, no reflection.

The first draft chose a narrow procedural subset on a single argument: `class`
instances and generators carry live state across a mid-match hot-swap. That
argument was **rebutted rather than outweighed** — a swap clears all variables,
so nothing survives it in a broken state — and the boundary widened. The
dependency now runs the other way and is recorded in both rulings.

Completed in `dd6c700`.
