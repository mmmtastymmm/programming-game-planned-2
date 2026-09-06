*Closed record — see [../README.md](../README.md). Not spec.*

# Q14 — The number model

## Ruling

*Ruling (2026-09-06):* **two numeric types, `int` and `fixed`, and no float.**

- **`int`** is a 64-bit signed integer. Overflow is a `fault` (Q8), delivered
  at the overflowing operation.
- **`fixed`** is a 64-bit signed integer scaled by 10⁶ — six decimal places,
  range ±9,223,372,036,854.775807. Overflow is a `fault`. There is no infinity,
  no NaN, and no negative zero: a `fixed` is an integer count of millionths and
  behaves like one.
- **`/` returns a `fixed`**, whatever its operands. `7 / 2` is `3.5`, as a
  Python programmer expects; `1 / 3` is `0.333333`, which is where the
  resemblance to Python's `float` ends and the only place a player has to
  notice it.

`float` is not a name in the language. A literal with a decimal point or an
exponent is a `fixed`; a literal the type cannot represent exactly — a seventh
decimal place, a magnitude past the range — is rejected at parse time rather
than silently rounded.

### The rules

Each is hash-affecting.

1. **Promotion.** Arithmetic between an `int` and a `fixed` promotes the `int`
   to `fixed` exactly, and faults if it does not fit. Comparison and equality
   across the two types are by value and exact, so `1 == fixed(1)` and equal
   values are the same `dict` key, as in Python.
2. **`//` and `%` floor**, with Python's semantics: `-7 // 2` is `-4` and
   `-7 % 3` is `2`. Both operands `int` gives `int`; otherwise `fixed`.
3. **Every `fixed` operation whose exact result is not representable rounds
   toward negative infinity.** One rounding rule, the same one `//` already
   uses, so the error of any operation is bounded by one millionth and always
   in the same direction. `2 / 3` is `0.666666`.
4. **Division and modulo by zero fault.** No infinity to return.
5. **`**`.** `int ** int` with a non-negative exponent is `int` and may
   overflow-fault. A negative `int` exponent, or a `fixed` base, gives `fixed`.
   A `fixed` *exponent* faults: there is no transcendental arithmetic in the
   language, and a root or a logarithm, if the game needs one, is a builtin with
   a specified integer algorithm, not an operator.
6. **Conversion follows Python.** `int(x)` on a `fixed` truncates toward zero;
   `fixed(n)` on an `int` is exact; `fixed(s)` accepts the literal grammar and
   nothing else. `str` of a `fixed` is its shortest exact decimal, and `docs/01`
   pins the format so that printing is one string on every peer.
7. **`bool` is an `int`**, as in Python: `True + 1` is `2`.
8. **Builtins follow Python with `fixed` in place of `float`.** `abs`, `min`,
   `max`, `sum` and `round` accept either type; `round` returns an `int` and
   rounds half to even, which is Python's rule. `docs/01` lists them.

### The scale is spec, not tuning

CLAUDE.md treats every number in the docs as a tuning constant that belongs in
a data file. The scale of `fixed` is the exception and is named as one: it
decides which literals parse, what every division returns, and therefore every
replay hash. It is stated once, in `docs/01`, and changing it is a new question
number.

### Consequences

- **Q13's first divergence shrinks.** It read "no floats, and `/` does not do
  what Python does". It now reads: no floats; `1.5` is a `fixed` with six
  places; `/` returns one; overflow faults. A Python programmer's arithmetic
  works until they need a seventh decimal place or a number past nine
  trillion, and then it fails loudly rather than drifting.
- **Every failure this model can produce is a `fault`.** Overflow, division by
  zero, a `fixed` exponent, an `int` too large to promote. Q8 gives each a
  defined landing, and none is silent.
- **`docs/01` is unblocked.** This was the last of its three blockers. T8 writes
  it.
- **The determinism scan needs a note.** `crates/sim/tests/no_floats.rs` flags
  a Rust float literal anywhere on a line, string literals included. A
  player-program fixture containing `1.5` inside a Rust string will read as a
  violation once the scan covers the language crate — the fixtures need to live
  outside Rust source, or the scan needs to skip strings. Recorded on T14.

## Outcome

- **Docs:** [00-overview.md](../../00-overview.md) — Decided section, the Q13
  bullet's float exclusion, the reserved-docs table and the paragraph after it.
  [CLAUDE.md](../../../CLAUDE.md) — crate layout; the tuning-constant rule gains
  its exception. [QUESTIONS.md](../../QUESTIONS.md) — the status block.
- **Task:** [T8](../../TASKS.md) — no longer waits on anything, and its
  must-pin list gains the number model's details. [T14](../../TASKS.md) — the
  scan-versus-fixture note above.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Fixed-width `i64`, overflow faults | One number type, no surprises, and overflow is a legible in-game failure. Q8 already makes it a `fault` interrupt delivered at the overflowing operation, so the boundary is spec. **Kept, as the `int` half.** |
| Fixed-width `i64`, overflow wraps | Never faults, never surprises the sim. Silently wrong answers are worse than loud ones in a language players debug under time pressure. |
| Arbitrary-precision integers | No overflow to specify at all, and deterministic. Unbounded memory and time per operation, which fights the per-tick cost model Q5 chose to own. |
| **Fixed-point values for fractions** *(chosen, alongside `int`)* | Makes division expressible without floats. A second numeric type, and every mixed-type operation is a rule someone has to remember — eight rules, above. |

And for what `/` does:

| Option | What it costs |
|---|---|
| Reject at parse time, forcing `//` | The most honest and the most annoying; the only option that cannot silently produce a different answer than the player expected. Only necessary if there is nothing fractional to return. |
| Silently mean integer division | Familiar-looking and quietly un-Pythonic — `7 / 2` is `3` and nobody is told. |
| **Return a `fixed`** *(chosen)* | Python's own behavior up to precision. Requires the second type to exist at all, which is the trade this ruling makes. |

### How the answer took its shape

The single-type option was the lean going in: `i64`, overflow faults, `/`
rejected so players write `//`. It has the smallest rule set and every failure
is loud. It lost on the same argument that widened Q13: Python was chosen for
familiarity, and a language where `7 / 2` is a parse error spends that
familiarity on the first line of arithmetic a player writes. A unit that moves
at a speed, aims at an angle, or splits a resource three ways needs a fraction,
and forcing every one of those through `//` and a hand-chosen scale factor is
the fixed-point type implemented by every player separately, without the
determinism guarantees.

**Decimal rather than binary scale.** A binary fixed-point (say 32.32) makes
multiplication a shift, but `0.1` is not representable in it, and a player who
types `0.1` and reads back `0.09999999` has met the float problem wearing a
different hat. A decimal scale makes every literal a player can type exact up
to the place limit, and makes the limit itself legible: six places.

**Six places.** Enough for a position, a speed, a ratio and a probability; a
range past nine trillion for the integer part; and a product of two values
fits in 128 bits for the intermediate, which is one multiply and one divide in
Rust. More places buy precision the game has no use for and cost integer range
it might. Pinned in `docs/01` as spec and changeable only under a new number.

**Floor everywhere.** Round-to-nearest gives a smaller error but needs a tie
rule, and then the language has two rounding rules — one for `//` and `%`,
which Python fixes as floor, and one for everything else. One rule, always
the same direction, bounded by one millionth, was judged easier to reason
about under time pressure than a smaller error a player cannot predict the
sign of. `round()` keeps Python's half-to-even because it is a named
operation a player reaches for deliberately.

**The register said "rationals"; the ruling says fixed-point.** A true rational
type — numerator and denominator — is exact but has unbounded denominators,
which is the arbitrary-precision problem again under another name. Fixed-point
is the bounded form of the same idea.

**What was not decided here.** Which roots, trigonometric or other functions the
game provides as builtins, and by what integer algorithm, is `docs/01`'s
builtin list informed by what `docs/02` needs. This ruling only says that they
are builtins with specified algorithms, never operators, and never inherited
from a host library.
