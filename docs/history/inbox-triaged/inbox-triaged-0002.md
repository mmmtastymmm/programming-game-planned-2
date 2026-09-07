*Closed record — see [../README.md](../README.md). Not spec.*

# I2 — How the opening program set enters the command log, and what a unit with an empty role slot does

The overview's diagram has an "opening program set" arrow into the command
log and nothing else mentions it: no text says whether the initial bundles
are the command log's first entries (a `redeploy` to every unit on tick 0,
which `docs/01` would handle for free), or match setup outside the log. And
`docs/01` has no state for a unit whose role slot holds no bundle at all —
before the first deploy, or if a role is created empty. Probably Q9's or
Q12's to absorb; noted so it is not lost.

## Outcome

- **Docs:** [question-answered-0009.md](../questions-answered/question-answered-0009.md)
  — Q9's rules 7 and 8 answer both halves: the opening program set is the
  command log's first entries, one deploy per role agreed for tick 0, and a
  unit whose role has no bundle runs the empty program, which is not a state
  but a program with nothing in it. Propagated to
  [00-overview.md](../../00-overview.md)'s Decided section with the rest of
  Q9.
