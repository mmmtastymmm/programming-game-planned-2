*Closed record — see [../README.md](../README.md). Not spec.*

# Q59 — What does the editor underline as a type error?

## Ruling

*Ruling (2026-10-15):* **only what the annotations and the builtins'
signatures make certainly wrong, in red, with the reason in the hover
box; it never stops a deploy.** Q56 said an annotation naming no type is
underlined, and nothing else; this rules the rest.

### The rules

All are the renderer's and reach no hash.

1. **What is underlined.** A call with too many or too few arguments, a
   keyword with no parameter or given twice; a game builtin's call with
   the wrong count or any keyword, which the host refuses (`see`, `hear`
   and `me` ignore extras, so those pass); an argument, a return, an
   annotated name's value or a default that its annotation rules out; an
   operator or comparison the language has for none of the types either
   side may hold; and an annotation naming no type.
2. **Only the certain.** A union fits if any member fits, a `num` may be
   an `int`, a `bool` is a `num`, and a value of unknown type fits
   everything. A value's type is known from a literal, an annotation, a
   call to an annotated `def`, class or builtin, an operator on known
   types, or all the assignments to an unannotated name joined — the
   check does not follow the order statements run in. An attribute
   (`me().pos`) is unknown.
3. **Signatures.** A `def`'s from its annotations, read in its own file;
   a builtin's from `data/language/docs.toml`, where a signature the
   language cannot write (`log`'s keyword after `*values`, `range`'s two
   forms) is not checked. A language builtin's argument count is not
   checked, only its annotated types; a game builtin's count is, and a
   test runs the host on each to hold the two together.
4. **Advice only.** It reads the tree, never the match, changes nothing
   a peer sees, and a program it underlines deploys and runs as it would
   have.
5. **Where it points.** Every expression the parser builds records the
   chars it was read from, which the compiler never reads; an underline
   covers exactly the argument, value or operation at fault.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided
  section and Programs and the editor.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Underline only the certain** *(chosen)* | Misses `t.terrain` on a `Tile \| None`, and anything reached through an attribute. Bought: a red line is always a real mistake, so a player learns to trust it. |
| A strict checker, as mypy is | Every `tile(x, y)` result would need an `is None` check before use, and every unannotated name would read as an error waiting to happen — noise in a game whose players are learning. |
| Flow-sensitive inference | Narrowing after `if t is None: return` and per-branch types. Much more machinery, and the certain-only rule already keeps the flow-insensitive join sound. |
| Check language builtins' argument counts | Their `docs.toml` signatures were written to be read, not enforced, and some (`dict(a=1)`) may accept more than they show; a false underline costs more than a missed one. |
| Underline whole lines | No change to the language crate, but on `f(a, b, c)` the player cannot tell which argument is wrong. |

### How the answer took its shape

The last of the three editor features the user asked for alongside Q57
and Q58: docstrings on hover, a way from the box to the definition, and
red underlines on incompatible types. Annotating the starter library
came first, so the underlines have types to read; checking it against
that library found no underline, which a test now holds.
