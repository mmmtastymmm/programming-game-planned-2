*Closed record — see [../README.md](../README.md). Not spec.*

# T11 — `class` with single inheritance and a closed dunder set

Classes landed in `crates/lang` as `docs/01-language/syntax.md`'s Classes
section specifies: one base or none, resolution up the chain, attributes on
the instance then the class, identity as a key even with `__eq__`, and user
exception classes deriving from the built-in set. The closed dunder set
dispatches and nothing else.

The piece that shaped the crate is `crates/lang/src/dispatch.rs`: a dunder
dispatch is a call, and the tick budget may pause a machine between any two
operations of it, so every operation that can reach user code — `==`, `<`,
`in`, `str`, `len`, truth, the merge sort's comparisons, `key=`, `any`/`all`,
construction, `log`'s `str` of each value — became a resumable state machine
parked on its frame rather than a synchronous helper. That also let `key=`
be a `def`, which T10 had left open.

Completed in `54d444f`.
