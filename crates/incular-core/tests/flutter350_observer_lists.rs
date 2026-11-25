use incular_core::{HashedObserverList, ObserverList};

#[test]
fn observer_list_retains_each_inserted_entry_in_order() {
    let mut list = ObserverList::new();
    for i in 0..10 {
        list.add(i);
    }
    assert!(list.iter().copied().eq(0..10));
    for i in (0..10).rev() {
        assert!(list.remove(&i));
        assert!(list.iter().copied().eq(0..i));
    }
    assert!(list.is_empty());

    for item in [0, 1, 0, 2] {
        list.add(item);
    }
    assert!(list.remove(&0));
    assert!(list.contains(&0));
    assert!(list.iter().copied().eq([1, 0, 2]));
    assert!(!list.remove(&9));
    list.clear();
    assert!(list.is_empty());
}

#[test]
fn hashed_observer_list_counts_duplicates_but_only_iterates_distinct_values() {
    let mut list = HashedObserverList::new();
    for i in 0..10 {
        list.add(i);
    }
    assert!(list.iter().copied().eq(0..10));
    for i in (0..10).rev() {
        assert!(list.remove(&i));
        assert!(list.iter().copied().eq(0..i));
    }

    list.add(0);
    for _ in 0..10 {
        list.add(1);
    }
    list.add(2);
    assert!(list.iter().copied().eq([0, 1, 2]));
    assert!(list.remove(&2));
    assert!(list.iter().copied().eq([0, 1]));
    assert!(list.remove(&1));
    assert!(list.iter().copied().eq([0, 1]));
    assert!(list.remove(&0));
    assert!(list.iter().copied().eq([1]));
    assert!(!list.is_empty());
    for _ in 0..9 {
        assert!(list.remove(&1));
    }
    assert!(list.is_empty());
}

#[test]
fn removing_and_adding_again_goes_to_the_end() {
    let mut list = HashedObserverList::new();
    for item in [0, 1, 2, 1] {
        list.add(item);
    }
    assert!(list.remove(&1));
    assert!(list.contains(&1));
    assert!(list.iter().copied().eq([0, 1, 2]));
    assert!(list.remove(&1));
    assert!(!list.remove(&1));
    list.add(1);
    assert!(list.iter().copied().eq([0, 2, 1]));
    list.clear();
    assert!(!list.contains(&0));
    assert!(list.is_empty());
    list.add(3);
    assert!(list.iter().copied().eq([3]));
}

#[test]
fn removing_across_arena_pages_keeps_survivor_order_and_reuses_slots() {
    let mut list = HashedObserverList::new();
    for item in 0..700 {
        list.add(item);
        if item % 2 == 0 {
            list.add(item);
        }
    }
    assert!(list.iter().copied().eq(0..700));
    for item in (0..700).step_by(2) {
        assert!(list.remove(&item));
        assert!(list.contains(&item));
        assert!(list.remove(&item));
    }
    assert!(list.iter().copied().eq((1..700).step_by(2)));
    for item in 700..800 {
        list.add(item);
    }
    assert!(list.iter().copied().eq((1..700).step_by(2).chain(700..800)));
}
