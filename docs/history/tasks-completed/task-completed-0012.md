*Closed record — see [../README.md](../README.md). Not spec.*

# T12 — `match`/`case`

`match` landed in `crates/lang` with every pattern form of
`docs/01-language/syntax.md`'s table — literals, captures, the wildcard,
sequences with one star, mappings with `**rest`, class patterns with keyword
sub-patterns, `|`, `as` and guards — under the rules the table states: arms
top to bottom, bindings of a failed arm persist, no arm matching is not an
error.

Patterns lower to bytecode against hidden slots, with four shape-testing
instructions and one that charges `op.pattern` per node tried. Literal
equality reuses the ordinary comparison, which is how an instance subject
gets its `__eq__` call. The parser refuses each spelling the table excludes
with a message naming it.

Completed in `5f20726`.
