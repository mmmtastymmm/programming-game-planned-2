*Closed record — see [../README.md](../README.md). Not spec.*

# T39 — `low_power` in `lang`: the fifth interrupt kind, which returns to main flow, and a program held for power

Q62 as Q63 amended it, ruled in `01`'s execution part. The kind
`low_power`, ranked between `fault` and `redeploy`, raised by the host;
one pending entry, coalesced; not delivered inside main flow's wait on an
action, where a `redeploy` beside it still is; a `fault` or a higher
handler running makes it wait, and it makes a `redeploy` wait. Its
delivery sets main flow aside as it stands; its hook `on_low_power()` —
bound at load from `main.py` or the bundle file `on_low_power.py`, arity
zero, any other a load error — runs under `budget.hook.low_power` in
`data/language/limits.toml`; its epilogue puts main flow back, nothing
cleared and no fault record written; and a missing hook leaves the
machine undisturbed. Escalation from the hook, by an escaping exception
or its budget, to `fault`, a spent budget as a `LimitError`, with the
main flow set aside abandoned; a higher kind preempting it the same. The
wait before a call: the host may answer that it is not ready for a game
builtin, which leaves main flow, or `on_low_power`, at the boundary
before the call, nothing charged, interrupts delivered there, to be asked
again when a later slice reaches it — never in `on_fault` or `on_dying`;
a `low_power` that finds its own hook held starts the hook over, and one
that finds it running merges; and the host may hold a machine's main
flow for a whole slice, as a dark building's is, its budget still
opened. The main flow set aside and a held call in the state hash and in
a copy. Language fixtures for each rule. Nothing raises the kind or
refuses a call until T31, so the golden fixtures hold; the PR says so if
they do not. With T31 and T32, fixes [P8](../../PROBLEMS.md).

Completed in `a78bade`, the merge of #122; the task's own commit is
`0e73249`. The interpreter has the fifth kind, `low_power`, between
`redeploy` and `fault`, with its hook, its budget and its escalation to
`fault`; a host may refuse a game builtin's call in main flow or in
`on_low_power`, which holds the program before the call with nothing
charged; and a host may hold main flow for a whole slice. The task was
built twice. Its first form restarted main flow, as Q62 ruled; the user
then asked that the interrupt return to where the program was, Q63 ruled
it, and the task was rewritten for that before it was committed: the hook
runs with main flow set aside and its epilogue puts main flow back. Two
things the rulings left unsaid were settled in the building and written
into `01`'s execution part: a hook budget that runs out reaches
`on_fault` as a `LimitError`, since that hook takes an exception; and the
state hash writes a kind by a fixed code, not its rank, and writes a held
call or a main flow set aside only when there is one, so no golden
fixture moved. Building it turned up
[I11](../../INBOX.md), a deficit above one tick's budget forgiven after a
tick, which was logged and left alone. With T31 and T32 it fixes
[P8](../../PROBLEMS.md), which stays open against T32.
