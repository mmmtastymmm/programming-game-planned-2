//! Named, seeded RNG streams (CLAUDE.md determinism rule 4).
//!
//! One consumer domain per stream. The point of naming them is isolation: if
//! combat rolls and movement jitter share a stream, adding one combat roll
//! shifts every future movement decision, and a change that should have been
//! local desyncs the whole match against a peer running the old build. Streams
//! also make replays legible — a hash divergence names the domain that caused
//! it.
//!
//! Adding a stream is a design act: name it here, and never draw from it in
//! more than one domain.

use crate::hash::Fnv1a;

/// One named randomness domain, advanced only by sim systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    name: &'static str,
    state: u64,
}

impl Stream {
    /// Derive the stream's initial state from the match seed and its name,
    /// then scramble once so streams with related names still decorrelate
    /// (`"wander"` and `"wander2"` hash to neighbours; one SplitMix64 step
    /// separates them).
    pub fn new(seed: u64, name: &'static str) -> Self {
        let mut h = Fnv1a::new();
        h.write_u64(seed);
        h.write_str(name);
        let mut state = h.finish();
        next_rand(&mut state);
        Self { name, state }
    }

    pub fn next_u64(&mut self) -> u64 {
        next_rand(&mut self.state)
    }

    /// Uniform-ish in `0..n`. Modulo bias is accepted deliberately: it is
    /// identical on every machine, which is the only property that matters
    /// here. Panics on `n == 0` — an empty choice set is a caller bug.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "Stream::below(0)");
        self.next_u64() % n
    }

    /// Feed the stream's *state* into a state hash. A replay that agrees on
    /// entity positions but has drawn a different number of randoms has
    /// already desynced; this makes it fail now rather than three ticks later.
    pub fn hash_into(&self, h: &mut Fnv1a) {
        h.write_str(self.name);
        h.write_u64(self.state);
    }
}

/// Deterministic SplitMix64 step. Every randomness draw in the sim goes
/// through here, against one named stream's state.
pub fn next_rand(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}
