*Closed record — see [README.md](README.md). Not spec.*

# Answered-question worksheets — Q1–Q13

The full worksheet bodies of answered questions — options weighed, numbers run,
paths not taken — moved here out of [QUESTIONS.md](../QUESTIONS.md) when the
question was answered, so that file holds only what is still open.

This is the **argument**, not the conclusion. The ruling lives in the matching
`questions-answered-NNN.md` shard and is the thing to cite. Open a worksheet
only to recover *why* a call was made — which is rare, and is exactly why these
are the largest files in the corpus and the ones split most aggressively.

Not every answered question has a worksheet; a question decided on sight never
had one.

---

**Q1 — Relationship to the predecessor project**

| Option | What it costs |
|---|---|
| **Fresh take, same core idea** *(chosen)* | Inherits the predecessor's architecture and its habits both. Guards against the habits explicitly or they return. |
| Same genre, different shape | Keeps programmable units and lockstep but changes scale, setting and match structure. More redesign for less carried-over certainty. |
| Deliberately different game | Only determinism carries over. Maximum freedom, and the ported determinism kit becomes the only thing the port bought. |
| Undecided — explore first | Defers the framing decision that every other question hangs off. Cheap now, expensive at the first contradiction. |

The port is already done and green, which asymmetrically favours the first two:
the architecture is not a bet any more, it is a fact.

---

**Q2 — What the player programs**

| Option | Scale knob | Failure mode | What it costs |
|---|---|---|---|
| **A fleet of identical units** *(chosen)* | Fleet size | One bad program, fifty dead units | Requires stable sorted queries under heavy concurrent sensing; makes fault behavior a headline problem |
| One unit, deeply | Puzzle complexity | You, out-thought | Difficulty must come from hand-authored puzzles — a content treadmill rather than an emergent system |
| A squad of specialists | Coordination depth | Deadlock between your own units | Needs inter-program communication in the language core from day one |
| A factory / production system | Throughput | A bottleneck you cannot see | Agents become dumb; the "programming a unit" fantasy largely evaporates |

The deciding argument was where difficulty is *sourced*. A fleet sources it from
interaction between copies, which the sim generates for free once determinism
holds. A single unit sources it from authored content, which someone has to keep
writing.

---

**Q3 — The player's role during a match**

| Option | What it costs |
|---|---|
| **Programming, with mid-match program updates** *(chosen)* | Every update is a lockstep command, so program identity becomes hashed per-tick state and hot-swap semantics become a hash-affecting design problem (Q11). Reaction speed re-enters PvP fairness. |
| Pure programming, no live input | Rejected. Buys a match determined by `(seed, program set)`, a few-kilobyte replay, one code path, and fairness independent of reaction speed — but removes the observe–diagnose–patch loop that is the genre. |
| Live orders to individual units as well | Rejected outright. Breaks Q2's "programs address a role or group, never a unit" and moves the skill being tested from code to clicking. |
| Between rounds only | Keeps programming central without twitch play, but multiplies match-structure decisions (round length, carry-over, scoring) before anything else is settled. |

The axis that actually decided it is **what the player is allowed to touch**, not
*whether* they may act during a match. Updating a program is authorship;
commanding a unit is play. This design permits the first and forbids the second,
which is a sharper line than "live input yes/no" and the reason the pure-
programming option lost without its simplifications being in doubt.

Those simplifications are worth naming, since they are what was paid: the replay
artifact grows from a single value to a timed command log, program versions must
be byte-exact and hashed, and mid-execution swap semantics become load-bearing.
All three are tractable; none is free.

---

**Q4 — The opposition**

| Option | What it costs |
|---|---|
| **PvE first, PvP later** *(chosen)* | Balance work is deferred, not avoided; the PvE tuning that ships first may not survive contact with a ladder. |
| PvE only | Balance becomes a design problem rather than an arms race, but the lockstep architecture is then largely unexercised by its motivating use case. |
| PvP only | Lockstep pays off immediately, but everything must be fair from day one, with no forgiving period while the sim is young. |
| Co-op / sandbox | Competition optional or absent; least pressure on fairness, and least pull toward the determinism discipline already committed to. |

Deferring PvP is free architecturally because lockstep is not retrofittable and
is therefore being built now either way. What it buys is slack on balance during
the period when the sim is changing fastest.

---

**Q5 — What is the language?**

Measured rather than argued, which is unusual for this corpus and was the point.
See [spikes/lang-determinism](../../spikes/lang-determinism/README.md).

| Option | Determinism | What it costs |
|---|---|---|
| **Purpose-built Python-shaped interpreter** *(chosen)* | Ours to design in | Every builtin is a determinism obligation forever; the interpreter is on the hash-critical path; it is work the spike proved avoidable |
| Embed Rhai (`no_float`, `only_i64`, curated packages) | **Measured: bit-identical across 4 fresh processes** | Version pinning plus a fixture to catch a bump; the cost model is not ours to shape; Rhai's syntax rather than Python's |
| Embed Lua 5.4 | **Measured: differs on every process** | Disqualified on core semantics, not configuration: `pairs()` order is per-process seeded and `7 / 2` is `3.5` |
| Compile to WASM (wasmtime/wasmi) | Deterministic by specification | Not spiked. The player does not write WASM, so a source language and compiler are still needed — it moves the work rather than removing it |

The decisive fact is what the spike did *not* find. Rhai passed, so the choice
stopped being "can we avoid writing an interpreter" and became "do we want to own
one". Owning it was chosen for control over the cost model and freedom from a
dependency that can change evaluation order under a checked-in hash — with the
cost, an interpreter permanently on the hash-critical path, accepted rather than
hand-waved.

Throughput did not discriminate: Rhai ran a plausible unit program in 4.4 µs,
putting 200 units at 30 ticks/s near 2.6% of one core. A hand-written
tree-walking interpreter has no reason to be dramatically worse, so Q6 is not
bounded by interpretation cost either way.

What the losing options contributed, and why the spike was worth running even
though its recommendation was declined: three of the four checklist items in the
ruling — float division, build-dependent limits, and a green test that ran
nothing — were discovered by measuring runtimes we are not going to use.

---

**Q13 — How much of Python?**

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

