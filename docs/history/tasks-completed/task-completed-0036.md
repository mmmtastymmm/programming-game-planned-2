*Closed record — see [../README.md](../README.md). Not spec.*

# T36 — The status bar reads the stored tick hash, not a fresh one

From [I6](../inbox-triaged/inbox-triaged-0006.md). `render`'s
`Driver::state_hash`, which the status bar calls every frame, returns the
hash the driver stored when the tick ran and computes one only before the
first tick or once the stored one has aged out. The driver's own per-tick
hash, which the exchange needs, is unchanged.

Completed in `0665332`, the merge of #101, which did the work and triaged the
inbox entry that asked for it in one PR; this file closes the task.
