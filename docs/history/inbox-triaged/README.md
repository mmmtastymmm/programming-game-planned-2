*Closed record — see [../README.md](../README.md). Not spec.*

# Triaged inbox entries

**One file per triaged observation**, named for its number:
`inbox-triaged-0007.md` holds I7 and nothing else.

Each file carries the observation as it was written — unedited, because the
wording someone reached for under time pressure is often the useful part — and an
`## Outcome` section saying where it went:

- `- **Question:**` — a `Q<n>` opened, because it was undecided.
- `- **Problem:**` — a `P<n>` opened, because decided text or code is wrong.
- `- **Task:**` — a `T<n>` opened, because it is work.
- `- **Docs:**` — fixed on the spot, with links to the edits.
- `- **Dropped:**` — a misreading, with the reason it was not real.

Every cited number must resolve; `scripts/check-registers.mjs` enforces it.

`Dropped` exists here and nowhere else. An answered question that changes nothing
is a forgotten propagation, but an inbox entry that turns out to be a false alarm
is the register working as intended — the cost of writing one down has to stay
lower than the cost of being sure, or the inbox goes unused and the observations
are simply lost instead.
