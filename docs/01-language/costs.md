*Part of [01-language](../01-language.md).*

# Costs

Every operation, builtin and method has a row here, and every row's cost is a
formula over **named constants**. The formulas are spec: they say what a cost
depends on. The constants' values are tuning, and live in
[data/language/costs.toml](../../data/language/costs.toml) — the one place a
number is stated, and the file the language crate loads (T10). Costs are spent
from the tick budget and the hook budget alike
([execution](execution.md#metering)).

A row that is not here does not exist: an operation the interpreter performs
that no row prices is a defect in this doc. Game builtins — sensing, moving,
acting — have their rows in the same file's `[game]` table, which `docs/02`
fills; this part fixes only how such a row is shaped.

## How to read a formula

- `n` is the size of the collection or string an operation traverses: the
  length of its input, or the sum of the lengths where there are several, or
  the length of what it produces where that is the larger. Each row says which.
- Named constants are the keys of the data file, written `op.name`,
  `builtin.name`, `method.type.name` or `factor.name`. A cost is always a
  non-negative integer.
- An operation's **base** cost is charged when it begins and its
  **per-element** part as each element or scalar is handled. The budget is
  checked only at boundaries ([execution](execution.md#metering)): an
  operation, once begun, completes, and may overspend the tick by the rest of
  its cost; the overspend is not carried forward, and nothing ever waits for
  budget it cannot have in one tick.

## Operations

The evaluator charges these as it walks the program.

| Operation | Cost |
|---|---|
| a statement, on execution | `op.statement` |
| a name load | `op.name` |
| a literal | `op.literal` |
| a unary, binary, comparison or boolean operator on numbers, booleans or `None` | `op.operator` |
| `+` on a list, tuple or str; `*` repetition | `op.operator + n × factor.copy`, `n` the elements or scalars produced |
| `==`, `!=`, `<` and the rest on lists, tuples or strs | `op.operator + n × factor.traverse`, `n` the elements compared before the answer is known |
| `in` on a list, tuple or str | `op.operator + n × factor.traverse`, `n` the elements examined |
| `a ** b` | `op.operator` per multiplication or division the procedure in [numbers](numbers.md#exponentiation) performs |
| an attribute read or write | `op.attribute` |
| a subscript read or write | `op.subscript` |
| a slice | `op.subscript + n × factor.copy`, `n` the elements or scalars copied |
| a call, plus each bound argument | `op.call + args × op.argument` |
| a class instantiation | `op.construct`, then its `__init__` as a call |
| a dunder dispatch | as the call it is |
| a list, tuple, dict or set display | `op.display + n × factor.copy`, `n` the elements written |
| one iteration of a comprehension or a `for` | `op.iteration`, plus the body's own operations |
| an f-string | `op.operator + n × factor.char`, `n` the scalars produced |
| a `dict` or `set` membership test or key lookup | `op.subscript` |
| `raise`, and each frame an exception unwinds through | `op.raise` per frame |
| `import`, the first time per run | `op.import`, then the module body's own operations |
| a `match` arm tested | `op.pattern` per pattern node tried |
| an interrupt's prologue or epilogue | nothing — system code |

## Builtins

`n` is the length of the iterable unless a row says otherwise. A builtin that
takes a `key=` function pays a call per element for it.

| Builtin | Cost |
|---|---|
| `len` | `builtin.len` |
| `range` | `builtin.range + n × factor.copy`, `n` the values produced |
| `min`, `max` | `builtin.minmax + n × factor.traverse` |
| `sum` | `builtin.sum + n × factor.traverse` |
| `abs` | `builtin.abs` |
| `sorted` | `builtin.sorted + n × factor.sort` |
| `enumerate` | `builtin.enumerate + n × factor.copy` |
| `zip` | `builtin.zip + n × factor.copy`, `n` the sum of the inputs' lengths |
| `any`, `all` | `builtin.anyall + n × factor.traverse`, `n` the elements examined before short-circuiting |
| `int`, `num`, `bool` | `builtin.convert`; of a `str`, plus `n × factor.char` |
| `str` | `builtin.convert + n × factor.char`, `n` the scalars produced |
| `list`, `dict`, `set` | `builtin.construct + n × factor.copy` |
| `isinstance` | `builtin.isinstance` |
| `round` | `builtin.round` |

## Methods

| Method | Cost |
|---|---|
| `list.append` | `method.list.append` |
| `list.extend` | `method.list.extend + n × factor.copy`, `n` the elements added |
| `list.insert` | `method.list.insert + n × factor.traverse`, `n` the elements after the position |
| `list.pop` | `method.list.pop + n × factor.traverse`, `n` the elements after the position |
| `list.remove`, `list.index`, `list.count` | `method.list.search + n × factor.traverse`, `n` the elements examined |
| `list.sort` | `method.list.sort + n × factor.sort` |
| `list.reverse` | `method.list.reverse + n × factor.traverse` |
| `list.clear` | `method.list.clear` |
| `list.copy` | `method.list.copy + n × factor.copy` |
| `dict.get`, `dict.pop`, `dict.setdefault` | `method.dict.lookup` |
| `dict.keys`, `dict.values`, `dict.items` | `method.dict.view + n × factor.copy` |
| `dict.update` | `method.dict.update + n × factor.copy`, `n` the entries added |
| `dict.clear` | `method.dict.clear` |
| `dict.copy` | `method.dict.copy + n × factor.copy` |
| `set.add`, `set.remove`, `set.discard`, `set.pop` | `method.set.single` |
| `set.clear` | `method.set.clear` |
| `set.copy` | `method.set.copy + n × factor.copy` |
| `set` operators `\| & - ^` and their augmented forms | `op.operator + n × factor.traverse`, `n` the sum of both operands' sizes |
| `str.split` | `method.str.split + n × factor.char`, `n` the input's length |
| `str.join` | `method.str.join + n × factor.char`, `n` the output's length |
| `str.strip`, `str.lstrip`, `str.rstrip` | `method.str.strip + n × factor.char` |
| `str.startswith`, `str.endswith` | `method.str.affix + n × factor.char`, `n` the affix's length |
| `str.find`, `str.replace` | `method.str.scan + n × factor.char`, `n` the input's length |
| `str.upper`, `str.lower`, `str.zfill` | `method.str.map + n × factor.char` |
| `str.isdigit`, `str.isalpha` | `method.str.test + n × factor.char` |
| `tuple.index`, `tuple.count` | `method.list.search + n × factor.traverse` |

## Game builtins

Each game builtin has a row in the data file's `[game]` table, keyed by its
name, and its formula has the same shape as a builtin's: a base cost plus, for
a query that returns a collection, `n × factor.traverse` over the sorted result
it returns (determinism rule 6). `docs/02` names the builtins and states each
row's `n`; nothing here prices a game builtin by name.

## What the constants buy

A cost model that is data can be tuned without touching the spec, and every
tuning is hash-affecting in the ordinary way — a fixture regenerates and the PR
says why (CLAUDE.md). The shape is what does not move: a traversal is linear
in `n`, a sort is `n × factor.sort`, a string operation is linear in its
scalars, and prologues and epilogues are free. Those are the properties a
player can reason about under time pressure; the numbers are what the game
adjusts to make the reasoning matter.
