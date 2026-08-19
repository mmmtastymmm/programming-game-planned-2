*Closed record — see [../README.md](../README.md). Not spec.*

# Q13 — How much of Python?

## Ruling

*Ruling (2026-08-18):* a **broad procedural-and-object subset**. Functions,
control flow, the built-in collections including `set`, comprehensions,
`class`, `match`, `import`, and Python's expression grammar — with **no
generators and no dynamic reflection**.

### The boundary

| | In the language |
|---|---|
| **Statements** | expression statements; assignment, including augmented (`+=`) and multiple targets; `if`/`elif`/`else`; `while`; `for ... in`; `break`; `continue`; `pass`; `def`; `class`; `match`/`case`; `import`; `return` |
| **Expressions** | int / str / bool / `None` literals; names; calls; attribute access; indexing and slicing; arithmetic, comparison (including chained: `0 <= x < w`) and boolean operators; the conditional expression `a if c else b`; list, dict, set and tuple literals; tuple unpacking; list, dict and set comprehensions; f-strings; `lambda`, restricted to a single expression |
| **Types** | `int` `str` `bool` `None` `list` `dict` `set` `tuple`, plus user-defined classes |
| **Builtins** | `len` `range` `min` `max` `sum` `abs` `sorted` `enumerate` `zip` `any` `all` `int` `str` `bool` `list` `dict` `set` `isinstance` — plus the game's own, which `docs/02` owns |

| | Excluded, and why |
|---|---|
| **Generators, `yield`** | The only remaining exclusion of substance, and the reason is now **implementation cost alone** rather than hot-swap. Lazy evaluation means a suspended frame the metered executor has to model on top of the one it already has. Cheap to revisit under a new number if a use case appears. |
| **`eval`, `exec`, `getattr`, `setattr`, `__import__`, introspection** | Reflection defeats static analysis and makes the module graph dynamic. Excluded permanently rather than pending. |
| **Decorators, `with`, `async`/`await`** | Grammar for use cases a unit program does not have. |
| **Nested `def`, `global`, `nonlocal`** | Removes closure-capture semantics as a question. Methods inside a `class` body are of course allowed — that is class scope, not a closure over locals. `lambda` survives for `sorted(xs, key=...)` and, being a single expression over parameters and globals, captures nothing. |
| **Floats** | Q14 owns this. Rule 2 forbids them in state-affecting paths regardless. |
| **Multiple inheritance** | Single inheritance only. C3 linearization is deterministic, so this is a complexity call rather than a determinism one — and a wrong MRO is a bug class nobody wants inside a lockstep hash. Revisitable. |

**`try`/`except`/`raise` remains deferred to Q8**, not excluded. Whether a program
handles its own faults is Q8's substance. Q13 rules only the *shape*: Python's,
if an error model is needed at all.

### What the additions require us to specify

Three of the four additions are only deterministic once we say something Python
does not:

- **`set` iterates in insertion order.** Python's does not iterate in any
  specified order at all, so this is a divergence, and it extends to the
  operators: `a | b` is the elements of `a` in order followed by those of `b` not
  already present; `a & b` and `a - b` are the elements of `a` in order, filtered;
  `a ^ b` is `a - b` followed by `b - a`. Without this, set support would be a
  rule-3 violation wearing familiar syntax.
- **`import` resolves only within a closed module set** — the player's own files
  plus game-provided modules. No filesystem, no network, no dynamic import. This
  extends rule 7 rather than breaking it: a *program* becomes a bundle of named
  files, and its version is the hash of every file's bytes taken in sorted name
  order. Circular imports are rejected at load time rather than given Python's
  partially-initialized-module semantics, which are deterministic but a poor
  thing to debug mid-match.
- **`class` needs a closed dunder set.** `docs/01` names exactly which special
  methods dispatch; anything outside that set is an ordinary method with an
  unusual name. An open-ended dunder protocol is an open-ended determinism
  surface.

`match` needs no divergence: case arms are tested top to bottom, which is already
Python's rule and already deterministic.

### Divergences from Python, in full

A player's first debugging tool is what they already know about Python, so the
list of places that knowledge misleads them has to stay short:

1. **No floats, and `/` does not do what Python does** (Q14 fixes exactly what).
2. **`set` iterates in insertion order**, and set operations produce a specified
   order. Python guarantees neither.
3. **No generators**, so no lazy sequences; `range` produces its values eagerly.
4. **Single inheritance only.**
5. **Functions do not nest.** Methods in a class body are fine; a `def` inside a
   `def` is not, so there are no closures over locals.
6. **`import` sees a closed module set**, and circular imports are an error
   rather than a partially-initialized module.
7. **No reflection.** No `getattr`, no `eval`, no introspection.
8. **Execution is metered.** A program is interrupted between operations and
   resumed on a later tick. This has no Python equivalent at all and is the one
   divergence a player must learn rather than merely avoid.

Everything else that is *in* behaves as Python does — notably `dict` iterates in
insertion order, which is Python's own guarantee since 3.7 and happens to satisfy
rule 3 for free.

### Reasoning

**The hot-swap argument that shaped the first draft of this ruling was retired,
not worked around.** That draft excluded `class` and generators because both
carry live state across a mid-match program swap: a suspended generator has a
resume point in a function body that no longer exists, and an instance of a
changed class has fields that may no longer be declared. The reply was that a
swap **clears all variables**, and that dissolves the objection completely — if
nothing survives the swap, nothing can survive it in a broken state. The
exclusion had one load-bearing reason and it is gone.

What remains is a straightforward trade of implementation cost against
familiarity, and familiarity is why Python was chosen at all (Q5). A player who
knows Python knows classes, `match`, `import` and sets; meeting them with "not in
this dialect" spends the goodwill that picking Python bought. The subset is now
close enough to Python that the divergence list above, not the exclusion list, is
what a player needs to read.

**Cost, stated plainly.** This is a materially larger interpreter than the first
draft: a class model with method dispatch, structural pattern matching, and a
module system are each substantial on their own. Section *Consequences* records
how that risk is managed without narrowing the target.

### Consequences

- **This ruling now depends on Q11.** It is sound because a hot-swap clears all
  variables. If Q11 later chooses any option that *resumes* — at an offset, at a
  declared re-entry point, or after finishing the current action — then classes
  and any future generators are carrying live state again and this boundary has
  to be reopened. Q11's worksheet records the dependency in both directions.
- **Q8's strictest option got harder, not easier.** The first draft claimed
  excluding reflection made "reject programs that can fault" viable. With
  user-defined classes and duck-typed attribute access, `x.foo()` needs type
  inference to check statically even with no `getattr` anywhere. Reflection's
  absence is necessary for that option and is no longer close to sufficient.
- **The boundary is the target; the build is staged.** `docs/01` specifies all of
  the above, and the first implementation milestone ships the procedural core —
  functions, control flow, collections, comprehensions — with `class`, `match`
  and `import` following. Staging the build does not narrow the spec, and keeping
  the spec whole is what stops the staging from quietly becoming the boundary.
- **`docs/01` must pin what this ruling only names**: the closed dunder set, the
  module resolution order, and the exact `match` pattern forms supported.
- **Widening later is a ruling, not a patch.** Generators and multiple
  inheritance are the two obvious candidates; each gets a new question number so
  the reason is recorded next to the cost.

---|---|
| **Statements** | expression statements; assignment, including augmented (`+=`) and multiple targets; `if`/`elif`/`else`; `while`; `for ... in`; `break`; `continue`; `pass`; `def` at top level; `return` |
| **Expressions** | int / str / bool / `None` literals; names; calls; attribute access; indexing and slicing; arithmetic, comparison (including chained: `0 <= x < w`) and boolean operators; the conditional expression `a if c else b`; list, dict and tuple literals; tuple unpacking; list and dict comprehensions; f-strings; `lambda`, restricted to a single expression |
| **Builtins** | `len` `range` `min` `max` `sum` `abs` `sorted` `enumerate` `zip` `any` `all` `int` `str` `bool` `list` `dict` — plus the game's own, which `docs/02` owns |

| | Excluded, and why |
|---|---|
| **`class`, inheritance, dunder methods** | The largest surface in Python and the one that most complicates hot-swap (see below). Nothing in a unit program needs user-defined types — Q2 made programs address a role, not model an object graph. |
| **Generators, `yield`** | A suspended generator is execution state that must be hashed *and* migrated across a program edit. See below. |
| **`import`, modules** | A program is one file of byte-exact text (rule 7). Modules would make "the program" a graph, and program identity is what Q11 hashes. |
| **Decorators, `with`, `async`/`await`, `match`** | Each is real grammar for a use case a unit program does not have. |
| **Nested `def`, `global`, `nonlocal`** | Removes closure-capture semantics as a question entirely. `lambda` survives because `sorted(xs, key=...)` needs it and a single expression cannot mutate anything. |
| **`set`** | Python's set iterates in hash order, which is rule 3. It could be re-specified as insertion-ordered, but a container that looks like Python's and iterates differently is worse than no container. Excluded until something needs it. |
| **`eval`, `exec`, `getattr`, introspection** | Reflection defeats static analysis, and Q8's strongest option (reject programs that can fault) needs analysis to be possible at all. |
| **Floats** | Q14 owns this. Rule 2 forbids them in state-affecting paths regardless. |

**`try`/`except`/`raise` is deferred to Q8, not excluded.** Whether a program can
handle its own faults *is* Q8's question, and answering it here by accident would
be exactly the "one canonical statement per fact" violation the corpus keeps
catching. What Q13 does rule is the *shape*: if an error model is needed, it will
be Python's `try`/`except`/`raise`, not something invented.

### Divergences from Python, in full

A player's first debugging tool is what they already know about Python, so the
list of places that knowledge misleads them has to fit on one page. It does:

1. **No floats, and `/` does not do what Python does** (Q14 fixes exactly what).
2. **No `set`.** Use a `dict` with `None` values, or a sorted list.
3. **No user-defined types.** No `class`, so no methods and no `self`.
4. **No generators**, so no lazy sequences; `range` produces its values eagerly.
5. **No `import`.** A program is one file, and the game's builtins are already in
   scope.
6. **Functions do not nest**, so there are no closures over local variables.
   `lambda` may only reference parameters and globals.
7. **Execution is metered.** A program is interrupted between operations and
   resumed on a later tick. This has no Python equivalent at all and is the one
   divergence a player must learn rather than merely avoid.

Everything else that is *in* behaves as Python does — notably `dict` iterates in
insertion order, which is Python's own guarantee since 3.7 and happens to satisfy
rule 3 for free.

### Reasoning

Four forces, and they agree:

**Hot-swap (Q11) is the decisive one, and it comes from another question.** Q3
lets a player replace a program mid-match while fifty units are somewhere in the
middle of running it. Every feature that carries *live state across the swap*
turns Q11 from a design question into a research project. A suspended generator
has a resume point inside a function body that no longer exists. An instance of a
class whose definition just changed has fields that may no longer be declared.
Excluding both is what keeps Q11 answerable by choosing between four simple
options rather than by inventing a migration model. That argument is invisible
from inside Q13 and is the reason to draw the line here rather than one notch
wider.

**Editing under fire (Q3) rewards a small, dense language.** The player is
rewriting code while their fleet dies. Comprehensions earn their grammar because
they compress the most common operation — filter a scan result — into one line
that can be typed correctly at speed. Classes cost ceremony at exactly the moment
ceremony is most expensive.

**Every feature is a determinism obligation that never expires (Q5).** We own the
interpreter now, so each construct is ours to specify, test and keep correct
across every future change. The subset above is roughly what a tree-walking
interpreter can carry without a specification anyone dreads reading.

**Familiarity is the point of choosing Python at all.** The subset was chosen by
asking what a competent Python programmer would actually type in a fifty-line
unit program. Comprehensions, f-strings, tuple unpacking, slicing and chained
comparisons all survived that test. Classes, generators, decorators and
context managers did not.

### Consequences

- **Q11 gets easier and must say so.** With no generators and no user-defined
  types, the only live state crossing a swap is local variables of plain types
  and the instruction position. That is what makes "restart", "resume at a
  declared re-entry point" and the rest tractable options.
- **Q8 owns the `try`/`except` slice**, and its "reject programs that can fault"
  option is *viable* precisely because reflection is excluded here.
- **`docs/01` can be written once Q14 lands.** This ruling is its boundary; `01`
  is its specification.
- **The parser still needs Python's hard part.** Significant indentation
  (INDENT/DEDENT), chained comparisons and f-string interpolation are all in
  scope. This subset is smaller than Python, not small.
- **Widening later is a ruling, not a patch.** Anything added to the boundary
  above gets a new question number, so the reason is recorded next to the cost.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section.
  [CLAUDE.md](../../../CLAUDE.md) — crate layout.
  [QUESTIONS.md](../../QUESTIONS.md) — Q8 and Q11 both gained the dependencies
  this ruling created.
- **Task:** [T11, T12, T13](../../TASKS.md) — `class`, `match` and `import`
  each become a staged, ⚠HASH implementation item.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Bare statement subset: functions, `if`/`while`/`for`, lists, dicts, no comprehensions | Smallest and fastest. A Python programmer hits a wall on the first line they would naturally write. |
| Procedural subset: the above plus comprehensions, slicing, tuple unpacking, f-strings, `lambda` | Ordinary Python code compiles unchanged, until it uses a class or a set. |
| **Broad subset: procedural plus `class`, `set`, `match`, `import`** *(chosen)* | A materially larger interpreter — method dispatch, structural pattern matching and a module system are each substantial. Buys a dialect a Python programmer rarely notices they are in. |
| Near-complete Python including generators and reflection | Reflection is excluded permanently: it defeats the static analysis Q8 may need and makes the module graph dynamic. Generators are excluded on cost alone. |

### How the line was drawn, and how it moved

The first draft of this ruling chose the *procedural* option, on one argument:
under Q3 a player swaps a program mid-match while fifty units are partway through
it, and both `class` instances and generators carry live state across that swap.
A suspended generator resumes into a function body that no longer exists; an
instance of a changed class has fields that may no longer be declared. Neither
has a good answer, and inventing a state-migration model to get one is a research
project rather than a design decision.

**That argument was rebutted rather than outweighed.** A hot-swap **clears all
variables**, so nothing survives it — and therefore nothing can survive it in a
broken state. The exclusion had exactly one load-bearing reason, and the reply
removed it. What was left was cost against familiarity, which is a much weaker
case for a narrow line: Python was chosen (Q5) precisely because players know it,
and meeting them with "not in this dialect" spends the goodwill that choice
bought.

Recorded because the shape recurs: the narrow line looked well-argued and was,
right up until an answer to a *different* open question dissolved its premise.
The dependency now runs the other way and is written into both rulings — if Q11
ever chooses an option that resumes rather than clears, this boundary reopens.

### The remaining exclusions, and why they are unequal

**Reflection is permanent.** `getattr`/`eval`/introspection defeat static
analysis and make the import graph dynamic; both matter more now that classes and
modules are in.

**Generators are provisional.** With hot-swap no longer an argument, their
exclusion rests on implementation cost alone — a lazy frame the metered executor
would have to model on top of the one it already has. That is a reason to defer,
not a reason to refuse, and a new question number is cheap.

**Multiple inheritance is a complexity call, not a determinism one.** C3
linearization is perfectly deterministic. It is excluded because a subtly wrong
MRO is a bug class nobody wants to discover inside a lockstep state hash.

### What the additions cost in specification, not just code

Three of the four are only deterministic once we say something Python does not
say: `set` needs a specified iteration order and specified operator output order;
`import` needs a closed module set, a rule for program identity across multiple
files, and a decision on circular imports; `class` needs a closed dunder set,
because an open-ended special-method protocol is an open-ended determinism
surface. `match` alone needed nothing — its arms already test top to bottom.
