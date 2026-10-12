*Closed record — see [../README.md](../README.md). Not spec.*

# Q56 — Amend Q13: should the language have type annotations?

## Ruling

*Ruling (2026-10-12):* **yes: Python's annotations, in Python's three
places, from a closed set of type expressions, and they do nothing when
the program runs.** Made under a new number because the ruling it amends,
[Q13](question-answered-0013.md), is a closed record: its boundary left
type annotations out of the language in every position, and `->` a
refused token.

### The rules

1. **Where.** A `def`'s parameters (`*args` and `**kwargs` included), its
   return as `-> T`, and a variable as `target: T` or `target: T = value`,
   with one name, attribute or subscript as the target. A `lambda`'s
   parameters take none, as in Python.
2. **They do nothing at run time, and cost nothing.** An annotation is
   never evaluated and never checked against a value. A bare annotation
   compiles to no operation, no statement and no boundary; an annotated
   assignment charges what the assignment does. A name annotated in a
   `def` is local there, with or without a value, as in Python.
3. **The type expressions.** `num`, `int` (an integral `num`, in an
   annotation only), `str`, `bool`, `None`; `list[T]`, `set[T]`,
   `dict[K, V]`, `tuple[A, B, …]` and `tuple[T, ...]`; `A | B`; a class
   or `module.Class`; the game's records `Machine`, `Tile` and `Sound`;
   and any of these written in a string. Anything else is a parse error.
4. **Names are not looked up.** An annotation naming a type that does not
   exist loads and runs; the editor underlines it. No `typing` module.

The tokens `->` and `...` exist for annotations only. No program that
loaded before can contain one, so no replay hash changes.

## Outcome

- **Docs:** [01-language/syntax.md](../../01-language/syntax.md) — the
  Decided section, the lexical rules, the statements table, the new
  Annotations section and divergence 28.
  [01-language/costs.md](../../01-language/costs.md) — the operations
  table. [06-architecture.md](../../06-architecture.md) — Forks, what
  begins no statement. [QUESTIONS.md](../../QUESTIONS.md) — the status
  block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Free, never evaluated** *(chosen)* | A wrong annotation is never caught when a bot runs; only the editor says so. Bought: no budget spent on types, no new runtime failure, and no rule to specify for every type form. |
| Checked on every call, raising `TypeError` | Every annotated call costs budget, and each type form needs a runtime rule — a new determinism surface. |
| Functions only | `x: num = 0` stays a parse error, which a Python programmer will type. |
| Plain names only | The editor cannot see inside a list or through an optional value. |
| An unknown type name is a load error | A typo in a type refuses a deploy that fixes something else mid-match. |

### How the answer took its shape

Asked for as the first step toward an editor that shows docstrings on
hover, jumps to a function's file, and underlines incompatible types. The
language had no annotations, so the editor could prove only what literal
values show; annotations let a player state a type where inference cannot
reach. The four choices were put to the user, who took each
recommendation. Building it found that a lambda's parameters sat inline in
every expression node: the annotation fields grew them until 32 nested
brackets overflowed a test thread's stack, so a lambda's parameters are
now boxed.
