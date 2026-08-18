*Closed record — see [README.md](README.md). Not spec.*

# Answered questions — Q1–Q5

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
