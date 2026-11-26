# Foundation persistent hash map

The three pinned `foundation/persistent_hash_map_test.dart` registrations
(23, 37, 67) passed. `flutter350_persistent_hash_map` reproduces 50 snapshots,
the four deterministic keys with both a complete hash collision and a shared
five-bit prefix, replacements of all colliding values, and 1,024 sequential
snapshots of a densely populated trie. Earlier snapshots keep their old
values and do not expose later keys.

Incular's bitmap-indexed trie shares untouched branches and copies only the
modified path. The additional native test reaches the highest hash bit,
mixes a full collision with the deep shared prefix, and tests address
identity of unchanged, non-clonable values across versions. Absent keys
return `None`; borrowed keys do not require an allocation. Internally Incular
keeps one compressed node representation and does not depend on Dart's
dense-node specialization or VM identity optimization. No relative timing
benchmark is claimed.
