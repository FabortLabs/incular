//! Safe generational storage used by retained framework trees.

use std::fmt;

/// Stable slot identity. A removed slot increments its generation before reuse.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArenaId {
    index: u32,
    generation: u32,
}

impl ArenaId {
    #[must_use]
    pub const fn from_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Slot generation, bumped on removal so reused slots are distinguishable.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Debug for ArenaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArenaId({}, {})", self.index, self.generation)
    }
}

const PAGE_SLOTS: usize = 256;

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Compact safe arena used by persistent framework trees. It never exposes
/// references across mutations, so tree algorithms can remain ordinary
/// `&mut self` code.
pub struct Arena<T> {
    pages: Vec<Vec<Slot<T>>>,
    slot_count: usize,
    free: Vec<u32>,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pages: Vec::new(),
            slot_count: 0,
            free: Vec::new(),
            len: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> ArenaId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.pages[index as usize / PAGE_SLOTS][index as usize % PAGE_SLOTS];
            debug_assert!(slot.value.is_none());
            slot.value = Some(value);
            self.len += 1;
            ArenaId::from_parts(index, slot.generation)
        } else {
            let index = u32::try_from(self.slot_count).expect("arena identity space exhausted");
            if self.slot_count.is_multiple_of(PAGE_SLOTS) {
                // Small arenas grow on demand. Once a page is full it never
                // moves again, even if the directory of pages reallocates.
                let capacity = if self.pages.is_empty() { 8 } else { PAGE_SLOTS };
                self.pages.push(Vec::with_capacity(capacity));
            }
            let page = self.pages.last_mut().expect("allocated an arena page");
            page.push(Slot {
                generation: 0,
                value: Some(value),
            });
            self.slot_count += 1;
            self.len += 1;
            ArenaId::from_parts(index, 0)
        }
    }

    #[must_use]
    pub fn get(&self, id: ArenaId) -> Option<&T> {
        let slot = self
            .pages
            .get(id.index as usize / PAGE_SLOTS)?
            .get(id.index as usize % PAGE_SLOTS)?;
        (slot.generation == id.generation)
            .then_some(slot.value.as_ref())
            .flatten()
    }

    pub fn get_mut(&mut self, id: ArenaId) -> Option<&mut T> {
        let slot = self
            .pages
            .get_mut(id.index as usize / PAGE_SLOTS)?
            .get_mut(id.index as usize % PAGE_SLOTS)?;
        (slot.generation == id.generation)
            .then_some(slot.value.as_mut())
            .flatten()
    }

    pub fn remove(&mut self, id: ArenaId) -> Option<T> {
        let slot = self
            .pages
            .get_mut(id.index as usize / PAGE_SLOTS)?
            .get_mut(id.index as usize % PAGE_SLOTS)?;
        if slot.generation != id.generation {
            return None;
        }
        let value = slot.value.take()?;
        if let Some(next_generation) = slot.generation.checked_add(1) {
            slot.generation = next_generation;
            self.free.push(id.index);
        }
        self.len -= 1;
        Some(value)
    }

    #[must_use]
    pub fn contains(&self, id: ArenaId) -> bool {
        self.get(id).is_some()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (ArenaId, &T)> {
        self.iter_slots(0..self.slot_count)
    }

    /// Visits live slots in an index range, preserving their generations.
    pub fn iter_slots(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = (ArenaId, &T)> {
        let start = range.start.min(self.slot_count);
        let end = range.end.clamp(start, self.slot_count);
        self.pages[start / PAGE_SLOTS..end.div_ceil(PAGE_SLOTS)]
            .iter()
            .enumerate()
            .flat_map(move |(page, slots)| {
                let base = (start / PAGE_SLOTS + page) * PAGE_SLOTS;
                let first = start.saturating_sub(base);
                let last = (end - base).min(slots.len());
                slots[first..last]
                    .iter()
                    .enumerate()
                    .map(move |(i, slot)| (base + first + i, slot))
            })
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|value| {
                    (
                        ArenaId::from_parts(
                            u32::try_from(index).expect("arena index was validated on insertion"),
                            slot.generation,
                        ),
                        value,
                    )
                })
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ArenaId, &mut T)> {
        self.pages
            .iter_mut()
            .flatten()
            .enumerate()
            .filter_map(|(index, slot)| {
                let generation = slot.generation;
                slot.value.as_mut().map(|value| {
                    (
                        ArenaId::from_parts(
                            u32::try_from(index).expect("arena index was validated on insertion"),
                            generation,
                        ),
                        value,
                    )
                })
            })
    }
}
