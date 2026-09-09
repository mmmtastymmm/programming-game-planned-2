*Part of [01-language](../01-language.md).*

# Syntax and the boundary

What a program is made of, and exactly how much of Python it is. The
[divergence list](#divergences-from-python-in-full) at the end is the part a
player reads; everything above it exists so that the list is complete.

## Decided

- **The language is ours, Python-shaped, deterministic by construction
  (Q5).** A purpose-built interpreter rather than an embedded runtime — not
  because an embedded one failed, but because owning it means determinism is
  designed in rather than audited for, the cost model is a design lever, and no
  dependency can change evaluation order under a checked-in replay hash. It is
  Python's *surface syntax*, not CPython's semantics.
- **The subset is broad (Q13).** Procedural Python — functions, control flow,
  collections, comprehensions, the expression grammar — plus `class`, `set`,
  `match` and `import`. Excluded: generators (implementation cost); dynamic
  reflection (permanently, since it defeats static analysis and makes the module
  graph dynamic); decorators, `with` and `async`/`await` (grammar for use cases a
  machine program does not have); nested `def`, `global` and `nonlocal`, which
  removes closure capture as a question while leaving `lambda` and methods in a
  `class` body; multiple inheritance; and floats, which rule 2 forbids in
  state-affecting paths anyway and which Q14, as amended by Q19, replaces with
  the single number type `num`.
  `try`/`except` was deferred to Q8, which admits it.

  Three additions are only deterministic once we diverge from Python, and these
  are normative: **`set` iterates in insertion order** (and its operators
  preserve that order — without this, `set` is a rule-3 violation wearing
  familiar syntax); **`import` resolves within a closed module set**, which makes
  a program a bundle of named files hashed in sorted name order (determinism rule
  7) with circular imports rejected at load; and **`class` dispatches a closed
  dunder set**, named below, since an open-ended dunder protocol is an
  open-ended determinism surface. `match` needs no divergence: arms are tested
  top to bottom, which is Python's rule already.

  Familiarity is the point of choosing Python at all, so the divergence list,
  not the exclusion list, is what a player has to read. This boundary is sound
  because a hot-swap clears all variables, which Q11 made so; a swap that ever
  resumed instead would reopen it.

## The bundle

A **program** is a bundle of named source files. Every machine of a deployment runs the
same bundle once a redeploy has reached it; until then, the one it had.

- **File names** match `[a-z_][a-z0-9_]*\.py` and are unique within the bundle.
  The entry file is **`main.py`**, which every bundle must contain. The name
  without `.py` is the module name `import` sees.
- **Source is UTF-8 bytes, stored byte-exact** — no whitespace normalisation, no
  line-ending translation, no BOM stripping. A file that is not valid UTF-8 is
  rejected at load.
- **Identity.** Determinism rule 7 (CLAUDE.md) is the canonical statement: the
  version is the hash of every file's name and bytes, each length-prefixed,
  taken in sorted name order. The exact layout is: FNV-1a 64-bit over, for
  each file in bytewise-ascending name order, the name's length as eight
  little-endian bytes, the name's bytes, the content's length as eight
  little-endian bytes, the content's bytes. The length prefixes make the
  encoding injective — no two bundles produce the same stream — which the
  first, bytes-only layout did not (P1). Two bundles that differ in one byte,
  or in one file name, are two versions.
- **Loading is deterministic**, so a bundle that fails to load — a parse error,
  a circular import, a missing `main.py`, a file over the size limit — is
  refused at deploy and never enters the command log. Every peer would refuse
  it identically; one that did not would have desynced already.

## Lexical rules

- **Encoding and identifiers.** Identifiers are ASCII: `[A-Za-z_][A-Za-z0-9_]*`.
  Python's Unicode identifiers are excluded because their NFKC normalisation is
  a versioned table the host would supply. Non-ASCII is legal inside string
  literals and comments only.
- **Indentation** is significant, as in Python: a block is introduced by `:`
  and a newline and is the run of lines indented deeper than the line that
  opened it. Indentation is spaces; a tab character in the indentation of any
  line is a parse error. Within one block every line has the same indentation;
  the amount may differ between blocks.
- **Line continuation.** Inside `()`, `[]` and `{}` newlines are whitespace. A
  backslash at end of line joins the next line. Comments run from `#` to end of
  line.
- **Keywords**, all reserved: `and as break class continue def elif else except
  False finally for from if import in is lambda match case None not or pass
  raise return True try while`, plus Python's `del assert with async await
  yield global nonlocal`, which are reserved so that using one is a parse
  error rather than a call to a missing name. `match` and `case` are keywords
  outright, not Python's soft keywords, so neither is a legal identifier.
- **Literals.** Numbers: decimal only, with `_` separators, a point or an
  exponent allowed — see [numbers](numbers.md#literals). String: `'…'`, `"…"`, triple-quoted,
  with the escapes `\\ \' \" \n \t \r \f \v \0 \xHH \uHHHH \UHHHHHHHH`; an `r`
  prefix disables escapes; an `f` prefix makes an f-string. Adjacent string
  literals concatenate. `b'…'` bytes literals do not exist. `True`, `False`,
  `None`.
- **Operators and delimiters.** `+ - * / // % **`, `| & ^ -` on sets,
  `< > <= >= == !=`, `=` and the augmented forms `+= -= *= /= //= %= **=` and
  `|= &= ^=`,
  and `( ) [ ] { } , : . ;`. The matrix operator `@`, the walrus `:=`, the
  annotation arrow `->` and the bitwise operators `~ << >>` do not exist, and
  `| & ^` act on sets only (Q19). `|` also separates alternatives in a `match`
  pattern.

## The boundary

What exists. Anything Python has that is not in these tables does not exist,
and using it is a parse error at load — never a runtime surprise.

### Statements

| Statement | Notes |
|---|---|
| expression statement | any expression, evaluated for its effect |
| assignment | `x = …`, multiple targets `a = b = …`, tuple/list unpacking with one `*name`, subscript and attribute targets |
| augmented assignment | `+= -= *= /= //= %= **=` and `\|= &= ^=` on sets |
| `if` / `elif` / `else` | |
| `while` / `else` | `else` runs when the loop ends without `break` |
| `for … in …` / `else` | iterates a list, tuple, str, dict (its keys) or set — over a **snapshot** taken at loop entry, so the body may mutate the collection freely; the target may unpack |
| `break`, `continue`, `pass` | |
| `def` | at a module's top level or directly in a `class` body only — inside any compound statement it is a parse error; positional and keyword parameters, defaults, `*args`, `**kwargs`; defaults are evaluated once at definition |
| `return` | |
| `class` | at a module's top level only; single base or none; body holds `def`s, assignments, `pass` and expression statements |
| `match` / `case` | patterns below; guards `if …` |
| `import`, `from … import …` | closed module set, below |
| `try` / `except` / `else` / `finally`, `raise` | exceptions, below |

Not statements: `del`, `assert`, `with`, `async`, `await`, `yield`, `global`,
`nonlocal`, decorators, and type
annotations in any position. Remove from a collection with its methods.

### Expressions

| Expression | Notes |
|---|---|
| literals | num, str, `True`/`False`, `None` |
| names | resolved by the scope rules below |
| arithmetic | `+ - * / // % **`, unary `-` `+`; result types in [numbers](numbers.md) |
| comparison | `< > <= >= == !=`, chained (`0 <= x < w`), `in` / `not in`; `is` / `is not` **only against `None`** — any other right operand is a parse error, because identity of values is an implementation detail |
| boolean | `and`, `or`, `not`, short-circuiting, returning an operand as Python does |
| conditional | `a if c else b` |
| call | positional and keyword arguments, `*seq`, `**map` |
| attribute | `x.name` |
| subscript, slice | `x[i]`, `x[a:b]`, `x[a:b:c]`; negative indices; a slice is a copy |
| list, tuple, dict, set displays | `[…]`, `(…,)`, `{k: v}`, `{…}`; `{}` is an empty dict |
| comprehensions | list, dict, set; nested `for` and `if` clauses; no generator expressions, so `sum(x for …)` is a parse error and `sum([x for …])` is the form |
| f-strings | `f"{expr}"` with `!s`; the alignment specs `:>W`, `:<W`, `:^W` on any value; the numeric specs that [numbers](numbers.md#conversions) owns, and no others; no nested f-strings, no `=` specifier |
| `lambda` | a single expression over its parameters and module-level names; the scope rules below say what it may name |

### Types

`num`, `str`, `bool`, `list`, `tuple`, `dict`, `set`, functions (a `def` or
`lambda`), classes, instances of user-defined classes, and `None`, whose type
has no name — test for it with `is None`. `bool` is a subclass of `num`. The
seven named types, the built-in exception classes and user classes are the
names `isinstance` and a `match` class pattern accept. `float` and `object`
are not names, so naming one is a `NameError`; `int` names the conversion
builtin, which is a function and not a type, so `isinstance(x, int)` is a
`TypeError`.

`isinstance(x, T)` is the **one permitted type query**, `T` a type or a tuple
of types; `type(x)`, `__class__`, `__dict__` and `dir` do not exist.

### Builtins

Every builtin is a determinism obligation; this is the whole list. Costs are
in [execution](execution.md#the-cost-model).

| Builtin | Behavior |
|---|---|
| `len(x)` | of a str, list, tuple, dict, set, or an instance defining `__len__` |
| `range(stop)`, `range(start, stop[, step])` | returns a **`list`** of its values, eagerly — `isinstance(range(3), list)` is `True` and `range(3) == [0, 1, 2]`; a `range` longer than the collection limit faults |
| `min`, `max` | of an iterable or of arguments, compared left to right; `key=`; ties keep the first, so the result is the first in iteration order |
| `sum(iterable[, start])` | of `num` elements |
| `abs(x)` | |
| `sorted(iterable, key=None, reverse=False)` | **stable**, by the one algorithm below; elements must be mutually comparable or the first comparison that fails raises `TypeError` |
| `enumerate(iterable[, start])`, `zip(*iterables)` | return lists, eagerly |
| `any`, `all` | |
| `int`, `num`, `str`, `bool` | conversions, in [numbers](numbers.md#conversions) and below |
| `list`, `dict`, `set` | constructors from an iterable or a mapping, copying. `tuple` names its type for `isinstance` but has no constructor: calling it is a `TypeError`, and a tuple is written as a literal |
| `isinstance(x, T)` | the type query above |
| `round(x[, n])` | [numbers](numbers.md#the-numeric-builtins) |

There is no Python `print` (the game's `print` is a printer's action, `docs/02`), `input`, `open`, `id`, `hash`, `eval`, `exec`, `getattr`,
`setattr`, `hasattr`, `vars`, `globals`, `locals`, `super`, `iter`, `next`,
`map`, `filter`, `reversed`, `repr`, `format`, `chr`, `ord`, `divmod`, `pow`.
Diagnostic output to the player is a game builtin (`docs/02`). What a player
would reach for from that list and miss is tracked in [INBOX.md](../INBOX.md).

### Methods

The closed method set, per type. Anything else is an `AttributeError`.

| Type | Methods |
|---|---|
| `list` | `append`, `extend`, `insert`, `pop([i])`, `remove`, `index`, `count`, `sort(key=None, reverse=False)` (stable, in place), `reverse`, `clear`, `copy` |
| `dict` | `get(k[, default])`, `keys`, `values`, `items` (each returns a list, in insertion order), `pop(k[, default])`, `setdefault`, `update`, `clear`, `copy` |
| `set` | `add`, `remove`, `discard`, `pop` (removes and returns the **first** element in insertion order), `clear`, `copy`; the operators `\| & - ^` and their augmented forms, with the ordering rules below |
| `str` | `split([sep])`, `join`, `strip`, `lstrip`, `rstrip`, `startswith`, `endswith`, `find`, `replace`, `upper`, `lower`, `isdigit`, `isalpha`, `zfill`. **Everything character-class-shaped is ASCII only**, since Unicode tables are versioned: `upper` and `lower` map ASCII letters; `isdigit` is true of `0`–`9` only and `isalpha` of `A`–`Z` and `a`–`z` only; "whitespace" for argument-less `split` and `strip`, and for what `int` and `num` accept around a literal, is exactly space, tab, newline, carriage return, form feed and vertical tab |
| `tuple` | `index`, `count` |

A `str` is a sequence of Unicode scalar values; `len` counts them, indexing and
slicing address them, and no normalisation is ever applied.

### `str` of everything

`str(x)`, and `{x}` in an f-string, is defined for every value, and the text
is one string on every peer:

| Value | `str` |
|---|---|
| `num` | [numbers](numbers.md#conversions) |
| `True`, `False`, `None` | `True`, `False`, `None` — a `bool` is a `num` but prints as Python prints it |
| `str` | itself |
| `list`, `tuple`, `dict`, `set` | Python's bracket forms — `[1, 'a']`, `(1,)`, `{'k': 2}`, `{1, 2}`, `set()` — with elements in iteration order, each written as `str` writes it except that a `str` element is **quoted**: single quotes, and `\\ \' \n \t \r` and `\xHH` / `\uHHHH` / `\UHHHHHHHH` for every scalar outside `0x20`–`0x7E` |
| an instance with `__str__`, exceptions included | what it returns, which must be a `str` or it is a `TypeError` |
| an exception instance without `__str__`, built-in or user-defined | the exception form below |
| any other instance without `__str__` | `<ClassName>` |
| a function or class | `<function name>`, `<class Name>` |
| the exception form | its class name, then `: ` and its arguments, as the exceptions section states |

There is no `repr`; the quoted form above is the one used inside containers
and nowhere else. Walking a nested container is bounded by the nesting depth
([execution](execution.md#limits)).

### The one sort

`sorted`, `list.sort`, `min` and `max` compare with `<` — `__lt__` on
instances — and since that is player code with a price and possible side
effects, the sequence of comparisons is spec:

- `min` and `max` apply `key=`, if given, to every element first, in order.
  Then `best` is the first element and each later element `x` is compared
  once: `min` replaces `best` when `x < best` is true, `max` when
  `best < x` is true. Equal elements never replace, so the first of equals
  wins, and the receiver of `__lt__` is exactly as written.
- `sorted` and `list.sort` are a **top-down stable merge sort**: a run of
  length `n` splits at `n // 2`, each half is sorted first (left, then right),
  and the halves merge by comparing `right < left` and taking `right` only
  when that is true. `reverse=True` sorts the same way but merges by comparing
  `left < right` and taking `right` only when that is true, which gives a
  descending order that keeps equal elements in their original sequence. A
  `key=` function is called once per element, in order, before any
  comparison.
- The first comparison that raises — a `TypeError` between unlike types, or
  anything a `__lt__` raises — propagates from that comparison, with every
  earlier comparison's side effects and cost already spent.

## Names and scope

Python's rules, minus the two statements that were excluded:

- A module's top-level names are its **globals**. Each machine has its own copy
  of every global; there is no state shared between machines.
- A `def` body is a **local** scope. A name assigned anywhere in the body is
  local throughout it (so reading it before assignment is an `UnboundLocalError`
  as in Python). A name only read resolves to a global, then to a builtin.
- **A function cannot rebind a global.** With `global` excluded, `counter += 1`
  inside a function is a local `counter` and faults unbound. A function may
  mutate a global collection or instance. This is on the divergence list.
- A `class` body is its own scope for the names it defines; methods see
  globals, not class-body names, exactly as in Python.
- **A comprehension is evaluated inline in the scope that contains it.** Its
  body reads that scope's names — a function's locals included — except that
  a **target is private to the comprehension**: inside it the target shadows
  any outer name of the same spelling, and it is not an assignment in the
  enclosing function, so it neither makes that name local there nor changes
  it. `def g(x): return [x + 1 for x in range(3)]` is `[1, 2, 3]` and leaves
  the parameter `x` alone. The outermost iterable is evaluated before any
  target exists.
- **A `lambda`'s body sees only its parameters and globals.** Q13 ruled it
  captures nothing, so a `lambda` whose body names a local of an enclosing
  function — a comprehension target included, so `[lambda: i for i in xs]`
  inside a `def` is refused — is a **load error**, not a closure. Its
  parameter defaults are evaluated where the `lambda` is, as in Python, and
  may name anything in scope: `lambda y, k=k: y[k]` is the way to hand a
  local in. This is on the divergence list.
- No name is ever resolved dynamically. `NameError` is a fault.

## Functions

`def` at top level or in a class body. Calls bind arguments as Python does:
positional, then keyword, defaults filling the rest; too many, too few, or a
duplicate name is a `TypeError`. `*args` binds a tuple, `**kwargs` a dict in
call order. Defaults are evaluated once, when the `def` runs, so a mutable
default is shared between calls, as in Python.

A function is a value: it can be stored, passed and returned. Since functions
cannot nest, a function value is always a module-level `def`, a method, or a
`lambda`, and none carries an environment with it.

Recursion is allowed up to the call-depth limit
([execution](execution.md#limits)); exceeding it is a `RecursionError`.

## Collections

- **`list`** and **`tuple`**: ordered sequences. Equality is elementwise. `+`
  concatenates; `*` by an integral `num` repeats. A `list` sorts stably by `<` on its
  elements.
- **`dict`**: insertion-ordered, as Python guarantees. Keys must be `num`,
  `str`, `bool`, `None`, a `tuple` of keys, or an instance (keyed by identity,
  below). Equal values are the same key, so `1` and `True` share one.
  A missing key is a `KeyError`.
- **`set`**: **insertion-ordered**, which Python's is not. Elements follow the
  key rules. The operators are ordered too: `a | b` is `a`'s elements in order
  then `b`'s not already present; `a & b` and `a - b` are `a`'s elements in
  order, filtered; `a ^ b` is `a - b` followed by `b - a`. `pop` takes the
  first. Re-adding a present element does not move it.
- **Comparison** between different types (`1 < "a"`, a list and a tuple) is a
  `TypeError`, as in Python 3.
- **Size.** Every collection and string is bounded by the collection limit;
  growing past it is a `LimitError`.

## Classes

```python
class Scout(Machine):
    speed = 2                      # class attribute

    def __init__(self, target):
        Machine.__init__(self, target) # no super(): name the base
        self.target = target

    def __str__(self):
        return f"Scout->{self.target}"
```

- **Single inheritance.** `class C(B)` or `class C`. Method resolution is `C`
  then `B` then `B`'s base, and so on; there is no `object` to name and no
  `super()`. Call a base method by naming the class.
- **Instances** have attributes set by assignment on `self` or on the instance.
  Reading `x.name` looks in the instance, then its class, then each base in
  turn; a name found nowhere is an `AttributeError`. `C.name` reads a class
  attribute, `C.name = v` sets one, and an instance without its own `name`
  sees the new value. Attribute storage is
  insertion-ordered, which matters only for `match` class patterns and never
  for iteration, since instances are not iterable unless `__getitem__` and
  `__len__` say so.
- **Identity.** An instance — an exception instance included — is keyed in a
  `dict` or `set` by identity, and `==`
  between instances is identity unless `__eq__` is defined. Identity is the
  instance's allocation order within the machine, which is deterministic; it is
  not observable as a number.
- **The closed dunder set.** These special methods dispatch, and only these.
  Any other `__name__` is an ordinary method with an unusual name.

| Dunder | Dispatched by |
|---|---|
| `__init__(self, …)` | construction, `C(…)` |
| `__str__(self)` | `str(x)`, f-strings |
| `__eq__(self, other)` | `==` with the instance on the **left** — there is no reflected form, so `1 == v` is `False` without dispatch and `v == 1` dispatches; `x in xs` compares each element on the left; `!=` is its negation |
| `__lt__(self, other)` | `<`; `>` is `other < self`; `<=` and `>=` are `<` or `==`; `sorted`, `min`, `max`, `list.sort` |
| `__len__(self)` | `len`; truthiness of the instance when defined |
| `__contains__(self, item)` | `in`; an instance without it as the right operand of `in` is a `TypeError` |
| `__getitem__(self, key)`, `__setitem__(self, key, value)` | subscript read and write; `for` iterates `x[0]`, `x[1]`, … up to `len(x)`, evaluated **once** at loop entry, when both `__getitem__` and `__len__` exist |
| `__add__`, `__sub__`, `__mul__`, `__neg__` | `+ - *` with the instance on the **left**, unary `-`; there are no reflected (`__radd__`) or in-place (`__iadd__`) forms, so `2 * v` is a `TypeError` and `v += w` is `v = v + w` |

No `__hash__`, `__iter__`, `__next__`, `__call__`, `__getattr__`,
`__setattr__`, `__repr__`, `__bool__`, `__enter__`, `__exit__`, `__del__`,
`__new__`, `__class_getitem__`, `__match_args__`. An instance is truthy unless
`__len__` is defined and returns zero.

## `match`

Arms are tried top to bottom; the first whose pattern matches and whose guard,
if any, is true runs. No arm matching is not an error. The pattern forms:

| Pattern | Matches |
|---|---|
| literal: `0`, `1.5`, `"s"`, `True`, `None` | by `==`, except that `True` and `False` match only a `bool` of that value and `None` only `None` — so `1` does not match `case True:` |
| capture: `name` | anything, binding `name` |
| wildcard: `_` | anything, binding nothing |
| sequence: `[a, b]`, `(a, *rest)` | a `list` or `tuple` of that shape; `str` is never a sequence here |
| mapping: `{"k": p, **rest}` | a `dict` holding those keys; extra keys are allowed |
| class: `Scout(target=t)` | an instance of that class or a subclass, with **keyword sub-patterns only**, each read as an attribute — a name the instance lacks fails the arm rather than raising; positional class patterns need `__match_args__`, which does not exist |
| or: `p1 \| p2` | either; both must bind the same names |
| as: `p as name` | `p`, also binding the whole to `name` |
| guard: `case p if cond:` | `cond` evaluated after binding |

Bindings made by a failed arm persist, as in Python; a pattern that binds the
same name twice is a parse error.

## `import` and modules

- **The module set is closed:** the bundle's own files, plus the game's
  modules (`docs/02`). `import name` and `from name import a, b` are the forms;
  `import name as alias` and `from name import a as b` are allowed; `from name
  import *`, relative imports, dotted names and packages are not.
- **Resolution.** A name is looked up first among the bundle's files, then
  among the game's modules. A bundle file whose name is also a game module is a
  load error, not a shadowing; a name found in neither is a load error too,
  since both sets are known at load. `from m import x` where `m` has no `x`
  when the import runs is an `AttributeError`, exactly as `m.x` would be.
- **Circular imports are a load error.** The import graph is walked at load;
  a cycle rejects the bundle.
- **A module runs once per machine** — its top-level statements execute the first
  time it is imported during a main-flow run, in the order the imports are
  reached, and a second import returns the same module. Since a restart of main
  flow clears all variables, a restart re-runs every module it imports. Module
  globals are per machine, like all globals.
- A module's attributes are its globals; `mod.name` reads one, and assignment
  to `mod.name` is a `TypeError`.
- Hooks (`on_fault`, `on_dying`) are recognised in `main.py` only.

## Exceptions

`try` / `except` / `else` / `finally` and `raise`, with Python's shape:

```python
try:
    risky()
except ValueError as e:
    handle(e)
except (KeyError, IndexError):
    other()
else:
    only_if_no_exception()
finally:
    always()
```

- `raise X(...)`, `raise X`, and bare `raise` to re-raise inside an `except`.
- The **built-in exception classes**, a closed set with single inheritance:
  `Exception` at the root; under it `ValueError`, `TypeError`, `IndexError`,
  `KeyError`, `AttributeError`, `NameError`, `UnboundLocalError` (a subclass
  of `NameError`), `ZeroDivisionError`, `OverflowError`, `RecursionError`,
  `LimitError`. `except Exception` catches all of them. User classes may derive
  from any of them.
- `LimitError` is what exhausting the collection-size, live-values or
  nesting-depth limit raises ([execution](execution.md#limits)). Budgets do
  not raise; they yield or escalate.
- `e.args` is the tuple of constructor arguments, and `str(e)` is the class
  name followed, if there are arguments, by `: ` and each argument as `str`
  writes it, joined by `, ` — so `ValueError("bad")` prints `ValueError: bad`
  and `raise KeyError(k)` prints `KeyError: ` and the key. `e.line` and
  `e.file` locate the operation that raised it, from the bundle's own bytes,
  and are therefore identical on every peer; `e.tick` is the tick of the raise.
- **An exception the interpreter raises carries no prose.** Its `args` is the
  offending value where there is exactly one — the key for `KeyError`, the
  index for `IndexError`, the name for `NameError`, `UnboundLocalError` and
  `AttributeError` — and is empty otherwise: `ZeroDivisionError`,
  `OverflowError`, `TypeError`, `ValueError`, `RecursionError` and
  `LimitError` have `args == ()`. Two peers therefore never disagree on a
  message, because there is none to word.
- An exception that escapes main flow, or a hook, is a **`fault`**. Escaping
  means **unwinding**, as in Python: on the way out every enclosing `finally`
  runs, and a `finally` that itself raises replaces the exception. `finally`
  blocks do **not** run when an interrupt *abandons* code, because abandonment
  is not unwinding. When the `fault` is delivered, what an interrupt landing
  mid-unwind does, and what the fault record holds are
  [execution](execution.md#delivery)'s rules, not restated here.

## Divergences from Python, in full

A player's first debugging tool is what they already know about Python, so
this is the list of places that knowledge misleads them. It is complete: a
difference not listed here is a defect in this doc.

1. **One number type.** Every number is a `num`, a decimal with a fixed
   number of places and no `int`/`float` split; `/` returns a fraction,
   overflow faults instead of going to infinity, and there are no bitwise
   operators. [numbers](numbers.md) states the places.
2. **`set` iterates in insertion order**, and set operations produce a
   specified order. Python guarantees neither.
3. **No generators**, so no `yield`, no generator expressions, and no lazy
   sequences: `range`, `enumerate`, `zip`, `dict.keys()` and friends produce
   lists eagerly.
4. **Single inheritance only**, and no `super()`: name the base class.
5. **Functions do not nest, and a `lambda` cannot see a function's locals.**
   Methods in a class body are fine; a `def` inside a `def` is not, so there
   are no closures. The scope section has the exact rule for lambdas and
   comprehensions.
6. **A function cannot rebind a global.** There is no `global` statement, so
   `counter += 1` inside a function is a local. Mutate a collection instead.
7. **`import` sees a closed module set**, and a circular import is a load error
   rather than a partially-initialised module.
8. **No reflection.** No `getattr`, `eval`, `type`, `__dict__`, or
   introspection; `isinstance` is the one type query.
9. **`is` compares against `None` only.**
10. **Identifiers are ASCII**, `match` and `case` are hard keywords, and a
    tab in indentation is a parse error.
11. **Strings are ASCII-classed.** `upper`, `lower`, `isdigit`, `isalpha` and
    the notion of whitespace touch ASCII only.
12. **Execution is metered.** A program is paused between operations and
    resumed on a later tick. This has no Python equivalent at all and is the
    one divergence a player must learn rather than merely avoid.
13. **A fault restarts the program.** An uncaught exception runs `on_fault` and
    then main flow starts over from the top of `main.py` with every variable
    cleared. Python's script simply ends.
14. **A program that runs off the end of `main.py` starts again** from the
    top, with every variable cleared. A program is a body the machine runs over
    and over; memory across runs is an explicit `while True:` loop.
15. **Many names are missing.** The statements, builtins, methods and dunders
    above are the whole set; Python has more of each. A missing name is an
    `AttributeError` or `NameError` and a missing statement is a parse error,
    never a silent no-op.
16. **`str` has no `repr`, and interpreter exceptions carry no message.** A
    container prints its `str` elements quoted; `str(e)` of a built-in raise
    is the class name and the offending value, if any.
17. **A module's attributes are read-only**: `mod.name = …` is a `TypeError`.
18. **`:.Nf` floors** rather than rounding, like every arithmetic result;
    `round` is the one operation that rounds to nearest.
19. **`round(x, n)` takes `0 ≤ n ≤ 12` only**; Python's negative places are
    a `ValueError`.
20. **A fractional exponent is a `TypeError`.** `x ** 0.5` does not exist; a
    root is a game builtin.
21. **`tuple` is a name that cannot be called.** `tuple(xs)` is a
    `TypeError`; write a literal.
22. **Instances are dict keys by identity even with `__eq__`.** Python makes
    them unhashable.
23. **`str(e)` of any exception** is the class name and its arguments, joined
    as the exceptions section states; Python prints the arguments alone.
24. **`isinstance(x, int)` is a `TypeError`**, since `int` is a function.
25. **`<=` and `>=` are derived** from `<` and `==`, so an instance defines
    `__lt__` and `__eq__` only.
26. **`for` iterates a snapshot** of a list, dict or set taken at loop entry,
    so the body may mutate the collection; Python raises or misbehaves.
27. **`def` and `class` live only at a module's top level or directly in a
    class body**; inside any compound statement they are a parse error.
