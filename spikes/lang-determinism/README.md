# Spike: can a stock embeddable runtime be bit-deterministic?

Answers the empirical half of **Q5** ([../../docs/QUESTIONS.md](../../docs/QUESTIONS.md)).
Q5's leaning was that the embeddable-runtime option is the one to *disprove
first*: if a stock runtime survives the determinism rules it dominates, and if it
cannot, learning that early is worth more than any preference.

```sh
cargo run --release --manifest-path spikes/lang-determinism/Cargo.toml
```

Standalone workspace on purpose — the root `cargo test` never builds a scripting
runtime. **This is an experiment, not a dependency of the game.**

## Method

Each candidate runs a battery designed so that it *could* fail: 60 map keys
inserted in neither sorted nor insertion order, eight user-defined functions,
a closure over captured state, recursion, integer division and modulo with
negative operands, a sort with many ties, and string building at size. The
transcript is hashed with FNV-1a, and the parent re-runs the battery in **four
fresh child processes** and compares.

Cross-process is the whole point. A hash-ordered container is stable within one
process and differs between them, so a same-process test cannot see the failure
this spike exists to find.

## Results

| Runtime | Cross-process | Why |
|---|---|---|
| **Rhai** (`no_float`, `only_i64`, curated packages) | **Deterministic** — 4/4 processes agree | `Map` is a `BTreeMap`, so iteration is sorted by construction |
| **Lua 5.4** (`table`+`string` libs only) | **Nondeterministic** — 4/4 processes differ | `pairs()` order is per-process seeded; `/` is always float division |

Lua fails on core semantics, not on configuration. `pairs()` returned
`alpha,oscar,quebec,zeta,...` on one run and a different order on the next, and
`7 / 2` is `3.5` — Lua 5.4 has integers but division always produces a float, so
rule 2 is violated by the language itself. Neither is fixable by choosing which
libraries to open.

Rhai passed with floats **removed from the language entirely**: `1.5` does not
parse under `no_float`, so rule 2 cannot be violated by accident. `timestamp()`
and `rand()` are absent once the engine is built from `new_raw()` plus a hand-
picked package list rather than `Engine::new()`, whose standard set includes a
wall clock.

## Findings that outlived the yes/no

1. **Rhai's limits default differently between debug and release builds.**
   `fib(18)` stack-overflowed under the debug default and would not have under
   release. Two peers on different build profiles would fault at different
   depths — a desync produced by a build flag rather than by code. Every limit
   (`max_call_levels`, `max_operations`, `max_expr_depths`, string/array/map
   sizes) must be pinned explicitly and become part of the spec.
2. **A battery that fails to run looks like a pass.** v2 of this spike errored
   identically in every process and scored green. The harness now refuses to
   score a transcript containing an error. Any determinism check that compares
   hashes without confirming the workload actually ran has this hole.
3. **`print` survives the curated package list** and must be captured via
   `on_print` or removed.
4. **Integer division truncates toward zero and modulo takes the dividend's
   sign** (`-7 / 2 == -3`, `-7 % 3 == -1`). Deterministic, but it is a spec
   detail that has to be written down rather than inherited silently.
5. **Throughput is not the constraint.** Between 4 and 7 µs per unit-tick in
   release across runs. 200 units at 30 ticks/s is 6000 unit-ticks/s, so
   6000 × 4 µs = 24 ms/s and 6000 × 7 µs = 42 ms/s — **2.4% to 4.2% of one
   core**. The spread is machine load rather than measurement error, which is
   why the useful claim is the order of magnitude: interpretation cost is
   nowhere near bounding the tick rate (Q6) at any fleet size contemplated, and
   that survives the whole range.

## Not tested

Stated so a green run is not read as more than it is:

- **Cross-architecture.** Every process here ran on the same arm64 machine.
  x86-vs-ARM agreement is the property lockstep actually needs and this spike
  does not demonstrate it. Cheapest next step: run the same battery in CI on
  `ubuntu-latest` and compare against a checked-in hash.
- **Cross-version.** A Rhai upgrade could change evaluation order or a builtin.
  If Rhai is adopted, its version needs pinning for the same reason the Rust
  toolchain is pinned, and a fixture needs to catch a bump.
- **WASM (wasmtime/wasmi).** Skipped deliberately. WASM is deterministic by
  specification, but the player does not write WASM — a source language and a
  compiler would still be needed, so it moves the work rather than removing it.
  It becomes worth revisiting only if Rhai fails on one of the untested axes.
- **Hot-swap semantics (Q11).** Rhai compiles to an `AST` that can be swapped
  cheaply, which is promising, but what happens to a unit mid-execution is a
  design question this spike did not touch.
