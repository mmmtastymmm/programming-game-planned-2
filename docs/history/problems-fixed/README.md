*Closed record — see [../README.md](../README.md). Not spec.*

# Fixed problems

**One file per fixed problem**, named for its number: `problem-fixed-0007.md`
would hold `P7` and nothing else. The filename is the index — `ls` is the table
of contents, so nothing here restates what this directory holds.

Each file carries the defect as it was recorded and the commit that closed it. A
docs defect closes with the docs edit; an implementation-lag entry closes with
the implementing commit. **The entry is written after that commit lands**, and
cites it — the same order the completed tasks use. Writing the entry first leaves
a file with no hash in it, which `scripts/check-registers.mjs` rejects and the
pre-commit hook blocks.

Moving an entry here is what makes the register's `fixed` count go up — the count
is derived from this directory, so a fix left behind in
[../../PROBLEMS.md](../../PROBLEMS.md) is still open as far as CI is concerned.
