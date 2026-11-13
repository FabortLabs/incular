//! Hashing for maps keyed by retained-tree identities.
//!
//! Element, render and semantic ids are small process-local arena handles,
//! never untrusted input. Hit testing, layout and the semantics pass look
//! them up once per visited node, where the default SipHash costs more than
//! the rest of the visit.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// Multiply-rotate hasher over machine words (the "Fx" construction).
#[derive(Clone, Copy, Default)]
pub(crate) struct FastHasher(u64);

impl FastHasher {
    const SEED: u64 = 0x517c_c1b7_2722_0a95;

    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(Self::SEED);
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
    }
    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }
    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.add(u64::from(value));
    }
    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }
    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }
    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        // The multiply leaves the low bits weakest; fold the high half in
        // so both the bucket index and the control byte are well mixed.
        self.0 ^ (self.0 >> 32)
    }
}

pub(crate) type FastHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;
pub(crate) type FastHashSet<K> = HashSet<K, BuildHasherDefault<FastHasher>>;
