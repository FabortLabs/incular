//! Process-wide identity counters.

use std::sync::atomic::{AtomicU64, Ordering};

/// Hands out unique, increasing `u64` identities from a `static` counter.
///
/// Identities are only compared for equality, so the counter uses relaxed
/// ordering. It never wraps: an exhausted counter reports exhaustion instead
/// of reusing an identity.
///
/// ```
/// use incular_core::IdGenerator;
///
/// static NEXT_ID: IdGenerator = IdGenerator::new();
/// let first = NEXT_ID.next();
/// assert!(NEXT_ID.next() > first);
/// ```
#[derive(Debug)]
pub struct IdGenerator {
    next: AtomicU64,
}

impl IdGenerator {
    /// A generator whose first identity is `1`, leaving `0` free as a
    /// sentinel.
    #[must_use]
    pub const fn new() -> Self {
        Self::starting_at(1)
    }

    /// A generator whose first identity is `first`.
    #[must_use]
    pub const fn starting_at(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }

    /// The next identity, or `None` once every `u64` has been handed out.
    #[must_use]
    pub fn try_next(&self) -> Option<u64> {
        self.next
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .ok()
    }

    /// The next identity.
    ///
    /// # Panics
    ///
    /// Panics once every `u64` has been handed out, which takes centuries
    /// at any realistic rate. Use [`Self::try_next`] to handle it.
    #[must_use]
    pub fn next(&self) -> u64 {
        self.try_next().expect("identity space exhausted")
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}
