*Closed record — see [../README.md](../README.md). Not spec.*

# T17 — The editor, the fault channels and the deployment colors in `render`

`docs/07`'s three rulings, none touching a hash. The editor (Q33): a
working copy per deployment, seeded from the programs directory and empty
for the rest of the team's deployments; tabs per deployment and per file;
highlighting off the lexer's keyword table; the copy loaded the sim's way on
every edit so a load error shows at its line; the running version against
the working copy; deploy of one or every changed copy; export to the
directory the game never watches. The fault channels (Q34): a scribble over
the machine for `fault_mark_ticks`, the per-deployment summary by file and
line, the inspector's record with the version it ran, a log line per record.
The colors (Q35): a bot's atlas is its deployment's, every machine on a ring
in its team's color, the player's white, buildings the white base tinted by
the team's. `data/interface.toml` is read and validated at start.

The layout it shipped with — fixed panels around the map — was amended the
same day by Q39 to floating windows; that is T18.

Completed in `1348ff3`.
