use std::{
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use incular_core::KeyboardKey;
use incular_gestures::LogicalKeySet;

fn keys(input: &str) -> LogicalKeySet {
    LogicalKeySet::new(
        input
            .chars()
            .map(|ch| KeyboardKey::Character(ch.to_string())),
    )
}

fn hash(value: &LogicalKeySet) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn one_to_four_keys_and_set_construction_preserve_membership() {
    for text in ["a", "ab", "abc", "abcd"] {
        let expected: HashSet<_> = text
            .chars()
            .map(|ch| KeyboardKey::Character(ch.to_string()))
            .collect();
        let set = keys(text);
        assert_eq!(set.keys().iter().cloned().collect::<HashSet<_>>(), expected);
        let from_set = LogicalKeySet::new(expected);
        assert_eq!(from_set, set);
    }
}

#[test]
fn independently_assembled_sets_work_as_the_same_map_key() {
    let four = keys("abcd");
    let reverse = keys("dcba");
    let from_set = LogicalKeySet::new(reverse.keys().iter().cloned().collect::<HashSet<_>>());
    assert_eq!(four, reverse);
    assert_eq!(four, from_set);
    assert_eq!(hash(&four), hash(&reverse));
    assert_eq!(hash(&four), hash(&from_set));

    let mut values = HashMap::from([(keys("a"), "one"), (four, "four")]);
    assert_eq!(values.get(&keys("a")), Some(&"one"));
    assert_eq!(values.get(&reverse), Some(&"four"));
    assert_eq!(values.remove(&from_set), Some("four"));
    assert_eq!(values.len(), 1);
}

#[test]
fn hashing_an_unchanged_key_set_is_stable() {
    for input in ["a", "ab", "abc", "abcd"] {
        let key = keys(input);
        let before = hash(&key);
        assert_eq!(hash(&key), before);
        assert_eq!(hash(&key.clone()), before);
    }
}

#[test]
fn every_permutation_and_a_repeated_key_have_the_same_hash() {
    for (forward, reverse) in [("a", "a"), ("ab", "ba"), ("abc", "cba"), ("abcd", "dcba")] {
        assert_eq!(hash(&keys(forward)), hash(&keys(reverse)));
    }
    assert_eq!(keys("ab"), keys("baaaab"));
    assert_eq!(hash(&keys("ab")), hash(&keys("baaaab")));
}
