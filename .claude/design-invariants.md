# Design-doc invariants

Properties the design corpus must have. Checked against a single change when one
is proposed, and against the whole corpus periodically. Edit them here rather
than in any skill or checklist that consumes them.

Each invariant was written after something got through — in this corpus or in
the predecessor project it was ported from. `Q<n>`, `P<n>`, `T<n>` and `I<n>`
refer to the four registers described in [CLAUDE.md](../CLAUDE.md): open entries
live in [docs/QUESTIONS.md](../docs/QUESTIONS.md),
[docs/PROBLEMS.md](../docs/PROBLEMS.md), [docs/TASKS.md](../docs/TASKS.md) and
[docs/INBOX.md](../docs/INBOX.md); closed ones live one per file under
`docs/history/`.

Some invariants name a structure the design does not have yet — an economy
graph, a canonical stat sheet, programs printed as shipped source. Each says so
in its opening clause, and starts binding the day that structure exists. They
are kept in that state deliberately: the failure each describes is invisible
until you already have the structure, which is too late to invent the check.

---

## I1 — One canonical statement per fact

Exactly one place defines each fact; everywhere else cites it. Two canonical
statements diverge — not *may* diverge, do.

**Check:** for any fact stated twice, one of the two must be marked a
cross-reference. Watch for the marker being dropped by a later edit.

## I2 — No orphan terms, in either direction

Every term used is defined somewhere; every defined thing has at least one
consumer. Deletions break both directions at once.

**Check:** grep each retired term across `docs/` — hits are needed edits or
deliberate history in `docs/history/`, decided per hit. Then the reverse: for
each thing the docs define, find who consumes it.

## I3 — Decided sections are normative and get a separate pass

Under this repo's conventions the owning doc's *Decided* section is what an
implementer builds from. It must agree with its own doc's body, and with every
other doc's Decided section. General reading passes skim it — it reads as
settled history, so the eye slides past.

**Check:** read every doc's Decided section as its own pass, not as part of
reading the doc.

## I4 — Every derived number reconciles with its inputs

A constant presented as derived must be recomputable from constants fixed
elsewhere. Consistency across documents is not evidence — a wrong number
propagates consistently.

**Check:** name the inputs, cite where each is fixed, do the arithmetic, compare
against the target the doc claims. When a table shares a formula, **recompute
every row** — they all inherit the error.

## I5 — The economy graph is traversable from the starting kit

Once the design has resources, recipes and gates, they form a directed graph
with whatever they unlock. Every obtainable thing must be reachable from what a
player starts with. Unreachability is invisible locally — each node looks
correctly priced.

**Check:** build the graph (tiers → raws → recipes → what each unlocks) and
traverse from the starting kit. Anything unreached is a bootstrap break.
Re-traverse whenever a price, a gate or a tier changes.

## I6 — The stat sheet is closed in both directions

Once one doc declares itself the canonical stat sheet — "if an effect can't name
its row, it isn't a stat effect" — every effect anywhere maps to a row, and every
row has at least one source that grows or modifies it.

**Check:** set-difference both ways — effects (equipment, perks, terrain, state)
against rows, and rows against their sources.

## I7 — Behavioral rules are single-valued

Any rule governing sim behavior must admit one reading. Two competent
implementers reading it must produce identical tick-by-tick behavior — this is a
lockstep-multiplayer game, so ambiguity is a desync, not a wording nit.

**Check:** for each behavioral rule, ask what an implementer does at every branch,
including the exhausted/empty/absent case. Hash-affecting rules carry `⚠HASH` in
`docs/TASKS.md`, and only there — an undecided rule is not yet a rule, so read
`docs/QUESTIONS.md` in full rather than grepping it for a marker it does not
use. Verify CLAUDE.md's determinism rules still hold: no floats in
state-affecting paths, no hash-order iteration, no wall clock, sorted queries
with entity-ID tiebreaks.

## I8 — Shipped programs are source code

Once the docs print programs in the unit language, they are the *actual shipped
source*, not illustrations. They are subject to the language reference, not to
prose review.

**Check:** execute them mentally against the current spec.

## I9 — Citations resolve

A doc citing `Q42`, `T7` or `docs/NN-name.md` must match what is actually there.
Rulings get amended, and citations to the pre-amendment meaning survive the
amendment — under this repo's scheme an amendment is a *new number*, so a
citation to the old one keeps resolving while silently meaning the wrong thing.
That is the case to hunt.

**Check:** resolve citations in changed regions; sample them elsewhere. Pay
attention to a citation whose *summary* of the cited ruling has drifted from what
the ruling now says.
