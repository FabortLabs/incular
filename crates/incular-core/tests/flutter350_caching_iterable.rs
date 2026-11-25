use std::cell::Cell;

use incular_core::{CachingIndexError, CachingIterable};

fn range(count: &Cell<usize>) -> CachingIterable<impl Iterator<Item = i32> + '_> {
    CachingIterable::new((1..=5).inspect(|_| count.set(count.get() + 1)))
}

#[test]
fn length_and_later_reads_share_the_cache() {
    let count = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    assert_eq!(list.len(), 5);
    assert_eq!(count.get(), 5);
    assert_eq!(list.len(), 5);
    assert_eq!(count.get(), 5);
    assert_eq!(list.last(), Some(5));
    assert_eq!(count.get(), 5);
    assert!(list.iter().eq(1..=5));
    assert_eq!(count.get(), 5);
}

#[test]
fn first_find_and_last_fill_only_as_far_as_needed() {
    let count = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    assert_eq!(list.first(), Some(1));
    assert_eq!(count.get(), 1);
    assert_eq!(list.iter().find(|item| *item == 3), Some(3));
    assert_eq!(count.get(), 3);
    assert_eq!(list.last(), Some(5));
    assert_eq!(count.get(), 5);
}

#[test]
fn filter_and_map_share_base_results_and_cache_their_outputs() {
    let count = Cell::new(0);
    let calls = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    let evens = list.filter(|item| item % 2 == 0);
    assert_eq!(count.get(), 0);
    assert_eq!(evens.first(), Some(2));
    assert_eq!(count.get(), 2);
    assert_eq!(list.first(), Some(1));
    assert_eq!(count.get(), 2);
    let mapped = evens.map(|item| {
        calls.set(calls.get() + 1);
        item + 1
    });
    assert!(mapped.iter().eq([3, 5]));
    assert_eq!(count.get(), 5);
    assert!(mapped.iter().eq([3, 5]));
    assert_eq!(calls.get(), 2);
    assert!(evens.iter().eq([2, 4]));
    assert_eq!(count.get(), 5);
    assert!(list.iter().eq([1, 2, 3, 4, 5]));
    assert_eq!(count.get(), 5);
}

#[test]
fn take_and_skip_do_not_request_an_extra_item() {
    let count = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    let skip = list.skip(1);
    let second_two = skip.take(2);
    assert_eq!(count.get(), 0);
    assert!(second_two.iter().eq([2, 3]));
    assert_eq!(count.get(), 3);
    let bounded = list.take_while(|item| *item < 4);
    let result = bounded.skip_while(|item| *item < 3);
    assert!(result.iter().eq([3]));
    assert_eq!(count.get(), 4);
    assert!(list.iter().eq(1..=5));
    assert_eq!(count.get(), 5);
}

#[test]
fn expansion_reads_the_base_once_across_two_derived_sequences() {
    let count = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    let first = list.flat_map(|item| [item, item]);
    assert_eq!(count.get(), 0);
    let expected = [1, 1, 2, 2, 3, 3, 4, 4, 5, 5];
    assert!(first.iter().eq(expected));
    assert_eq!(count.get(), 5);
    let second = list.flat_map(|item| [item, item]);
    assert_eq!(count.get(), 5);
    assert!(second.iter().eq(expected));
    assert_eq!(count.get(), 5);
}

#[test]
fn signed_random_access_reports_both_bounds_and_reuses_the_cache() {
    let count = Cell::new(0);
    let list = range(&count);
    assert_eq!(count.get(), 0);
    assert_eq!(list.element_at(-1), Err(CachingIndexError { index: -1 }));
    assert_eq!(count.get(), 0);
    for (index, expected) in [(1, 2), (0, 1), (2, 3), (4, 5), (3, 4)] {
        assert_eq!(list.element_at(index), Ok(expected));
    }
    assert_eq!(list.element_at(5), Err(CachingIndexError { index: 5 }));
    assert_eq!(count.get(), 5);
}

#[test]
fn interleaved_cursors_fuse_the_source_at_first_exhaustion() {
    let calls = Cell::new(0);
    let list = CachingIterable::new(std::iter::from_fn(|| {
        let index = calls.get();
        calls.set(index + 1);
        match index {
            0 => Some(10),
            1 => Some(20),
            2 => None,
            _ => panic!("source was advanced after exhaustion"),
        }
    }));
    let mut a = list.iter();
    let mut b = list.iter();
    assert!(!list.is_empty());
    assert_eq!(calls.get(), 1);
    assert_eq!(a.next(), Some(10));
    assert_eq!(a.next(), Some(20));
    assert_eq!(b.next(), Some(10));
    assert_eq!(b.next(), Some(20));
    assert_eq!(b.next(), None);
    assert_eq!(a.next(), None);
    assert_eq!(list.len(), 2);
    assert_eq!(calls.get(), 3);
}
