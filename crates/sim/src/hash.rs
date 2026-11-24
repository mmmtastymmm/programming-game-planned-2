//! Deterministic state hashing for lockstep desync detection and
//! golden-replay tests (CLAUDE.md).
//!
//! Hand-rolled FNV-1a: platform-independent, and — unlike `DefaultHasher` —
//! not seeded with per-process randomness. `DefaultHasher`'s output is also
//! explicitly not guaranteed stable across Rust releases, which would silently
//! invalidate every checked-in fixture on a toolchain bump.

pub struct Fnv1a(u64);

impl Fnv1a {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    pub fn new() -> Self {
        Self(Self::OFFSET)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }

    /// `count` zero bytes, in one step: a zero byte leaves the state as it
    /// is under the exclusive-or and multiplies it by the prime, so a run
    /// of them is one multiplication by the prime raised to their count,
    /// in the same wrapping arithmetic. The value is exactly that of
    /// writing them one by one (T42).
    pub fn write_zeros(&mut self, count: u64) {
        let mut left = count;
        while left > 0 {
            let step = left.min(u64::from(u32::MAX));
            self.0 = self.0.wrapping_mul(Self::PRIME.wrapping_pow(step as u32));
            left = left.saturating_sub(step);
        }
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_i64(&mut self, v: i64) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Length-prefixed, so `["ab", "c"]` and `["a", "bc"]` do not collide.
    pub fn write_str(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.write_bytes(s.as_bytes());
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_of_zero_bytes_in_one_step_is_the_bytes_one_by_one() {
        for count in [0u64, 1, 2, 3, 7, 64, 1000, 43_264] {
            // After other bytes, and before them: the run is in a chain.
            let mut one = Fnv1a::new();
            let mut run = Fnv1a::new();
            one.write_str("before");
            run.write_str("before");
            for _ in 0..count {
                one.write_u8(0);
            }
            run.write_zeros(count);
            assert_eq!(one.finish(), run.finish(), "{count} zero bytes");
            one.write_u64(count);
            run.write_u64(count);
            assert_eq!(one.finish(), run.finish(), "after {count} zero bytes");
        }
        // A run is not nothing: it moves the hash.
        let mut h = Fnv1a::new();
        h.write_zeros(5);
        assert_ne!(h.finish(), Fnv1a::new().finish());
    }
}
