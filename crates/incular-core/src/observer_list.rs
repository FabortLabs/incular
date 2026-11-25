//! Foundation's two observer lists choose different duplicate semantics.

use std::{collections::HashMap, hash::Hash, rc::Rc};

use crate::{Arena, ArenaId};

/// A small, ordered observer collection. Iteration includes each
/// registration, and removing a value removes its first registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverList<T> {
    entries: Vec<T>,
}

impl<T> Default for ObserverList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ObserverList<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, observer: T) {
        self.entries.push(observer);
    }

    pub fn remove(&mut self, observer: &T) -> bool
    where
        T: PartialEq,
    {
        let Some(index) = self.entries.iter().position(|entry| entry == observer) else {
            return false;
        };
        self.entries.remove(index);
        true
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[must_use]
    pub fn contains(&self, observer: &T) -> bool
    where
        T: PartialEq,
    {
        self.entries.contains(observer)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.entries.iter()
    }
}

struct Node<T> {
    observer: Rc<T>,
    count: usize,
    prev: Option<ArenaId>,
    next: Option<ArenaId>,
}

/// An observer collection optimized for many registrations. Iteration
/// yields each distinct value once, in first-registration order. It takes
/// one removal per registration to remove a value from iteration.
pub struct HashedObserverList<T> {
    index: HashMap<Rc<T>, ArenaId>,
    nodes: Arena<Node<T>>,
    first: Option<ArenaId>,
    last: Option<ArenaId>,
}

impl<T> Default for HashedObserverList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HashedObserverList<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            nodes: Arena::new(),
            first: None,
            last: None,
        }
    }

    pub fn add(&mut self, observer: T)
    where
        T: Eq + Hash,
    {
        if let Some(&id) = self.index.get(&observer) {
            let node = self.nodes.get_mut(id).expect("indexed observer is live");
            node.count = node.count.checked_add(1).expect("observer count overflow");
            return;
        }
        let observer = Rc::new(observer);
        let id = self.nodes.insert(Node {
            observer: observer.clone(),
            count: 1,
            prev: self.last,
            next: None,
        });
        if let Some(last) = self.last {
            self.nodes
                .get_mut(last)
                .expect("last observer is live")
                .next = Some(id);
        } else {
            self.first = Some(id);
        }
        self.last = Some(id);
        self.index.insert(observer, id);
    }

    pub fn remove(&mut self, observer: &T) -> bool
    where
        T: Eq + Hash,
    {
        let Some(&id) = self.index.get(observer) else {
            return false;
        };
        let node = self.nodes.get_mut(id).expect("indexed observer is live");
        if node.count > 1 {
            node.count -= 1;
            return true;
        }
        self.index.remove(observer);
        let node = self.nodes.remove(id).expect("indexed observer is live");
        if let Some(prev) = node.prev {
            self.nodes
                .get_mut(prev)
                .expect("previous observer is live")
                .next = node.next;
        } else {
            self.first = node.next;
        }
        if let Some(next) = node.next {
            self.nodes
                .get_mut(next)
                .expect("next observer is live")
                .prev = node.prev;
        } else {
            self.last = node.prev;
        }
        true
    }

    pub fn clear(&mut self) {
        self.index.clear();
        self.nodes = Arena::new();
        self.first = None;
        self.last = None;
    }

    #[must_use]
    pub fn contains(&self, observer: &T) -> bool
    where
        T: Eq + Hash,
    {
        self.index.contains_key(observer)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.first.is_none()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::successors(self.first, |id| {
            self.nodes.get(*id).expect("ordered observer is live").next
        })
        .map(|id| {
            self.nodes
                .get(id)
                .expect("ordered observer is live")
                .observer
                .as_ref()
        })
    }
}
