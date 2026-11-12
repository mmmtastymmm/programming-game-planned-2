*Closed record — see [../README.md](../README.md). Not spec.*

# T26 — Forks in `sim`: positions, watches, stopping mid-tick, steps, saved states, edits

Q55, ruled in `06`'s Forks section. `sim.save()`, the match's full state at
a tick's end, and a fork — a type of its own wrapping a world, holding
every entry point below, none of which the match's sim has — built from one
or from a re-run, standing at that tick's end stopped as *opened*;
positions and their order, the step-4 damage position included; statements
beginning on their `op.statement` and the clauses that begin again, a
docstring never, a `while` each time its condition is tested, as [T30](task-completed-0030.md) charges it;
a fork recording the match's commands it took per tick, applying them and
then its own log, with its faithful, ahead, stale and diverged flags
(`SetSpeed` never stales it); content hashes; `sim.step` taking a team's
watches and reporting, per watched machine and tick, the first beginning
with its position and a count, read-only and outside the hash; the language
VM stopping at any boundary mid-slice and resuming, and reporting frames,
every run module's globals, the record with action and progress, budget,
deficit, pending interrupts and handler state; the line, over, tick,
continue and run steps, *tick* reporting watched beginnings as `sim.step`
does and stopping at none, *run* paused at a tick's end, a stop at the
tick's end for a machine step 5 removes with its team, *back* not offered
at a first stop, the step limit as an argument, and every stop with its
cause and position, the match's end among them; saving and restoring a
fork's state, which saved states a fork may use, and the re-run fallback,
taking the match's log only before the fork's first tick and its own record
after; edits at slice positions or a tick's end, never of a machine at its
own death, each entry naming its machine and a frame by depth, with live
values counted as an assignment would; what-if deploys in the fork's log; a
fork's snapshot at any position, the team's own machines as they stand and
everything else as at the last completed tick's end; a new branch
discarding later entries and states; what-if deploys validated as a
submission is; the PvP bound and refusal. `06`'s forks-and-watches gate: a
golden replay watched on every line gives its stream unchanged, and a fork
stopped and resumed at every boundary, one restored from each saved state,
and a replay's playback each reproduce it — with mutations in
`scripts/lib/rust-gates.mjs` that make a watch charge budget and a resumed
slice re-charge its deficit, each of which the gate must catch. Tests that
a re-run across an edit applies it to its machine, that a yield before a
watched statement is no hit, that a fork run ahead goes stale when the
match's log gains a `Mark` but not a `SetSpeed`, and that a stale fork's
re-run takes none of the commands that staled it.

Completed in `c031608`, the merge of #114, after `36a76ab`, the merge of
#113. #113 gave the language VM its half: a tracer shown every boundary,
a slice that stops at one and resumes on the budget it opened with, a
copy of a machine, the inspection, and an edit counted as an assignment
would be. #114 built the fork on it: `sim::Fork`, a type of its own that
holds every entry point above, while the match's `Sim` gained only
`save()` and the watches `step` takes and reports on, which are read-only
and outside the hash; `replay` plays a replay on a fork and sends no
command. `crates/sim/tests/forks.rs` takes the rules one at a time, the
four tests named here among them, and thirty defects seeded by hand into
the fork and the VM are each caught by one of them, three only after the
first run missed them and their tests were strengthened.
`crates/replay/tests/forks.rs` is `06`'s gate, and the gates check seeds
the two mutations named here, each caught with the gate's own message
while the golden replay still passes. No replay hash moved. Nothing in
`render` uses any of it yet: that is [T27](../../TASKS.md) and
[T28](../../TASKS.md), and [P5](../../PROBLEMS.md) stays open against
them.
