*Closed record — see [../README.md](../README.md). Not spec.*

# Q52 — Should a docstring cost budget?

## Ruling

*Ruling (2026-10-11):* **no: a docstring is dropped at load and costs
nothing.** Until now the language had no docstring rule, so a string at the
top of a function compiled as an expression statement and was charged a
statement and a literal every time the function ran.

### The rules

1. **What a docstring is.** The first statement of a module, a `def` or a
   `class` body, when it is a string literal on its own — adjacent literals
   that concatenate into one included. An f-string is not one, as in
   Python, and nor is a string statement in any other position.
2. **What it costs.** Nothing: it compiles to nothing, is never evaluated
   and is no boundary. A body holding only a docstring runs as an empty one.
3. **No `__doc__`.** The text is not kept; the language has no reflection
   to read it back with (`01`, the divergence list).

This is hash-affecting, as every language rule is: a program with a
docstring now spends less per pass. No checked-in golden replay or language
fixture holds a docstring, so none of their hashes moves.

## Outcome

- **Docs:** [01-language/syntax.md](../../01-language/syntax.md) — the
  Decided section, the statement table, Functions and divergence 8.
  [01-language/costs.md](../../01-language/costs.md) — the operations
  table. [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Free, dropped at load** *(chosen)* | The text is gone after load, so nothing in a program can read it; bought: documenting code never costs budget, as in Python, where a docstring is not run per call. |
| Charged as a statement (as before) | A well-documented helper called in a tight loop pays for its comments; players learn to strip docstrings, the opposite of what a starter library should teach. |
| Free, and kept as `__doc__` | A new attribute and a reflection surface the language otherwise refuses. |

### How the answer took its shape

Raised while planning a starter library with good docstrings: the
starter bot's `dist2`, which `nearest` calls once per point, was paying for
its docstring on every call.
