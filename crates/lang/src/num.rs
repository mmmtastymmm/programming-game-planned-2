//! `num`: the one number type (Q14, Q19). A 128-bit signed count of
//! trillionths — twelve decimal places — whose every operation is exact and
//! then **floored** to trillionths (`docs/01-language/numbers.md`).
//!
//! Multiplication and division need the exact product or quotient before it
//! is scaled back, which does not fit in 128 bits. The wide arithmetic below
//! is hand-written on `u128` limbs so it is bit-identical on every target;
//! T14 pins its boundary cases.

use crate::errors::{ExcClass, Exception};
use std::cmp::Ordering;
use std::fmt;

/// The scale of `num`: spec, not tuning (`numbers.md`). Changing it is a new
/// question number.
pub const SCALE: i128 = 1_000_000_000_000;
/// `SCALE` as the unsigned magnitude type.
const SCALE_U: u128 = 1_000_000_000_000;
/// Decimal places, which is `log10(SCALE)`.
pub const PLACES: u32 = 12;

/// A number: a count of trillionths.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Num(i128);

type R<T> = Result<T, Exception>;

fn overflow() -> Exception {
    Exception::new(ExcClass::OverflowError, vec![])
}
fn zero_division() -> Exception {
    Exception::new(ExcClass::ZeroDivisionError, vec![])
}

// ---------------------------------------------------------------- wide ---

/// An unsigned 256-bit magnitude as two `u128` limbs.
#[derive(Clone, Copy, PartialEq, Eq)]
struct U256 {
    hi: u128,
    lo: u128,
}

impl U256 {
    const ZERO: U256 = U256 { hi: 0, lo: 0 };

    fn from_u128(v: u128) -> U256 {
        U256 { hi: 0, lo: v }
    }

    /// Exact product of two `u128`s, by schoolbook on 64-bit halves.
    fn mul(a: u128, b: u128) -> U256 {
        let (a_hi, a_lo) = (a >> 64, a & u64::MAX as u128);
        let (b_hi, b_lo) = (b >> 64, b & u64::MAX as u128);
        // Each partial product of two 64-bit halves fits in 128 bits exactly.
        let ll = a_lo.wrapping_mul(b_lo);
        let lh = a_lo.wrapping_mul(b_hi);
        let hl = a_hi.wrapping_mul(b_lo);
        let hh = a_hi.wrapping_mul(b_hi);
        // Middle column: lh + hl + (ll >> 64), tracking its carry into hi.
        let (mid, c1) = lh.overflowing_add(hl);
        let (mid, c2) = mid.overflowing_add(ll >> 64);
        let carry = (c1 as u128).wrapping_add(c2 as u128);
        let lo = (mid << 64) | (ll & u64::MAX as u128);
        let hi = hh.wrapping_add(mid >> 64).wrapping_add(carry << 64);
        U256 { hi, lo }
    }

    fn shl1(self) -> U256 {
        U256 {
            hi: (self.hi << 1) | (self.lo >> 127),
            lo: self.lo << 1,
        }
    }

    fn bit(self, i: u32) -> bool {
        if i >= 128 {
            (self.hi >> i.wrapping_sub(128)) & 1 == 1
        } else {
            (self.lo >> i) & 1 == 1
        }
    }

    fn sub(self, other: U256) -> U256 {
        let (lo, borrow) = self.lo.overflowing_sub(other.lo);
        let hi = self.hi.wrapping_sub(other.hi).wrapping_sub(borrow as u128);
        U256 { hi, lo }
    }

    fn ge(self, other: U256) -> bool {
        (self.hi, self.lo) >= (other.hi, other.lo)
    }

    /// Quotient and remainder by a non-zero `u128`, by binary long division:
    /// 256 iterations, no target-dependent instruction anywhere.
    fn divrem(self, d: u128) -> (U256, u128) {
        debug_assert!(d != 0);
        let d = U256::from_u128(d);
        let mut q = U256::ZERO;
        let mut r = U256::ZERO;
        let mut i: u32 = 256;
        while i > 0 {
            i = i.wrapping_sub(1);
            r = r.shl1();
            if self.bit(i) {
                r.lo |= 1;
            }
            if r.ge(d) {
                r = r.sub(d);
                if i >= 128 {
                    q.hi |= 1u128 << i.wrapping_sub(128);
                } else {
                    q.lo |= 1u128 << i;
                }
            }
        }
        (q, r.lo)
    }

    /// The magnitude as an `i128` with the given sign, if it fits.
    fn to_i128(self, negative: bool) -> Option<i128> {
        if self.hi != 0 {
            return None;
        }
        if negative {
            if self.lo > (i128::MAX as u128).wrapping_add(1) {
                return None;
            }
            // Two's complement: -(lo) as i128, valid up to 2^127.
            Some((self.lo as i128).wrapping_neg())
        } else {
            i128::try_from(self.lo).ok()
        }
    }
}

fn magnitude(v: i128) -> u128 {
    v.unsigned_abs()
}

/// `floor(n / d)` on a 256-bit signed numerator and a non-zero `i128` divisor,
/// as an `i128` if it fits. Floors toward negative infinity: a non-zero
/// remainder on a negative quotient rounds the magnitude up.
fn floor_div_wide(n: U256, n_negative: bool, d: i128) -> R<i128> {
    let d_negative = d < 0;
    let (q, r) = n.divrem(magnitude(d));
    let negative = n_negative != d_negative;
    let q = if negative && r != 0 {
        // magnitude + 1
        let (lo, c) = q.lo.overflowing_add(1);
        U256 {
            hi: q.hi.wrapping_add(c as u128),
            lo,
        }
    } else {
        q
    };
    q.to_i128(negative).ok_or_else(overflow)
}

// ----------------------------------------------------------------- Num ---

impl Num {
    pub const ZERO: Num = Num(0);
    pub const ONE: Num = Num(SCALE);
    pub const MIN: Num = Num(i128::MIN);
    pub const MAX: Num = Num(i128::MAX);

    /// From a raw count of trillionths.
    pub const fn from_raw(raw: i128) -> Num {
        Num(raw)
    }

    pub const fn raw(self) -> i128 {
        self.0
    }

    /// From an integer, exactly, or `OverflowError`.
    pub fn from_int(i: i128) -> R<Num> {
        i.checked_mul(SCALE).map(Num).ok_or_else(overflow)
    }

    /// From a `bool`: `True` is `1`.
    pub fn from_bool(b: bool) -> Num {
        if b { Num::ONE } else { Num::ZERO }
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// No fractional part.
    pub fn is_integral(self) -> bool {
        self.0.checked_rem(SCALE) == Some(0)
    }

    /// The integral value as an `i128`, or `None` if fractional.
    pub fn as_integral(self) -> Option<i128> {
        if self.is_integral() {
            self.0.checked_div(SCALE)
        } else {
            None
        }
    }

    /// `int(x)`: the integral value nearest zero, as a `num`.
    pub fn trunc(self) -> Num {
        // Division truncates toward zero, which is exactly `int`'s rule.
        Num(self.0.checked_div(SCALE).unwrap_or(0).wrapping_mul(SCALE))
    }

    pub fn checked_add(self, o: Num) -> R<Num> {
        self.0.checked_add(o.0).map(Num).ok_or_else(overflow)
    }

    pub fn checked_sub(self, o: Num) -> R<Num> {
        self.0.checked_sub(o.0).map(Num).ok_or_else(overflow)
    }

    pub fn checked_neg(self) -> R<Num> {
        self.0.checked_neg().map(Num).ok_or_else(overflow)
    }

    pub fn checked_abs(self) -> R<Num> {
        self.0.checked_abs().map(Num).ok_or_else(overflow)
    }

    /// `a * b`: the exact product in 256 bits, floored to trillionths.
    pub fn checked_mul(self, o: Num) -> R<Num> {
        let product = U256::mul(magnitude(self.0), magnitude(o.0));
        let negative = (self.0 < 0) != (o.0 < 0) && self.0 != 0 && o.0 != 0;
        floor_div_wide(product, negative, SCALE).map(Num)
    }

    /// `a / b`: the exact quotient floored to trillionths.
    pub fn checked_div(self, o: Num) -> R<Num> {
        if o.0 == 0 {
            return Err(zero_division());
        }
        let scaled = U256::mul(magnitude(self.0), SCALE_U);
        floor_div_wide(scaled, self.0 < 0, o.0).map(Num)
    }

    /// `a // b`: floor division, integral.
    pub fn checked_floordiv(self, o: Num) -> R<Num> {
        if o.0 == 0 {
            return Err(zero_division());
        }
        let q = floor_div_wide(U256::from_u128(magnitude(self.0)), self.0 < 0, o.0)?;
        Num::from_int(q)
    }

    /// `a % b`: the divisor's sign, and `a == (a // b) * b + a % b` exactly.
    pub fn checked_rem(self, o: Num) -> R<Num> {
        let q = self.checked_floordiv(o)?;
        let qb = q.checked_mul(o)?;
        self.checked_sub(qb)
    }

    /// `a ** b` by the procedure `numbers.md` pins: square-and-multiply from
    /// the exponent's most significant bit down, every product floored; a
    /// negative exponent is `(1 / a) ** -b`. Returns the result and the number
    /// of multiplications and divisions performed, which the cost table
    /// charges.
    pub fn checked_pow(self, exp: Num) -> R<(Num, u32)> {
        let e = exp
            .as_integral()
            .ok_or_else(|| Exception::new(ExcClass::TypeError, vec![]))?;
        let mut steps: u32 = 0;
        let (base, e) = if e < 0 {
            if self.0 == 0 {
                return Err(zero_division());
            }
            steps = steps.wrapping_add(1);
            (Num::ONE.checked_div(self)?, e.unsigned_abs())
        } else {
            (self, e as u128)
        };
        if e == 0 {
            return Ok((Num::ONE, steps));
        }
        let top = 127u32.wrapping_sub(e.leading_zeros());
        let mut result = Num::ONE;
        let mut i = top;
        loop {
            if i != top {
                result = result.checked_mul(result)?;
                steps = steps.wrapping_add(1);
            }
            if (e >> i) & 1 == 1 {
                result = result.checked_mul(base)?;
                steps = steps.wrapping_add(1);
            }
            if i == 0 {
                break;
            }
            i = i.wrapping_sub(1);
        }
        // The first squaring of 1 is skipped above, so `x ** 1` is one
        // multiplication and `x ** 2` is `x * x` exactly.
        Ok((result, steps))
    }

    /// Floor to `places` decimal places (`:.Nf` and `round(x, n)`'s input).
    pub fn floor_to_places(self, places: u32) -> Num {
        if places >= PLACES {
            return self;
        }
        let unit = 10i128.pow(PLACES.wrapping_sub(places));
        Num(self.0.div_euclid(unit).wrapping_mul(unit))
    }

    /// `round(x, n)`: half to even, to `n` places.
    pub fn round_half_even(self, places: u32) -> Num {
        if places >= PLACES {
            return self;
        }
        let unit = 10i128.pow(PLACES.wrapping_sub(places));
        let q = self.0.div_euclid(unit);
        let r = self.0.rem_euclid(unit);
        let half = unit.checked_div(2).unwrap_or(0);
        let up = match r.cmp(&half) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => q.rem_euclid(2) == 1,
        };
        let q = if up { q.wrapping_add(1) } else { q };
        Num(q.wrapping_mul(unit))
    }

    /// Parse a literal: decimal digits with `_` between digits, an optional
    /// point, an optional exponent. Exactly representable or `None`.
    pub fn parse_literal(text: &str) -> Option<Num> {
        let (mantissa, exp) = match text.find(['e', 'E']) {
            Some(i) => (&text[..i], Some(&text[i.wrapping_add(1)..])),
            None => (text, None),
        };
        let (int_part, frac_part) = match mantissa.find('.') {
            Some(i) => (&mantissa[..i], &mantissa[i.wrapping_add(1)..]),
            None => (mantissa, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        let int_digits = digits(int_part)?;
        let frac_digits = digits(frac_part)?;
        let exp: i32 = match exp {
            Some(e) => e.parse().ok()?,
            None => 0,
        };
        // value = (int_digits.frac_digits) × 10^exp, as an exact count of
        // trillionths: shift the decimal point by PLACES + exp.
        let all: String = format!("{int_digits}{frac_digits}");
        let shift = (PLACES as i32)
            .checked_add(exp)?
            .checked_sub(frac_digits.len() as i32)?;
        let raw = if shift >= 0 {
            let mut v: i128 = 0;
            for c in all.chars() {
                v = v.checked_mul(10)?.checked_add(c.to_digit(10)? as i128)?;
            }
            v.checked_mul(10i128.checked_pow(shift as u32)?)?
        } else {
            // Digits past the twelfth place must all be zero.
            let cut = all.len().checked_sub(shift.unsigned_abs() as usize)?;
            if all[cut..].chars().any(|c| c != '0') {
                return None;
            }
            let mut v: i128 = 0;
            for c in all[..cut].chars() {
                v = v.checked_mul(10)?.checked_add(c.to_digit(10)? as i128)?;
            }
            v
        };
        Some(Num(raw))
    }

    /// Parse `int(s)`: whitespace, optional sign, digits with `_` between.
    pub fn parse_int_str(s: &str) -> R<Num> {
        let t = s.trim_matches(is_ascii_ws);
        let (neg, body) = strip_sign(t);
        let ds = digits(body).ok_or_else(|| Exception::new(ExcClass::ValueError, vec![]))?;
        if ds.is_empty() {
            return Err(Exception::new(ExcClass::ValueError, vec![]));
        }
        let mut v: i128 = 0;
        for c in ds.chars() {
            v = v
                .checked_mul(10)
                .and_then(|v| v.checked_add(c.to_digit(10).unwrap_or(0) as i128))
                .ok_or_else(overflow)?;
        }
        let v = if neg {
            v.checked_neg().ok_or_else(overflow)?
        } else {
            v
        };
        Num::from_int(v)
    }

    /// Parse `num(s)`: whitespace, optional sign, then exactly a literal.
    pub fn parse_num_str(s: &str) -> R<Num> {
        let t = s.trim_matches(is_ascii_ws);
        let (neg, body) = strip_sign(t);
        let n =
            Num::parse_literal(body).ok_or_else(|| Exception::new(ExcClass::ValueError, vec![]))?;
        if neg { n.checked_neg() } else { Ok(n) }
    }

    /// The shortest exact decimal (`str(x)`).
    pub fn to_decimal(self) -> String {
        let mag = magnitude(self.0);
        let int_part = mag.wrapping_div(SCALE_U);
        let frac = mag.wrapping_rem(SCALE_U);
        let sign = if self.0 < 0 { "-" } else { "" };
        if frac == 0 {
            return format!("{sign}{int_part}");
        }
        let frac = format!("{frac:0>12}");
        let frac = frac.trim_end_matches('0');
        format!("{sign}{int_part}.{frac}")
    }

    /// `:.Nf`: floored to `places`, printed with exactly `places` digits.
    pub fn to_fixed(self, places: u32) -> String {
        let v = self.floor_to_places(places);
        let mag = magnitude(v.0);
        let int_part = mag.wrapping_div(SCALE_U);
        let frac = mag.wrapping_rem(SCALE_U);
        let sign = if v.0 < 0 { "-" } else { "" };
        if places == 0 {
            return format!("{sign}{int_part}");
        }
        let frac = format!("{frac:0>12}");
        format!("{sign}{int_part}.{}", &frac[..places as usize])
    }
}

fn is_ascii_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c' | '\x0b')
}

fn strip_sign(t: &str) -> (bool, &str) {
    if let Some(rest) = t.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = t.strip_prefix('+') {
        (false, rest)
    } else {
        (false, t)
    }
}

/// Digits with `_` allowed only between two digits; returns the digits alone.
fn digits(s: &str) -> Option<String> {
    if s.is_empty() {
        return Some(String::new());
    }
    if s.starts_with('_') || s.ends_with('_') || s.contains("__") {
        return None;
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_digit() {
            out.push(c);
        } else if c != '_' {
            return None;
        }
    }
    Some(out)
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

impl fmt::Debug for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "num({})", self.to_decimal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A literal, with a leading `-` applied the way the parser's unary
    /// minus would.
    fn n(s: &str) -> Num {
        match s.strip_prefix('-') {
            Some(rest) => Num::parse_literal(rest)
                .expect("literal")
                .checked_neg()
                .expect("neg"),
            None => Num::parse_literal(s).expect("literal"),
        }
    }

    #[test]
    fn literals_are_exact_or_rejected() {
        assert_eq!(n("1.5").raw(), 1_500_000_000_000);
        assert_eq!(n("0.1").raw(), 100_000_000_000);
        assert_eq!(n("1e3"), n("1000"));
        assert_eq!(n("1.5e-2"), n("0.015"));
        assert_eq!(n("2.5000000000000"), n("2.5"));
        assert_eq!(n("3_000.25"), n("3000.25"));
        assert!(Num::parse_literal("0.0000000000001").is_none());
        assert!(Num::parse_literal("1e-13").is_none());
        assert!(Num::parse_literal("1_").is_none());
        assert!(Num::parse_literal("_1").is_none());
        assert!(Num::parse_literal(".").is_none());
    }

    #[test]
    fn floor_everywhere() {
        assert_eq!(n("2").checked_div(n("3")).unwrap(), n("0.666666666666"));
        assert_eq!(n("-2").checked_div(n("3")).unwrap(), n("-0.666666666667"));
        assert_eq!(n("0.1").checked_mul(n("0.1")).unwrap(), n("0.01"));
        assert_eq!(n("-7").checked_floordiv(n("2")).unwrap(), n("-4"));
        assert_eq!(n("-7").checked_rem(n("3")).unwrap(), n("2"));
        assert_eq!(n("7").checked_rem(n("-3")).unwrap(), n("-2"));
        assert_eq!(n("1.000001").checked_mul(n("0.5")).unwrap(), n("0.5000005"));
    }

    #[test]
    fn pow_follows_the_procedure() {
        assert_eq!(n("2").checked_pow(n("10")).unwrap().0, n("1024"));
        // `(1 / 3) ** 2`: the reciprocal floors to 0.333333333333 first, and
        // its square floors again — not the 0.111111111111 exact arithmetic
        // would give.
        assert_eq!(n("3").checked_pow(n("-2")).unwrap().0, n("0.111111111110"));
        assert_eq!(
            n("0.00001").checked_pow(n("-3")).unwrap().0,
            n("1000000000000000")
        );
        assert_eq!(n("7").checked_pow(n("0")).unwrap(), (Num::ONE, 0));
        assert_eq!(n("7").checked_pow(n("1")).unwrap().0, n("7"));
        let (sq, steps) = n("1.5").checked_pow(n("2")).unwrap();
        assert_eq!(sq, n("1.5").checked_mul(n("1.5")).unwrap());
        assert_eq!(steps, 2);
        assert!(n("2").checked_pow(n("0.5")).is_err());
        assert!(n("0").checked_pow(n("-1")).is_err());
    }

    #[test]
    fn boundaries() {
        assert!(Num::MAX.checked_add(Num::ONE).is_err());
        assert!(Num::MIN.checked_neg().is_err());
        assert!(Num::MIN.checked_abs().is_err());
        // The largest integral value and its square overflow.
        let big = Num::from_int(170_141_183_460_469_231_731_687_303).unwrap();
        assert!(big.checked_mul(big).is_err());
        // A product that needs 256 bits but fits after scaling.
        let a = Num::from_int(1_000_000_000_000_000_000_000).unwrap();
        assert_eq!(
            a.checked_mul(n("2")).unwrap(),
            Num::from_int(2_000_000_000_000_000_000_000).unwrap()
        );
        assert!(n("1").checked_div(Num::ZERO).is_err());
        assert!(n("1").checked_rem(Num::ZERO).is_err());
    }

    #[test]
    fn printing_is_the_shortest_exact_decimal() {
        assert_eq!(n("2").to_decimal(), "2");
        assert_eq!(n("1.5").to_decimal(), "1.5");
        assert_eq!(
            n("1").checked_div(n("3")).unwrap().to_decimal(),
            "0.333333333333"
        );
        assert_eq!(n("-0.5").to_decimal(), "-0.5");
        assert_eq!(n("-2").checked_div(n("3")).unwrap().to_fixed(2), "-0.67");
        assert_eq!(n("-0.4").to_fixed(0), "-1");
        assert_eq!(n("3.14159").to_fixed(3), "3.141");
    }

    #[test]
    fn rounding_and_conversion() {
        assert_eq!(n("2.5").round_half_even(0), n("2"));
        assert_eq!(n("3.5").round_half_even(0), n("4"));
        assert_eq!(n("-2.5").round_half_even(0), n("-2"));
        assert_eq!(n("-1.5").trunc(), n("-1"));
        assert_eq!(Num::parse_int_str(" +42 ").unwrap(), n("42"));
        assert!(Num::parse_int_str("1.5").is_err());
        assert!(Num::parse_int_str("1_").is_err());
        assert_eq!(Num::parse_num_str(" +2 ").unwrap(), n("2"));
        assert!(Num::parse_num_str("0.0000000000001").is_err());
        assert!(Num::parse_int_str(&"1".repeat(60)).is_err());
    }
}
