//! Flutter's small enum-indexed bit field, stored in a 64-bit mask.

use std::{marker::PhantomData, ops::Index};

/// Largest unsigned integer kept in a Flutter VM small-integer bit field.
pub const K_MAX_UNSIGNED_SMI: u64 = (1_u64 << 62) - 1;

/// Up to 62 independent bits keyed by a zero-based index. For an enum,
/// implement `From<YourEnum> for usize` to define the indices explicitly.
///
/// Rust uses a real `u64` on all supported targets, including WebAssembly.
/// Unlike Flutter's JavaScript fallback, this type does not need a web stub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitField<T = usize> {
    bits: u64,
    len: usize,
    index: PhantomData<fn(T)>,
}

impl<T> BitField<T> {
    pub const MAX_LEN: usize = 62;

    /// Creates a field with all bits clear.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds 62.
    #[must_use]
    pub const fn new(len: usize) -> Self {
        Self::filled(len, false)
    }

    /// Creates a field with all valid bits set to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds 62.
    #[must_use]
    pub const fn filled(len: usize, value: bool) -> Self {
        assert!(len <= Self::MAX_LEN, "bit field length must be at most 62");
        Self {
            bits: Self::filled_bits(len, value),
            len,
            index: PhantomData,
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    const fn filled_bits(len: usize, value: bool) -> u64 {
        if value { (1_u64 << len) - 1 } else { 0 }
    }

    /// Sets every bit at once; pass `false` to clear the field.
    pub const fn reset(&mut self, value: bool) {
        self.bits = Self::filled_bits(self.len, value);
    }
}

impl<T: Into<usize>> BitField<T> {
    fn mask(&self, index: T) -> u64 {
        let index = index.into();
        assert!(
            index < self.len,
            "bit field index {index} outside length {}",
            self.len
        );
        1_u64 << index
    }

    /// Reads one bit.
    ///
    /// # Panics
    ///
    /// Panics if the index is outside the field.
    #[must_use]
    pub fn get(&self, index: T) -> bool {
        self.bits & self.mask(index) != 0
    }

    /// Changes one bit without touching its neighbors.
    ///
    /// # Panics
    ///
    /// Panics if the index is outside the field.
    pub fn set(&mut self, index: T, value: bool) {
        let mask = self.mask(index);
        if value {
            self.bits |= mask;
        } else {
            self.bits &= !mask;
        }
    }
}

impl<T: Into<usize>> Index<T> for BitField<T> {
    type Output = bool;

    fn index(&self, index: T) -> &Self::Output {
        if self.get(index) { &true } else { &false }
    }
}
