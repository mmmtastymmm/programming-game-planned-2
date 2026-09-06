*Closed record — see [../README.md](../README.md). Not spec.*

# Q5 — What is the language?

## Ruling

*Ruling (2026-08-18):* **a purpose-built interpreter for a Python-shaped
language, written in this project and deterministic by construction.** Not an
embedded third-party runtime, and *not* CPython semantics — a subset chosen for
this game, wearing Python's surface syntax so it reads as familiar on sight.

*Reasoning.* The spike
([spikes/lang-determinism](../../../spikes/lang-determinism/README.md)) was aimed at
disproving the embeddable-runtime option and failed to: Rhai is bit-identical
across four fresh processes once floats are removed from the language and the
package set is curated by hand. So this ruling is **not made out of necessity**.
The alternative works, and was rejected anyway.

What owning the interpreter buys:

- Determinism becomes a property we *design in* rather than one we *audit for*.
  Every builtin, every iteration order, every numeric edge case is ours to
  specify rather than to discover in someone else's changelog.
- No dependency whose next release can change evaluation order underneath a
  checked-in replay hash. Rhai would have needed version pinning and a fixture to
  catch a bump, exactly as the Rust toolchain does.
- The cost model becomes a design lever. "One operation per tick" is a rule we
  can state and tune; an embedded runtime does not expose that cleanly, and Q3
  makes the cost model something the player reasons about under time pressure.
- Python's surface is the most familiar syntax available to the audience, which
  matters more here than in most games because editing under fire (Q3) is the
  core loop.

What it costs, stated plainly: every builtin is a determinism obligation forever,
the interpreter sits on the hash-critical path, and the work the spike showed
could be skipped is work we are choosing to do. The golden-replay gate and the
source scan are what keep that choice honest rather than aspirational.

*Consequences.*

- **The spike's findings do not retire with the option they were measuring.**
  They become the checklist for our own implementation:
  - `/` **must not be float division.** Python's `/` returns a float and Lua's
    does too — that alone violated rule 2 in the spike and is what disqualified
    Lua on core semantics. Ours divides integers or does not exist (Q14).
  - **Iteration order is specified, never inherited.** Python dicts are
    insertion-ordered, which is fine; Python *sets* are hash-ordered, which is
    fatal. Every container we ship states its order in the spec.
  - **Every limit is pinned in the spec, not left to a build default.** Rhai's
    recursion limit differed between debug and release builds and would have
    desynced two peers on different profiles — a desync caused by a build flag
    rather than by code.
  - **A test that fails to run must not score green.** The spike's own harness
    scored an error transcript as deterministic across all four processes,
    because an identical error hashes identically. The interpreter's test suite
    needs that guard explicitly.
- The subset boundary is **Q13** and the number model is **Q14**. Neither can be
  deferred past writing `docs/01`, because both change what the parser accepts.
- `crates/` gains a language crate. Its name is not chosen here.
- Cross-architecture agreement remains unproven for *any* candidate, ours
  included. Owning the interpreter does not grant it — it only means the bug
  would be ours to fix. The CI check the spike proposed is still owed.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section and the
  reserved-docs table. [CLAUDE.md](../../../CLAUDE.md) — crate layout.
- **Question:** [Q13](question-answered-0013.md) (the subset boundary) and
  [Q14](../../QUESTIONS.md) (the number model). Both change what the parser
  accepts, so neither could be deferred past writing `docs/01`.
- **Task:** [T10, T11, T12, T13, T14](../../TASKS.md) — the language
  implementation, staged across M2. [T8](../../TASKS.md) — answer the language
  questions this ruling left open, then write `docs/01`.
  [T15](../../TASKS.md) — the cross-architecture check the Consequences above
  call still owed; owning the interpreter does not grant that property.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

Measured rather than argued, which is unusual for this corpus and was the point.
See [spikes/lang-determinism](../../../spikes/lang-determinism/README.md).

| Option | Determinism | What it costs |
|---|---|---|
| **Purpose-built Python-shaped interpreter** *(chosen)* | Ours to design in | Every builtin is a determinism obligation forever; the interpreter is on the hash-critical path; it is work the spike proved avoidable |
| Embed Rhai (`no_float`, `only_i64`, curated packages) | **Measured: bit-identical across 4 fresh processes** | Version pinning plus a fixture to catch a bump; the cost model is not ours to shape; Rhai's syntax rather than Python's |
| Embed Lua 5.4 | **Measured: differs on every process** | Disqualified on core semantics, not configuration: `pairs()` order is per-process seeded and `7 / 2` is `3.5` |
| Compile to WASM (wasmtime/wasmi) | Deterministic by specification | Not spiked. The player does not write WASM, so a source language and compiler are still needed — it moves the work rather than removing it |

The decisive fact is what the spike did *not* find. Rhai passed, so the choice
stopped being "can we avoid writing an interpreter" and became "do we want to own
one". Owning it was chosen for control over the cost model and freedom from a
dependency that can change evaluation order under a checked-in hash — with the
cost, an interpreter permanently on the hash-critical path, accepted rather than
hand-waved.

Throughput did not discriminate: Rhai ran a plausible unit program in single-digit
microseconds, putting 200 units at 30 ticks/s at a few percent of one core. The
figure moved by roughly 2× between runs on the same machine, so it is recorded as
an order of magnitude rather than a number — the claim that survives the spread
is that interpretation cost is nowhere near bounding Q6, and a hand-written
tree-walking interpreter has no reason to be dramatically worse.

What the losing options contributed, and why the spike was worth running even
though its recommendation was declined: three of the four checklist items in the
ruling — float division, build-dependent limits, and a green test that ran
nothing — were discovered by measuring runtimes we are not going to use.
