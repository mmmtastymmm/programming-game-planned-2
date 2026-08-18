*Closed record — see [README.md](README.md). Not spec.*

# Answered questions — Q1–Q13

Rulings, oldest first, one entry per answered question. **The heading states the
range this shard actually holds; it is derived and machine-checked, so append
here and let CI tell you when the range or the size has moved.**

Each entry is `**Q<n> — <title>**` followed by the ruling and the reasoning that
survived the argument. The full worksheet — options weighed, numbers run, paths
not taken — lives in the matching `questions-worksheets-NNN.md` shard, because
the ruling is what gets cited and the worksheet almost never does.

An amended ruling gets a **new entry with a new number**, never an edit to the
old one: citations to the pre-amendment meaning outlive the amendment, and the
only way to catch that is for both texts to exist.

---

**Q1 — Relationship to the predecessor project**

*Ruling (2026-08-17):* a **fresh take on the same core idea**. Lockstep
multiplayer, players program their units, deterministic sim in plain Rust. What
is redone is everything around that: the corpus discipline, the scope, and the
command surface.

*Reasoning.* The predecessor's determinism architecture was sound — it is
already ported here and proven green in CI on a machine that is not the author's.
What went wrong there was not architectural. It was corpus discipline (registers
that appended forever until one reached 166 KB, counts maintained by hand that
drifted four times in a day, a split convention that had silently inverted in
seven of 62 part files) and an ever-widening command surface. Keeping the
architecture and re-deciding the game is the split that preserves what worked
without inheriting what did not.

*Consequences.* The determinism rules in CLAUDE.md stand unchanged and are not
reopened by this design pass. `crates/sim`'s placeholder world model gets
replaced rather than extended.

---

**Q2 — What the player programs**

*Ruling (2026-08-17):* a **fleet of identical units**, driven by a small number
of player-written programs. Many units, few programs.

*Reasoning.* The fantasy is emergence from copies: fifty units running three
programs produce behavior the author did not write line by line, and the scale
knob is fleet size rather than puzzle intricacy. The alternatives each move the
difficulty somewhere this design does not want it — into a single program's
sophistication, into coordination between hand-written specialists, or into
routing a system whose agents are dumb.

*Consequences.* Programs are addressed to a **role or group, not to an
individual unit**, so the language needs no unit-identity concept in its core.
Determinism rule 6 (sorted queries, ties broken by entity id) becomes
load-bearing rather than theoretical: fifty units query the same world state in
the same tick, and any unstable ordering is a desync. And the failure mode — one
bad program is fifty dead units — makes fault behavior a headline design problem
rather than a footnote, which is why it is opened as its own question rather than
settled inside the language question.

---

**Q3 — The player's role during a match**

*Ruling (2026-08-17):* the player **writes and updates programs while the match
runs**. A program update is a lockstep-synchronized command. There is no *other*
live input: no direct orders to an individual unit, no clicking a unit and
telling it where to go.

*Reasoning.* Observing a program fail and patching it mid-match is the core loop
of this genre — the game is programming, not program submission. The alternative
considered seriously was pure programming: submit a set, watch it play out. That
buys real simplifications, and they compound (a match determined entirely by
`(seed, program set)`, a replay of a few kilobytes, one code path for single- and
multiplayer, PvP fairness independent of reaction speed). It was rejected anyway,
because it removes the observe–diagnose–patch cycle that is the fantasy. A player
who cannot intervene is watching a submission, not playing.

The restriction that *survives* is the one that matters: updates are to
**programs**, never to individual units. The player never issues an order. That
keeps Q2's "programs address a role or group, never a unit" intact and keeps the
skill being tested squarely on the code.

*Consequences.*

- `Command` is a real ordered per-tick log, not a single opening value. Its
  principal variant is a program deploy.
- A program update must take effect on the **same tick on every peer**, which is
  the standard lockstep scheduling problem: an update is agreed for a future tick
  so every peer holds it before that tick executes. How far ahead, and what
  happens when a peer is late, is Q12.
- Determinism rule 7 becomes load-bearing rather than housekeeping. Programs are
  byte-exact text, versions are identified by hashing source bytes, and which
  version a unit is running is part of the state hash.
- What happens to a unit **mid-execution** when its program changes — restart,
  resume at the same point, keep or discard local state — is hash-affecting and
  unresolved. That is Q11, and it cannot be deferred past the language question.
- A replay is `(seed, timed command log)`. Still small, because deploys are rare
  next to ticks, but no longer a single value.
- PvP fairness re-enters: with unrestricted updates, reaction speed matters
  again. Deferred alongside PvP itself (Q4) and noted in Q12.

---

**Q4 — The opposition**

*Ruling (2026-08-17):* **PvE first, PvP later.**

*Reasoning.* PvE lets tuning be forgiving while the sim is young; PvP demands
fairness from the first ranked match. The ordering costs nothing architecturally,
because lockstep determinism is being built now regardless — it is not
retrofittable, which is the whole reason CLAUDE.md's rules are non-negotiable
rather than aspirational.

*Consequences.* The golden-replay gate earns its keep from day one even with no
PvP, because the same determinism is what makes PvE reproducible and its bugs
reportable. Balance questions may be deferred; determinism questions may not.

---

**Q5 — What is the language?**

*Ruling (2026-08-18):* **a purpose-built interpreter for a Python-shaped
language, written in this project and deterministic by construction.** Not an
embedded third-party runtime, and *not* CPython semantics — a subset chosen for
this game, wearing Python's surface syntax so it reads as familiar on sight.

*Reasoning.* The spike
([spikes/lang-determinism](../../spikes/lang-determinism/README.md)) was aimed at
disproving the embeddable-runtime option and failed to: Rhai is bit-identical
across four fresh processes once floats are removed from the language and the
package set is curated by hand. So this ruling is **not made out of necessity**.
The alternative works, and was rejected anyway.

What owning the interpreter buys:

- Determinism becomes a property we *design in* rather than one we *audit for*.
  Every builtin, every iteration order, every numeric edge case is ours to
  specify rather than to discover in someone else's changelog.
- No dependency whose next release can change evaluation order underneath a
  checked-in replay hash. Rhai would have needed version pinning and a fixture to
  catch a bump, exactly as the Rust toolchain does.
- The cost model becomes a design lever. "One operation per tick" is a rule we
  can state and tune; an embedded runtime does not expose that cleanly, and Q3
  makes the cost model something the player reasons about under time pressure.
- Python's surface is the most familiar syntax available to the audience, which
  matters more here than in most games because editing under fire (Q3) is the
  core loop.

What it costs, stated plainly: every builtin is a determinism obligation forever,
the interpreter sits on the hash-critical path, and the work the spike showed
could be skipped is work we are choosing to do. The golden-replay gate and the
source scan are what keep that choice honest rather than aspirational.

*Consequences.*

- **The spike's findings do not retire with the option they were measuring.**
  They become the checklist for our own implementation:
  - `/` **must not be float division.** Python's `/` returns a float and Lua's
    does too — that alone violated rule 2 in the spike and is what disqualified
    Lua on core semantics. Ours divides integers or does not exist (Q14).
  - **Iteration order is specified, never inherited.** Python dicts are
    insertion-ordered, which is fine; Python *sets* are hash-ordered, which is
    fatal. Every container we ship states its order in the spec.
  - **Every limit is pinned in the spec, not left to a build default.** Rhai's
    recursion limit differed between debug and release builds and would have
    desynced two peers on different profiles — a desync caused by a build flag
    rather than by code.
  - **A test that fails to run must not score green.** The spike's own harness
    scored an error transcript as deterministic across all four processes,
    because an identical error hashes identically. The interpreter's test suite
    needs that guard explicitly.
- The subset boundary is **Q13** and the number model is **Q14**. Neither can be
  deferred past writing `docs/01`, because both change what the parser accepts.
- `crates/` gains a language crate. Its name is not chosen here.
- Cross-architecture agreement remains unproven for *any* candidate, ours
  included. Owning the interpreter does not grant it — it only means the bug
  would be ours to fix. The CI check the spike proposed is still owed.

---

**Q13 — How much of Python?**

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
