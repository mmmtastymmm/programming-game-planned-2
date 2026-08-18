*Closed record — see [../README.md](../README.md). Not spec.*

# Fixed problems

**One file per fixed problem**, named for its number:
`problem-fixed-0007.md` holds P7 and nothing else.

Each file carries the defect as it was recorded and the commit that closed it. A
docs defect closes with the docs edit; an implementation-lag entry closes with
the implementing commit. Because the fix and its history entry land together, the
hash goes in a small follow-up commit.

Moving an entry here is what makes the register's `fixed` count go up — the count
is derived from this directory, so a fix left behind in
[../../PROBLEMS.md](../../PROBLEMS.md) is still open as far as CI is concerned.
