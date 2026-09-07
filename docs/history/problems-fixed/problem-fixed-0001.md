*Closed record — see [../README.md](../README.md). Not spec.*

# P1 — Determinism rule 7's bundle hash is not injective

Rule 7 as written — "the hash of every file's bytes taken in sorted name
order" — and the layout `docs/01-language/syntax.md` first pinned from it
(each file's bytes then a zero byte, names not hashed) let two different
bundles share one version. `{main.py: "x\0y"}` and `{main.py: "x", z.py:
"y"}` hash the same byte stream, and two bundles differing only in the name of
a file nothing imports hash the same. The version is part of the state hash,
so two peers running different programs could agree. The fix length-prefixes
each file's name and its bytes, as `crates/sim/src/hash.rs` already does for
strings, and amends rule 7's wording to say so. Found by review of the
uncommitted `docs/01`, before any code depended on the layout.

Fixed in `89a1d37`: CLAUDE.md's rule 7 now reads "every file's name and
bytes, each length-prefixed", and syntax's bundle section gives the layout.
