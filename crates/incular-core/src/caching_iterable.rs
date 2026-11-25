//! A lazy sequence that walks its source at most once.

use std::{cell::RefCell, iter::FusedIterator};

struct Cache<I: Iterator> {
    source: Option<I>,
    items: Vec<I::Item>,
}

impl<I: Iterator> Cache<I> {
    fn fill_next(&mut self) -> bool {
        match self.source.as_mut().and_then(Iterator::next) {
            Some(item) => {
                self.items.push(item);
                true
            }
            None => {
                self.source = None;
                false
            }
        }
    }

    fn fill_to(&mut self, index: usize) -> bool {
        while self.items.len() <= index {
            if !self.fill_next() {
                return false;
            }
        }
        true
    }
}

/// The requested index was negative or beyond the end of the sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachingIndexError {
    pub index: isize,
}

impl std::fmt::Display for CachingIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "caching iterable index {} is out of bounds", self.index)
    }
}

impl std::error::Error for CachingIndexError {}

/// Flutter's lazy caching iterable. Cursors and derived sequences share the
/// source cache; each derived sequence has a separate cache for its own
/// results. Source callbacks must not recursively advance the same sequence.
pub struct CachingIterable<I: Iterator> {
    cache: RefCell<Cache<I>>,
}

impl<I: Iterator> CachingIterable<I> {
    /// Stores the source without evaluating it.
    #[must_use]
    pub const fn new(source: I) -> Self {
        Self {
            cache: RefCell::new(Cache {
                source: Some(source),
                items: Vec::new(),
            }),
        }
    }

    /// Exhausts and caches the sequence on the first call.
    #[must_use]
    pub fn len(&self) -> usize {
        let mut cache = self.cache.borrow_mut();
        while cache.fill_next() {}
        cache.items.len()
    }

    /// Checks only the first item, if it is not already cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.cache.borrow_mut().fill_to(0)
    }
}

impl<I: Iterator> CachingIterable<I>
where
    I::Item: Clone,
{
    /// Reads a zero-based index, filling the cache only through that item.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<I::Item> {
        let mut cache = self.cache.borrow_mut();
        if cache.fill_to(index) {
            cache.items.get(index).cloned()
        } else {
            None
        }
    }

    /// Flutter's signed index contract, expressed as a Rust result.
    ///
    /// # Errors
    /// Returns [`CachingIndexError`] for a negative or missing index.
    pub fn element_at(&self, index: isize) -> Result<I::Item, CachingIndexError> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.get(index))
            .ok_or(CachingIndexError { index })
    }

    #[must_use]
    pub fn first(&self) -> Option<I::Item> {
        self.get(0)
    }

    #[must_use]
    pub fn last(&self) -> Option<I::Item> {
        let mut cache = self.cache.borrow_mut();
        while cache.fill_next() {}
        cache.items.last().cloned()
    }

    pub fn iter(&self) -> CachingCursor<'_, I> {
        self.into_iter()
    }

    pub fn map<'a, T: Clone + 'a>(
        &'a self,
        transform: impl FnMut(I::Item) -> T + 'a,
    ) -> CachingIterable<impl Iterator<Item = T> + 'a> {
        CachingIterable::new(self.iter().map(transform))
    }

    pub fn filter<'a>(
        &'a self,
        predicate: impl FnMut(&I::Item) -> bool + 'a,
    ) -> CachingIterable<impl Iterator<Item = I::Item> + 'a> {
        CachingIterable::new(self.iter().filter(predicate))
    }

    pub fn flat_map<'a, T: IntoIterator + 'a>(
        &'a self,
        transform: impl FnMut(I::Item) -> T + 'a,
    ) -> CachingIterable<impl Iterator<Item = T::Item> + 'a>
    where
        T::Item: Clone,
    {
        CachingIterable::new(self.iter().flat_map(transform))
    }

    pub fn take(&self, count: usize) -> CachingIterable<impl Iterator<Item = I::Item> + '_> {
        CachingIterable::new(self.iter().take(count))
    }

    pub fn skip(&self, count: usize) -> CachingIterable<impl Iterator<Item = I::Item> + '_> {
        CachingIterable::new(self.iter().skip(count))
    }

    pub fn take_while<'a>(
        &'a self,
        predicate: impl FnMut(&I::Item) -> bool + 'a,
    ) -> CachingIterable<impl Iterator<Item = I::Item> + 'a> {
        CachingIterable::new(self.iter().take_while(predicate))
    }

    pub fn skip_while<'a>(
        &'a self,
        predicate: impl FnMut(&I::Item) -> bool + 'a,
    ) -> CachingIterable<impl Iterator<Item = I::Item> + 'a> {
        CachingIterable::new(self.iter().skip_while(predicate))
    }
}

/// An independent position in a shared [`CachingIterable`].
pub struct CachingCursor<'a, I: Iterator> {
    iterable: &'a CachingIterable<I>,
    next: usize,
}

impl<I: Iterator> Iterator for CachingCursor<'_, I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iterable.get(self.next)?;
        self.next += 1;
        Some(item)
    }
}

impl<I: Iterator> FusedIterator for CachingCursor<'_, I> where I::Item: Clone {}

impl<'a, I: Iterator> IntoIterator for &'a CachingIterable<I>
where
    I::Item: Clone,
{
    type Item = I::Item;
    type IntoIter = CachingCursor<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        CachingCursor {
            iterable: self,
            next: 0,
        }
    }
}
