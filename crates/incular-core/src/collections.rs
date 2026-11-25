//! Flutter-style equality and sorting helpers for Rust collections.

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::{BuildHasher, Hash},
    ops::Range,
};

/// Compares nullable lists by length, order, and each element's equality.
/// Two references to the same slice compare equal without reading elements.
#[must_use]
pub fn list_equals<T: PartialEq>(a: Option<&[T]>, b: Option<&[T]>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::ptr::eq(a, b) || a == b,
        _ => false,
    }
}

/// Compares nullable sets by membership, ignoring iteration order.
#[must_use]
pub fn set_equals<T: Eq + Hash, A: BuildHasher, B: BuildHasher>(
    a: Option<&HashSet<T, A>>,
    b: Option<&HashSet<T, B>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.len() == b.len() && (std::ptr::addr_eq(a, b) || a.iter().all(|item| b.contains(item)))
        }
        _ => false,
    }
}

/// Compares nullable maps by keys and values, ignoring iteration order.
/// A missing key is distinct from a present key with an optional empty value.
#[must_use]
pub fn map_equals<K: Eq + Hash, V: PartialEq, A: BuildHasher, B: BuildHasher>(
    a: Option<&HashMap<K, V, A>>,
    b: Option<&HashMap<K, V, B>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.len() == b.len()
                && (std::ptr::addr_eq(a, b)
                    || a.iter().all(|(key, value)| b.get(key) == Some(value)))
        }
        _ => false,
    }
}

/// Returns the index of `value` in an already sorted slice, or `None`.
/// As in Flutter, duplicates return the first matching midpoint encountered
/// by the search, not necessarily the first occurrence in the slice.
#[must_use]
pub fn binary_search<T: Ord>(sorted: &[T], value: &T) -> Option<usize> {
    let (mut lo, mut hi) = (0, sorted.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match sorted[mid].cmp(value) {
            Ordering::Equal => return Some(mid),
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
        }
    }
    None
}

/// Sorts in place, preserving the relative order of equal elements.
/// Uses Rust's stable slice sort rather than prescribing Dart's algorithm.
pub fn merge_sort<T: Ord>(list: &mut [T]) {
    list.sort();
}

/// Stably sorts a subrange with a comparator without moving elements outside it.
///
/// # Panics
///
/// Panics if the range is not a valid half-open range of the list.
pub fn merge_sort_by<T>(
    list: &mut [T],
    range: Range<usize>,
    compare: impl FnMut(&T, &T) -> Ordering,
) {
    list[range].sort_by(compare);
}
