*Closed record — see [../README.md](../README.md). Not spec.*

# T2 — Doc checks

`check-links.mjs`, `check-registers.mjs`, `check-doc-layout.mjs` and
`check-mermaid.mjs`, all wired into `scripts/ci.sh`.

Each was negative-tested against a deliberately broken corpus copy rather than
trusted on a green run — an empty corpus makes a passing check nearly vacuous.

Completed in `feaa794`.
