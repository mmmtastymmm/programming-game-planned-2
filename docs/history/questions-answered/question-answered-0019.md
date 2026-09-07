*Closed record — see [../README.md](../README.md). Not spec.*

# Q19 — Amend Q14: one number type, `num`, an i128 scaled by 10¹²

## Ruling

*Ruling (2026-09-06):* **one numeric type and no other.** `num` is a 128-bit
signed integer count of **trillionths** — twelve decimal places, an integer
part up to about 1.7 × 10²⁶, no infinity, no NaN, no negative zero. `2` and
`2.0` are the same value; `7 / 2` is `3.5`; `1 / 3` is `0.333333333333`. There
is no `int` type, no `fixed`, no `float`; `int(x)` survives as the truncating
conversion a Python programmer expects, returning a `num`.

This amends [Q14](question-answered-0014.md), which chose two types, `int` and
`fixed`, and is made under a new number because Q14's file is a closed record.
Everything in Q14 not named here stands: `/` returns a fraction, `//` and `%`
floor with Python's semantics, every unrepresentable result rounds toward
negative infinity, overflow and division by zero fault, a literal the type
cannot represent is a parse error, `round` is half to even, and the scale is
spec rather than tuning.

### The rules

Replacing Q14's, where they differ. Each is hash-affecting.

1. **One type, so no promotion.** Every arithmetic operator takes two `num`
   and returns a `num`. Comparison and equality are by value. There is one
   column in the operator table, not two.
2. **Integral contexts require integral values.** An index, a slice bound, a
   `range` argument, a `str` repetition count, a `zfill` width, and a `round`
   place count must have no fractional part; a value with one is a
   `TypeError`. `len` and `index` return integral values.
3. **No bitwise operators.** `& | ^ ~ << >>` are not operators on numbers; on
   sets, `| & - ^` remain. Q13's expression grammar never listed bitwise
   operators, and a decimal type has no bits to expose.
4. **Printing is the shortest exact decimal.** `str(2)` is `"2"`, `str(1.5)`
   is `"1.5"`, `str(1 / 3)` is `"0.333333333333"`. An integral value prints
   with no point.
5. **`isinstance(x, num)`** is the type query; `bool` is a subclass of `num`.
6. **The scale is 10¹²**, stated once in `docs/01`, spec not tuning, and a
   new question number to change — as Q14 said of 10⁶.
7. **The representation is spec.** i128, because the product of two values
   must be computed exactly before it is scaled back, which needs 256 bits of
   intermediate — a small hand-written routine in the language crate, and one
   that must be bit-identical on every target. The determinism scan already
   bans the alternative.

### Consequences

- **The divergence list shrinks again.** Q14's "no floats; `1.5` is a `fixed`
  with six places" becomes "every number is a `num` with twelve places";
  nothing about mixed types remains for a player to learn.
- **Overflow and precision leave the player's experience.** At 10²⁶ and
  10⁻¹² neither is reachable by a unit program doing game arithmetic. Both
  are still faults and still spec, because a language that says "cannot
  happen" is a language with undefined behavior.
- **T10 gains a wide-multiply routine** and the determinism suite (T14) a
  fixture for it: the boundary cases of a 256-bit intermediate are exactly
  where two implementations would disagree.
- **`docs/01` changes in place**, since it is uncommitted, and carries this
  ruling beside Q14 in its Decided section.

## Outcome

- **Docs:** [01-language/numbers.md](../../01-language/numbers.md) — rewritten
  around one type. [01-language/syntax.md](../../01-language/syntax.md) — the
  literal grammar, the operator and type tables, the divergence list.
  [01-language/costs.md](../../01-language/costs.md) — the conversion row.
  [01-language.md](../../01-language.md) — the parts table and invariant 4.
  [CLAUDE.md](../../../CLAUDE.md) — the tuning-constant exception.
  [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T8](../../TASKS.md) — the must-pin list. [T10](../../TASKS.md)
  — the wide-multiply routine. [T14](../../TASKS.md) — its fixture.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Two types, `int` and `fixed`, as Q14 ruled | Every rule about the seam: promotion, mixed-key equality, `sum` changing type mid-way, a two-column operator table, a divergence a player has to learn. Reading the finished `docs/01` was what showed the seam was most of the text. |
| One type, i64 scaled by 10⁶ | The seam is gone, but the integer part caps near 9 × 10¹², so two values around four million multiply to a fault, and six places is visible in `1 / 3`. Players would notice both. |
| **One type, i128 scaled by 10¹²** *(chosen)* | Nobody will notice either limit. A 256-bit intermediate for multiply and divide, hand-written, deterministic, and a fixture to pin it. Sixteen bytes per number, which nothing here cares about. |
| Arbitrary precision | No limits at all, and unbounded time and memory per operation, which Q14 already rejected for fighting the cost model. |

### How the answer took its shape

Q14 chose two types on the argument that `int` should stay exact and
unbounded-feeling while fractions got their own bounded type. Writing the
numbers part of `docs/01` against that ruling produced a document that was
mostly about the boundary between the two — and a player reads that boundary
as the language's arithmetic being complicated, when the intent was for it to
be Python's. Collapsing to one type was proposed first at Q14's scale, and the
objection was range: 10¹² is a number a game can reach. Widening the
representation to i128 answered that outright, and the extra places came free
with it. The cost moved from the player, who had a seam to learn, to the
implementer, who has one routine to write and one fixture to keep.
