use std::hash::{BuildHasherDefault, Hash, Hasher};

use incular_core::PersistentHashMap;

#[derive(Clone, Copy)]
struct Key {
    hash: u64,
    payload: u64,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

#[derive(Default)]
struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(byte);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

type WithHashes<V> = PersistentHashMap<Key, V, BuildHasherDefault<IdentityHasher>>;

const fn key(hash: u64, payload: u64) -> Key {
    Key { hash, payload }
}

#[test]
fn fifty_snapshot_versions_each_retain_only_their_own_mappings() {
    let mut maps: Vec<PersistentHashMap<String, usize>> = vec![PersistentHashMap::empty()];
    for i in 0..50 {
        maps.push(maps.last().unwrap().put(format!("key:{i}"), i));
    }
    assert!(maps[0].is_empty());
    for (len, map) in maps.iter().enumerate() {
        assert_eq!(map.len(), len);
        for i in 0..len {
            assert_eq!(map.get(format!("key:{i}").as_str()), Some(&i));
        }
        assert_eq!(map.get(format!("key:{len}").as_str()), None);
    }
}

#[test]
fn exact_hash_collisions_and_shared_prefixes_do_not_erase_each_other() {
    let a = key(1, 1);
    let b = key(1 << 5, 2);
    let c = key(1, 3);
    let d = key(1 | 1 << 5, 4);
    let map: WithHashes<&str> = PersistentHashMap::with_hasher(BuildHasherDefault::default());
    let map = map.put(a, "a").put(b, "b").put(c, "c");
    assert_eq!(map.get(&a), Some(&"a"));
    assert_eq!(map.get(&b), Some(&"b"));
    assert_eq!(map.get(&c), Some(&"c"));
    assert_eq!(map.get(&key(1, 9)), None);
    let map2 = map.put(d, "d");
    assert_eq!(map2.get(&d), Some(&"d"));
    assert_eq!(map.get(&d), None);
    let map3 = map2
        .put(a, "updated(a)")
        .put(b, "updated(b)")
        .put(c, "updated(c)")
        .put(d, "updated(d)");
    assert_eq!(map3.len(), 4);
    for (key, before, after) in [
        (a, "a", "updated(a)"),
        (b, "b", "updated(b)"),
        (c, "c", "updated(c)"),
        (d, "d", "updated(d)"),
    ] {
        assert_eq!(map2.get(&key), Some(&before));
        assert_eq!(map3.get(&key), Some(&after));
    }
}

#[test]
fn all_versions_survive_32_by_32_hash_slot_growth() {
    let empty: WithHashes<u64> = PersistentHashMap::with_hasher(BuildHasherDefault::default());
    let mut maps = vec![empty];
    for i in 0..32 * 32 {
        maps.push(maps.last().unwrap().put(key(i, i), i));
    }
    for (len, map) in (0..=32 * 32).zip(maps) {
        for i in 0..len {
            assert_eq!(map.get(&key(i, i)), Some(&i));
        }
        assert_eq!(map.get(&key(len, len)), None);
    }
}

#[test]
fn final_hash_chunk_differs_safely_and_older_values_do_not_need_cloning() {
    struct NoClone(u8);
    let a = key(0, 0);
    let b = key(1 << 63, 1);
    let c = key(0, 2);
    let empty: WithHashes<NoClone> = PersistentHashMap::with_hasher(BuildHasherDefault::default());
    let first = empty.put(a, NoClone(1));
    let second = first.put(b, NoClone(2)).put(c, NoClone(3));
    assert_eq!(second.get(&a).map(|v| v.0), Some(1));
    assert_eq!(second.get(&b).map(|v| v.0), Some(2));
    assert_eq!(second.get(&c).map(|v| v.0), Some(3));
    assert!(std::ptr::eq(
        first.get(&a).unwrap(),
        second.get(&a).unwrap()
    ));
    let next = second.put(c, NoClone(4));
    assert!(std::ptr::eq(second.get(&b).unwrap(), next.get(&b).unwrap()));
    assert_eq!(next.get(&c).map(|v| v.0), Some(4));
    assert_eq!(second.get(&c).map(|v| v.0), Some(3));
}
