//! Pinned foundation/collections_test.dart equality and stable sort behavior.
use incular_core::collections::{
    binary_search, list_equals, map_equals, merge_sort, merge_sort_by, set_equals,
};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[test]
fn nullable_lists_compare_contents_in_order() {
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    let c = [1, 2];
    let d = [3, 2, 1];
    assert_eq!(
        [
            list_equals::<i32>(None, None),
            list_equals(Some(&a), None),
            list_equals(None, Some(&b)),
            list_equals(Some(&a), Some(&a)),
            list_equals(Some(&a), Some(&b)),
            list_equals(Some(&a), Some(&c)),
            list_equals(Some(&a), Some(&d)),
        ],
        [true, false, false, true, true, false, false]
    );
    let a = [f32::NAN];
    let b = [f32::NAN];
    assert!(list_equals(Some(&a), Some(&a)));
    assert!(!list_equals(Some(&a), Some(&b)));
    assert!(!list_equals::<u8>(None, Some(&[])));
}

#[test]
fn nullable_sets_compare_membership_without_order() {
    let a = HashSet::from([1, 2, 3]);
    let b = HashSet::from([1, 2, 3]);
    let c = HashSet::from([1, 2]);
    let d = HashSet::from([3, 2, 1]);
    let none: Option<&HashSet<i32>> = None;
    assert_eq!(
        [
            set_equals(none, none),
            set_equals(Some(&a), none),
            set_equals(none, Some(&b)),
            set_equals(Some(&a), Some(&a)),
            set_equals(Some(&a), Some(&b)),
            set_equals(Some(&a), Some(&c)),
            set_equals(Some(&a), Some(&d)),
        ],
        [true, false, false, true, true, false, true]
    );
}

#[test]
fn nullable_maps_compare_key_values_without_order() {
    let a = HashMap::from([(1, 1), (2, 2), (3, 3)]);
    let b = HashMap::from([(1, 1), (2, 2), (3, 3)]);
    let c = HashMap::from([(1, 1), (2, 2)]);
    let d = HashMap::from([(3, 3), (2, 2), (1, 1)]);
    let e = HashMap::from([(3, 1), (2, 2), (1, 3)]);
    let none: Option<&HashMap<i32, i32>> = None;
    assert_eq!(
        [
            map_equals(none, none),
            map_equals(Some(&a), none),
            map_equals(none, Some(&b)),
            map_equals(Some(&a), Some(&a)),
            map_equals(Some(&a), Some(&b)),
            map_equals(Some(&a), Some(&c)),
            map_equals(Some(&a), Some(&d)),
            map_equals(Some(&a), Some(&e)),
        ],
        [true, false, false, true, true, false, true, false]
    );
    let empty_a = HashMap::from([(1, None::<i32>)]);
    let empty_b = HashMap::from([(2, None::<i32>)]);
    assert!(!map_equals(Some(&empty_a), Some(&empty_b)));
}

#[test]
fn binary_search_uses_flutter_midpoints_and_reports_missing_values() {
    let items = [1, 2, 3];
    assert_eq!(
        [1, 2, 3, 12].map(|n| binary_search(&items, &n)),
        [Some(0), Some(1), Some(2), None]
    );
    assert_eq!(binary_search(&[1, 2, 2, 2, 3], &2), Some(2));
    assert_eq!(binary_search(&[], &0), None);
}

fn random_values(size: usize, upper: usize) -> impl Iterator<Item = usize> {
    let mut state: u64 = 0xd9fa_7cff_a34b_543c;
    (0..size).map(move |_| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as usize % upper
    })
}

#[test]
fn merge_sort_orders_random_inputs_including_empty_and_duplicate_values() {
    for size in 0..250 {
        let mut list = random_values(size, size.max(1)).collect::<Vec<_>>();
        let mut control = list.clone();
        control.sort_unstable();
        merge_sort(&mut list);
        assert_eq!(list, control);
    }
}

#[test]
fn merge_sort_preserves_stability_and_never_moves_items_outside_the_subrange() {
    for size in [8, 50, 511, 512, 513] {
        let mut list = random_values(size, size / 4)
            .enumerate()
            .map(|(order, id)| Rc::new((id, order)))
            .collect::<Vec<_>>();
        merge_sort_by(&mut list, 0..size, |a, b| a.0.cmp(&b.0));
        assert!(
            list.windows(2)
                .all(|items| (items[0].0, items[0].1) <= (items[1].0, items[1].1))
        );
        let before = list.clone();
        let first = size / 4;
        let last = size - first;
        merge_sort_by(&mut list, first..last, |a, b| b.0.cmp(&a.0));
        assert!(
            list[first..last]
                .windows(2)
                .all(|w| { w[0].0 > w[1].0 || w[0].0 == w[1].0 && w[0].1 < w[1].1 })
        );
        assert!(
            (0..first)
                .chain(last..size)
                .all(|i| Rc::ptr_eq(&list[i], &before[i]))
        );
    }
}

#[test]
fn merge_sort_special_cases_preserve_every_original_element() {
    for size in [511, 512, 513] {
        for (shape, expected) in [
            (0, (0..size).collect::<Vec<_>>()),
            (1, (1..size).chain([0]).collect()),
            (2, std::iter::once(size - 1).chain(0..size - 1).collect()),
            (3, (0..size).rev().collect()),
        ] {
            let mut list = (0..size)
                .map(|index| {
                    let key = match shape {
                        1 if index == 0 => 1,
                        2 if index == size - 1 => -1,
                        3 => (size - 1 - index) as i32,
                        _ => 0,
                    };
                    (key, index)
                })
                .collect::<Vec<_>>();
            merge_sort_by(&mut list, 0..size, |a, b| a.0.cmp(&b.0));
            assert!(list.windows(2).all(|w| w[0].0 <= w[1].0));
            assert_eq!(
                list.into_iter().map(|(_, i)| i).collect::<Vec<_>>(),
                expected
            );
        }
    }
}
