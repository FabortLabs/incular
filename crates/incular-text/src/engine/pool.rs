//! Retained layout pools bounded by entry count and by approximate bytes.

use super::{LayoutKey, TextCaretPosition, TextLayout, TextLine};
use incular_rendering::{GlyphPosition, GlyphRun};
use lru::LruCache;
use parley::Layout;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    mem::size_of,
    num::NonZeroUsize,
    sync::Arc,
};

/// Retained layouts kept beyond the largest composed document. A document
/// grows the pool to hold all of its paragraphs plus this many other entries,
/// so a file longer than the base pool cannot evict its own paragraphs before
/// they are reused.
pub(super) const LAYOUT_CACHE_CAPACITY: NonZeroUsize = NonZeroUsize::new(2048).unwrap();

/// Upper bound a pool grows to when its working set keeps missing.
const MAX_RETAINED_LAYOUTS: usize = 1 << 16;

/// Default approximate byte budget shared by an engine's layout pools.
pub(super) const DEFAULT_CACHE_BYTE_BUDGET: usize = 32 << 20;

/// Layout requests in one decay window.
const DECAY_WINDOW: u32 = 1024;

/// Bookkeeping per entry beyond the key and value: the LRU node's links, its
/// hash-map slot, and the evicted-key slot it may later occupy.
const ENTRY_OVERHEAD: usize = 64;

/// Parley keeps clusters, glyphs, runs and styles per source byte; measured
/// at roughly this many bytes for Latin text.
const PARLEY_BYTES_PER_TEXT_BYTE: usize = 72;

/// Approximate heap bytes a retained value keeps alive.
pub(super) trait RetainedBytes {
    fn retained_bytes(&self, key: &LayoutKey) -> usize;
}

impl RetainedBytes for Arc<TextLayout> {
    fn retained_bytes(&self, _key: &LayoutKey) -> usize {
        // Shared allocations (the same layout in two pools, or in a document)
        // are counted by each holder, so the total is an upper bound.
        let layout = &**self;
        let mut bytes = size_of::<TextLayout>()
            + layout.lines.len() * size_of::<TextLine>()
            + layout.font_runs.len() * size_of::<super::FontRunDebug>();
        for line in layout.lines.iter() {
            bytes += line.runs.len() * size_of::<Arc<GlyphRun>>()
                + line.clusters.len() * size_of::<super::TextClusterSpan>();
            for run in line.runs.iter() {
                bytes += size_of::<GlyphRun>() + run.glyphs.len() * size_of::<GlyphPosition>();
            }
            if !line
                .runs
                .first()
                .is_some_and(|run| Arc::ptr_eq(&run.glyphs, &line.glyphs))
            {
                bytes += line.glyphs.len() * size_of::<GlyphPosition>();
            }
        }
        // Caret stops retained at layout time. Stops derived lazily later,
        // for the few layouts a caret visits, are not counted.
        if let Some(carets) = layout.caret_positions.get() {
            bytes += carets.len() * size_of::<TextCaretPosition>();
        }
        bytes
    }
}

impl RetainedBytes for Layout<()> {
    fn retained_bytes(&self, key: &LayoutKey) -> usize {
        size_of::<Self>() + key.text.len() * PARLEY_BYTES_PER_TEXT_BYTE
    }
}

/// One retained value with its accounted size and the decay window that last
/// touched it.
struct Entry<V> {
    value: V,
    bytes: usize,
    window: u64,
}

/// LRU of retained layouts bounded by entry count and by approximate bytes.
///
/// A cyclic pass over more keys than an LRU holds misses on every access,
/// because each entry is evicted just before its reuse: thousands of labels
/// relaid out at a new width, or the paragraphs of a long file. Keys evicted
/// recently are remembered by hash; missing one of them proves the working
/// set exceeded the capacity, so the pool doubles, up to
/// `MAX_RETAINED_LAYOUTS`.
///
/// Growth decays: a window of layout requests that grew nothing, hit at
/// least 95% of lookups and averaged at least two lookups per touched entry
/// shows the working set is far smaller than the pool, so capacity drops
/// back toward the base and the least recently used entries are released.
///
/// Independently, least recently used entries are evicted while the pool's
/// accounted bytes exceed its budget. Entries pinned for a document being
/// composed are exempt, so a long file never evicts its own paragraphs.
pub(super) struct LayoutPool<V = Arc<TextLayout>> {
    entries: LruCache<LayoutKey, Entry<V>>,
    evicted: LruCache<u64, ()>,
    pub(super) growths: u64,
    pub(super) shrinks: u64,
    bytes: usize,
    byte_budget: usize,
    /// Entries exempt from byte eviction and decay while a document is being
    /// composed; cleared after a window without one.
    pinned: usize,
    pinned_this_window: bool,
    window: u64,
    window_requests: u32,
    window_lookups: u32,
    window_hits: u32,
    window_touched: u32,
    grew_this_window: bool,
}

impl<V: RetainedBytes> LayoutPool<V> {
    pub(super) fn new(byte_budget: usize) -> Self {
        Self {
            entries: LruCache::new(LAYOUT_CACHE_CAPACITY),
            evicted: LruCache::new(LAYOUT_CACHE_CAPACITY),
            growths: 0,
            shrinks: 0,
            bytes: 0,
            byte_budget,
            pinned: 0,
            pinned_this_window: false,
            window: 0,
            window_requests: 0,
            window_lookups: 0,
            window_hits: 0,
            window_touched: 0,
            grew_this_window: false,
        }
    }

    pub(super) fn get(&mut self, key: &LayoutKey) -> Option<V>
    where
        V: Clone,
    {
        self.window_lookups = self.window_lookups.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(key) {
            self.window_hits = self.window_hits.saturating_add(1);
            if entry.window != self.window {
                entry.window = self.window;
                self.window_touched = self.window_touched.saturating_add(1);
            }
            return Some(entry.value.clone());
        }
        self.note_miss(key);
        None
    }

    /// Removes and returns the entry, for values used mutably and put back.
    pub(super) fn take(&mut self, key: &LayoutKey) -> Option<V> {
        self.window_lookups = self.window_lookups.saturating_add(1);
        let Some(entry) = self.entries.pop(key) else {
            self.note_miss(key);
            return None;
        };
        self.window_hits = self.window_hits.saturating_add(1);
        if entry.window != self.window {
            self.window_touched = self.window_touched.saturating_add(1);
        }
        self.bytes -= entry.bytes;
        Some(entry.value)
    }

    fn note_miss(&mut self, key: &LayoutKey) {
        if self.evicted.pop(&key_hash(key)).is_some() {
            let doubled = self.entries.cap().get().saturating_mul(2);
            if self.reserve(doubled.min(MAX_RETAINED_LAYOUTS)) {
                self.growths += 1;
                self.grew_this_window = true;
            }
        }
    }

    pub(super) fn put(&mut self, key: LayoutKey, value: V) {
        let bytes = size_of::<LayoutKey>()
            + key.text.capacity()
            + size_of::<Entry<V>>()
            + ENTRY_OVERHEAD
            + value.retained_bytes(&key);
        self.bytes += bytes;
        let entry = Entry {
            value,
            bytes,
            window: self.window,
        };
        if let Some((evicted, entry)) = self.entries.push(key, entry) {
            self.bytes -= entry.bytes;
            // A replaced key is still cached; only a capacity eviction proves
            // the working set did not fit.
            if !self.entries.contains(&evicted) {
                self.evicted.put(key_hash(&evicted), ());
            }
        }
        // Byte evictions are not remembered: missing such a key proves the
        // budget, not the capacity, is too small, and growing cannot help.
        while self.bytes > self.byte_budget && self.entries.len() > self.pinned.max(1) {
            self.pop_lru();
        }
    }

    fn pop_lru(&mut self) {
        if let Some((_, entry)) = self.entries.pop_lru() {
            self.bytes -= entry.bytes;
        }
    }

    /// Grows to at least `capacity` entries; returns whether it grew.
    pub(super) fn reserve(&mut self, capacity: usize) -> bool {
        match NonZeroUsize::new(capacity) {
            Some(capacity) if capacity > self.entries.cap() => {
                self.entries.resize(capacity);
                self.evicted.resize(capacity);
                true
            }
            _ => false,
        }
    }

    /// Keeps `entries` exempt from byte eviction and decay for at least the
    /// next full window.
    pub(super) fn pin(&mut self, entries: usize) {
        self.pinned = self.pinned.max(entries);
        self.pinned_this_window = true;
    }

    /// Counts one layout request toward the decay window, and shrinks the
    /// pool at the end of a window that proved it oversized.
    pub(super) fn tick(&mut self) {
        self.window_requests += 1;
        if self.window_requests < DECAY_WINDOW {
            return;
        }
        let (lookups, hits, touched) = (
            self.window_lookups as usize,
            self.window_hits as usize,
            self.window_touched as usize,
        );
        // Touched entries were reused within the window on average, so it
        // spanned the working set rather than a slice of a longer cycle, and
        // `touched` estimates the working set.
        let oversized = !self.grew_this_window
            && hits * 20 >= lookups * 19
            && touched * 2 <= lookups
            && self.capacity() > LAYOUT_CACHE_CAPACITY.get();
        if oversized {
            let floor = if self.pinned > 0 {
                self.pinned + LAYOUT_CACHE_CAPACITY.get()
            } else {
                LAYOUT_CACHE_CAPACITY.get()
            };
            // Headroom over the observed working set, as growth would leave.
            let target = (touched * 4).max(floor).next_power_of_two();
            if target < self.capacity() {
                self.shrink(target);
            }
        }
        if !self.pinned_this_window {
            self.pinned = 0;
        }
        self.pinned_this_window = false;
        self.grew_this_window = false;
        self.window += 1;
        self.window_requests = 0;
        self.window_lookups = 0;
        self.window_hits = 0;
        self.window_touched = 0;
    }

    /// Lowers capacity and releases the entries beyond it.
    fn shrink(&mut self, capacity: usize) {
        while self.entries.len() > capacity {
            self.pop_lru();
        }
        let capacity = NonZeroUsize::new(capacity).unwrap_or(LAYOUT_CACHE_CAPACITY);
        self.entries.resize(capacity);
        self.evicted.resize(capacity);
        self.shrinks += 1;
    }

    pub(super) fn capacity(&self) -> usize {
        self.entries.cap().get()
    }

    pub(super) const fn bytes(&self) -> usize {
        self.bytes
    }

    pub(super) fn set_byte_budget(&mut self, bytes: usize) {
        self.byte_budget = bytes;
        while self.bytes > self.byte_budget && self.entries.len() > self.pinned.max(1) {
            self.pop_lru();
        }
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.evicted.clear();
        self.bytes = 0;
    }
}

fn key_hash(key: &LayoutKey) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
