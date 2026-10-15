*Closed record — see [../README.md](../README.md). Not spec.*

# Q57 — What does resting the pointer on a name in a file window show?

## Ruling

*Ruling (2026-10-12):* **its signature as written, its docstring, and
where it is defined — or nothing, if the name resolves to nothing.**
Nothing before ruled what the editor shows beyond highlighting and the
load error's squiggle.

### The rules

All are the renderer's and reach no hash.

1. **The box.** The signature as the source writes it, annotations
   included (Q56), highlighted as code; the docstring, cleaned as
   `inspect.cleandoc` does; and the file and line that define the name,
   or the kind of builtin.
2. **Resolution follows the language.** A `def` or `class` of the file;
   an imported name, followed to the file that defines it; a module's own
   name, showing its docstring; `module.name`; `self.method` in a class
   and `Class.method`, through single inheritance; and only then a game
   builtin, a language builtin or an exception class. A name the file
   assigns at its top level hides a builtin of the same name. A robot file
   is never a module (`07`, the tree).
3. **No guesses.** An attribute of anything but a plain name, such as
   `me().pos`, and a name that resolves to nothing, show no box.
4. **Builtins are documented in data.** `data/language/docs.toml` holds a
   signature and a description for every game builtin, language builtin
   and exception class, and a test holds it to exactly those, against the
   cost table, the language's list and its exception classes.
5. **Typing does not break it.** A file that stops parsing keeps its last
   outline until it parses again.

## Outcome

- **Docs:** [07-interface.md](../../07-interface.md) — the Decided
  section and Programs and the editor.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| **Resolve as the language does, silent otherwise** *(chosen)* | An attribute of a value (`me().pos`, `trail.pop`) shows nothing until types can say what the value is. Bought: every box shown is right. |
| Match any `def` of that name anywhere in the tree | A method named like another file's function would show the wrong docstring. |
| Builtin text drawn from `docs/02`'s tables | Prose written for the spec, not for a player at the keyboard, and parsing tables out of markdown would break on any edit to them. |

### How the answer took its shape

The first of three editor features the user asked for: docstrings on
hover, a way from the box to the file that defines the function, and red
underlines on incompatible types. Annotations (Q56) came first, so a
signature carries its types. The other two follow under their own
numbers.
