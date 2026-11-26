//! Immutable hash-array mapped trie for cheaply deriving map snapshots.

use std::{
    borrow::Borrow,
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash},
    sync::Arc,
};

const BITS: u32 = 5;
const MASK: u64 = (1 << BITS) - 1;

type SharedNode<K, V> = Arc<Node<K, V>>;

enum Node<K, V> {
    Leaf {
        hash: u64,
        key: K,
        value: V,
    },
    Collision {
        hash: u64,
        // Each child is a leaf with the same complete hash.
        leaves: Box<[SharedNode<K, V>]>,
    },
    Branch {
        bitmap: u32,
        // Only occupied slots are stored, ordered by their bitmap bit.
        children: Box<[SharedNode<K, V>]>,
    },
}

/// An immutable key/value map. Inserting into a snapshot copies only the
/// hash trie path to the modified key, leaving old snapshots valid.
/// Neither keys nor values have to implement `Clone`.
pub struct PersistentHashMap<K, V, S = RandomState> {
    root: Option<SharedNode<K, V>>,
    len: usize,
    hasher: Arc<S>,
}

impl<K, V, S> Clone for PersistentHashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            len: self.len,
            hasher: Arc::clone(&self.hasher),
        }
    }
}

impl<K, V> PersistentHashMap<K, V> {
    #[must_use]
    pub fn empty() -> Self {
        Self::with_hasher(RandomState::new())
    }
}

impl<K, V> Default for PersistentHashMap<K, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<K, V, S> PersistentHashMap<K, V, S> {
    /// Sets the hash builder for this map and its future snapshots.
    #[must_use]
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            root: None,
            len: 0,
            hasher: Arc::new(hasher),
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
}

impl<K: Eq + Hash, V, S: BuildHasher> PersistentHashMap<K, V, S> {
    /// Returns a new version with this mapping, leaving `self` unchanged.
    #[must_use]
    pub fn put(&self, key: K, value: V) -> Self {
        let hash = self.hasher.hash_one(&key);
        let (root, inserted) = if let Some(root) = &self.root {
            insert(root, hash, key, value, 0)
        } else {
            (Arc::new(Node::Leaf { hash, key, value }), true)
        };
        Self {
            root: Some(root),
            len: self.len + usize::from(inserted),
            hasher: Arc::clone(&self.hasher),
        }
    }

    /// Returns the value associated with `key`. A borrowed form of an
    /// owned key is accepted, e.g. `&str` for a `String` key.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let mut node = self.root.as_deref()?;
        let mut shift = 0;
        let hash = self.hasher.hash_one(key);
        loop {
            match node {
                Node::Leaf { .. } => return leaf_value(node, hash, key),
                Node::Collision {
                    hash: stored,
                    leaves,
                } => {
                    if *stored != hash {
                        return None;
                    }
                    return leaves.iter().find_map(|leaf| leaf_value(leaf, hash, key));
                }
                Node::Branch { bitmap, children } => {
                    let bit = hash_bit(hash, shift);
                    if bitmap & bit == 0 {
                        return None;
                    }
                    node = &children[rank(*bitmap, bit)];
                    shift += BITS;
                }
            }
        }
    }
}

fn leaf_value<'a, K: Borrow<Q>, V, Q: Eq + ?Sized>(
    node: &'a Node<K, V>,
    hash: u64,
    key: &Q,
) -> Option<&'a V> {
    match node {
        Node::Leaf {
            hash: stored,
            key: stored_key,
            value,
        } if hash == *stored && key == stored_key.borrow() => Some(value),
        _ => None,
    }
}

fn hash_bit(hash: u64, shift: u32) -> u32 {
    1 << ((hash >> shift) & MASK)
}

fn rank(bitmap: u32, bit: u32) -> usize {
    (bitmap & (bit - 1)).count_ones() as usize
}

fn insert<K: Eq, V>(
    node: &SharedNode<K, V>,
    hash: u64,
    key: K,
    value: V,
    shift: u32,
) -> (SharedNode<K, V>, bool) {
    match &**node {
        Node::Leaf {
            hash: stored,
            key: stored_key,
            ..
        } => {
            let same_key = hash == *stored && key == *stored_key;
            let leaf = Arc::new(Node::Leaf { hash, key, value });
            if same_key {
                return (leaf, false);
            }
            if hash == *stored {
                return (
                    Arc::new(Node::Collision {
                        hash,
                        leaves: Box::new([Arc::clone(node), leaf]),
                    }),
                    true,
                );
            }
            (join(*stored, Arc::clone(node), hash, leaf, shift), true)
        }
        Node::Collision {
            hash: stored,
            leaves,
        } => {
            if *stored != hash {
                let leaf = Arc::new(Node::Leaf { hash, key, value });
                return (join(*stored, Arc::clone(node), hash, leaf, shift), true);
            }
            let existing = leaves
                .iter()
                .position(|leaf| leaf_value(leaf, hash, &key).is_some());
            let leaf = Arc::new(Node::Leaf { hash, key, value });
            let entries = with_child(
                leaves,
                existing.unwrap_or(leaves.len()),
                leaf,
                existing.is_some(),
            );
            (
                Arc::new(Node::Collision {
                    hash,
                    leaves: entries,
                }),
                existing.is_none(),
            )
        }
        Node::Branch { bitmap, children } => {
            let bit = hash_bit(hash, shift);
            let index = rank(*bitmap, bit);
            let existing = bitmap & bit != 0;
            let (child, inserted) = if existing {
                insert(&children[index], hash, key, value, shift + BITS)
            } else {
                (Arc::new(Node::Leaf { hash, key, value }), true)
            };
            let children = with_child(children, index, child, existing);
            (
                Arc::new(Node::Branch {
                    bitmap: bitmap | bit,
                    children,
                }),
                inserted,
            )
        }
    }
}

fn with_child<K, V>(
    children: &[SharedNode<K, V>],
    index: usize,
    child: SharedNode<K, V>,
    replaced: bool,
) -> Box<[SharedNode<K, V>]> {
    let mut result = Vec::with_capacity(children.len() + usize::from(!replaced));
    result.extend_from_slice(&children[..index]);
    result.push(child);
    result.extend_from_slice(&children[index + usize::from(replaced)..]);
    result.into_boxed_slice()
}

// Separate different full hashes at the first index where they diverge.
fn join<K, V>(
    a_hash: u64,
    a: SharedNode<K, V>,
    b_hash: u64,
    b: SharedNode<K, V>,
    shift: u32,
) -> SharedNode<K, V> {
    debug_assert_ne!(a_hash, b_hash);
    let a_bit = hash_bit(a_hash, shift);
    let b_bit = hash_bit(b_hash, shift);
    let children: Box<[SharedNode<K, V>]> = if a_bit == b_bit {
        Box::new([join(a_hash, a, b_hash, b, shift + BITS)])
    } else if a_bit < b_bit {
        Box::new([a, b])
    } else {
        Box::new([b, a])
    };
    Arc::new(Node::Branch {
        bitmap: a_bit | b_bit,
        children,
    })
}
