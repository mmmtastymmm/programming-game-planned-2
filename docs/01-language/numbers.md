*Part of [01-language](../01-language.md).*

# Numbers

One numeric type and no float. Every operator's result, every rounding, and
every way arithmetic can fail is here, and nothing about a number is left to
the host.

## Decided

- **The number model (Q14, as amended by Q19).** Q14 ruled a fractional type
  with a fixed decimal scale in place of floats, with these properties, which
  stand: `/` returns a fraction, so `7 / 2` is `3.5` as a Python programmer
  expects; `//` and `%` floor with Python's semantics; every result that is
  not representable rounds toward negative infinity — one rounding rule, the
  one `//` already has; overflow, division by zero and a fractional exponent
  are all faults (Q8); a literal the type cannot represent exactly is rejected
  at parse time; `round` keeps Python's half-to-even; and the scale is language
  spec stated once here, not a tuning constant, because it decides every
  replay hash. Q14's second type, a separate `int`, and its scale of 10⁶ are
  withdrawn by Q19.
- **One number type, `num`, an i128 scaled by 10¹² (Q19).** Every number is a
  `num`: a 128-bit signed count of trillionths, twelve decimal places, an
  integer part to about 1.7 × 10²⁶. `2` and `2.0` are one value. There is no
  `int` type, no `fixed`, no `float`, and no bitwise operators; `int(x)`
  survives as the truncating conversion and returns a `num`. An index, a
  count, or any other integral context given a value with a fractional part is
  a `TypeError`. Printing is the shortest exact decimal, so an integral value
  prints without a point. The representation is spec: the exact product of two
  values needs 256 bits before it is scaled back.

## The type

| | |
|---|---|
| Representation | 128-bit two's complement count of **trillionths** (10⁻¹²) |
| Places | 12 |
| Range | −170 141 183 460 469 231 731 687 303.715884105728 … 170 141 183 460 469 231 731 687 303.715884105727 (an integer part of 27 digits) |

**The scale of `num` is 10¹².** This number is spec: it decides which literals
parse and what every division returns, so it is stated here and nowhere else,
and changing it is a new question number, not a tuning edit.

`bool` is a subclass of `num`: `True` is `1`, `False` is `0`, `True + 1` is
`2`. A `num` is falsy when it is zero.

There is no `int`, `fixed`, `float`, infinity, NaN, negative zero, complex, or
arbitrary-precision integer. `isinstance(x, num)` is the type query.

**Integral values.** A `num` with no fractional part is *integral*. Wherever
Python wants an `int` — an index, a slice bound, a `range` argument, a `str`
repetition count, a `zfill` width, `round`'s place count — the language wants
an integral `num`, and a value with a fractional part there is a `TypeError`.
`len`, `index`, `count`, `find` and `enumerate` produce integral values.

## Literals

`0`, `42`, `1_000_000`, `1.5`, `.5`, `2.`, `1e3`, `1.5e-2`, `3_000.25`,
`0.000000000001`. All are `num`. The literal's value must be an exact count of
trillionths within range, or it is a parse error: `0.0000000000001` (thirteen
places) is not a literal, and neither is `1e-13`. `2.5000000000000` is,
because its value is. Hex, octal and binary forms do not exist.

A parse error is a load failure ([syntax](syntax.md#the-bundle)), never a
runtime rounding.

## Operators

| Expression | Result | Rule |
|---|---|---|
| `a + b`, `a - b` | `num` | exact, then overflow check |
| `a * b` | `num` | the exact product in 256 bits, floored to trillionths, then overflow check |
| `a / b` | `num` | the exact quotient floored to trillionths |
| `a // b` | `num` | floor division, Python's semantics: `-7 // 2` is `-4`; the result is integral |
| `a % b` | `num` | the divisor's sign: `-7 % 3` is `2`; `a == (a // b) * b + a % b` holds exactly |
| `a ** b` | `num` | see [exponentiation](#exponentiation) |
| `-a`, `+a` | `num` | `-` of the minimum value overflows |
| `a < b` etc., `a == b` | `bool` | by value |

**Rounding.** Every result that is not an exact count of trillionths rounds
**toward negative infinity** — the same direction as `//`, so the language has
one rounding rule. `2 / 3` is `0.666666666666`; `-2 / 3` is
`-0.666666666667`; `0.1 * 0.1` is `0.01`. The error of any single operation is
less than one trillionth and always in the same direction.

**Division and modulo by zero** — `/`, `//`, `%` — raise `ZeroDivisionError`.

**Overflow** anywhere raises `OverflowError`. Nothing wraps.

**No bitwise operators.** `& | ^ ~ << >>` do not exist on numbers. `| & - ^`
are set operators only ([syntax](syntax.md#collections)).

**Comparison chains** (`0 <= x < w`) evaluate each operand once, left to right,
short-circuiting, as in Python. Comparing a number with a non-number by `<` is
a `TypeError`; `==` between a number and a non-number is `False`.

### Exponentiation

| Exponent | Result |
|---|---|
| integral, `≥ 0` | square-and-multiply in one fixed order: `result = 1`; for each bit of the exponent from the most significant down, `result = result * result`, then if the bit is set `result = result * x`; every product floored to trillionths and overflow-checked as `*` is. `x ** 0` is `1`, `x ** 1` is `x`, `x ** 2` equals `x * x` |
| integral, `< 0` | `(1 / x) ** -b`: the reciprocal first, floored as `/` is, then the positive power by the procedure above — so `0.00001 ** -3` is `1000000000000000`, not a division by zero |
| fractional | `TypeError`. There is no transcendental arithmetic in the language; a root, if the game needs one, is a builtin with a specified integer algorithm |

`0 ** 0` is `1`; `0 ** n` for negative `n` is a `ZeroDivisionError`.

## Conversions

| Call | Result |
|---|---|
| `int(x)` | of a `num`, the integral value nearest zero: `int(-1.5)` is `-1`, `int(True)` is `1`. Of a `str`: optional surrounding whitespace, an optional sign, then digits and `_` only — `int("1.5")` is a `ValueError`, as in Python |
| `num(x)` | of a `num`, itself; of a `bool`, `0` or `1`. Of a `str`: optional surrounding whitespace, an optional sign, then exactly what the parser accepts as a literal, else `ValueError` — `num(" +2 ")` is `2` and `num("0.0000000000001")` is a `ValueError` |
| `str(x)` | the **shortest exact decimal**: a leading `-` if negative, the integer part, then a point and the fractional digits with trailing zeros removed — and no point at all when there are none. `str(2)` is `"2"`, `str(1.5)` is `"1.5"`, `str(1 / 3)` is `"0.333333333333"`, `str(-0.5)` is `"-0.5"`. Never scientific notation. |
| `bool(x)` | `False` for zero, else `True` |

**f-string numeric specs**, the complete set — this part owns the list and
[syntax](syntax.md#expressions) cites it:

| Spec | Prints |
|---|---|
| `:.Nf`, `0 ≤ N ≤ 12` | exactly `N` fractional digits, truncating (not rounding) the rest |
| `:d` | the value, which must be integral, as `str` does; `TypeError` otherwise |
| `:,` | as `str`, with `,` every three digits of the integer part |
| `:0Wd` | as `:d`, zero-padded to width `W` |

The alignment specs `:>W`, `:<W`, `:^W` apply to any value and are
[syntax](syntax.md#expressions)'s. No other numeric specs exist; `:e`, `:g`,
`:%`, `:x` and `:b` are parse errors.

## The numeric builtins

| Builtin | Behavior |
|---|---|
| `abs(x)` | overflow at the minimum value |
| `min(…)`, `max(…)` | by `<`; ties return the first |
| `sum(iterable[, start])` | left to right; `start` defaults to `0`; overflow faults at the element that causes it |
| `round(x)` | the nearest integral value, **half to even**: `round(2.5)` is `2`, `round(3.5)` is `4`, `round(-2.5)` is `-2` |
| `round(x, n)` | rounded half to even to `n` places, `0 ≤ n ≤ 12` and integral; otherwise a `ValueError` |
| `sorted`, `list.sort` | by value; stable |

`round` is the one place a number rounds to nearest rather than down: it is a
named operation a player reaches for deliberately, and it keeps Python's rule.

Roots, trigonometry and any other function the game needs are **game
builtins** (`docs/02`), each with a specified integer algorithm, a specified
result and a specified cost — never an operator, never inherited from a host
library.

## The faults arithmetic can raise

| Exception | Raised by |
|---|---|
| `OverflowError` | any result outside the range; `-` or `abs` of the minimum value |
| `ZeroDivisionError` | `/`, `//`, `%` by zero; `0 ** n` for negative `n` |
| `TypeError` | a fractional exponent; a fractional value in an integral context; `<` against a non-number |
| `ValueError` | `int` or `num` of an unparsable string; `round` with `n` out of range |

Each is an ordinary exception: catchable, and a `fault` if not caught
([execution](execution.md#interrupts)).
