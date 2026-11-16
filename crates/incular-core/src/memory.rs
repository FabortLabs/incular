//! Process heap accounting for memory diagnostics.
//!
//! [`CountingAllocator`] wraps the system allocator with four atomic
//! counters. An application opts in by installing it as its global
//! allocator; DevTools and the memory log then report the live heap next to
//! GPU and process totals, which separates "the heap grew" from "the
//! renderer holds more textures":
//!
//! ```
//! use incular_core::memory::CountingAllocator;
//!
//! #[global_allocator]
//! static ALLOCATOR: CountingAllocator = CountingAllocator;
//!
//! # fn main() {
//! let text = String::from("counted from the first allocation");
//! let stats = incular_core::memory::heap_stats().expect("allocator installed");
//! assert!(stats.live_bytes >= text.len() as u64);
//! # }
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static TOTAL_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The system allocator plus live-byte accounting. Each allocation costs a
/// few relaxed atomic updates; nothing is recorded per allocation, so there
/// is no bookkeeping memory and no lock.
#[derive(Clone, Copy, Debug, Default)]
pub struct CountingAllocator;

/// Heap totals observed by an installed [`CountingAllocator`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeapStats {
    /// Bytes requested by allocations that are still live. Allocator
    /// headers, alignment padding and free-list fragmentation are not
    /// included, so the process commits somewhat more than this.
    pub live_bytes: u64,
    /// Highest `live_bytes` reached so far.
    pub peak_bytes: u64,
    pub live_allocations: u64,
    /// Allocations made since the process started.
    pub total_allocations: u64,
}

/// Heap totals, or `None` when the application has not installed
/// [`CountingAllocator`] (nothing was ever counted).
#[must_use]
pub fn heap_stats() -> Option<HeapStats> {
    let total_allocations = TOTAL_ALLOCATIONS.load(Relaxed);
    (total_allocations > 0).then(|| HeapStats {
        live_bytes: LIVE_BYTES.load(Relaxed),
        peak_bytes: PEAK_BYTES.load(Relaxed),
        live_allocations: LIVE_ALLOCATIONS.load(Relaxed),
        total_allocations,
    })
}

fn grew(bytes: usize) {
    let live = LIVE_BYTES.fetch_add(bytes as u64, Relaxed) + bytes as u64;
    PEAK_BYTES.fetch_max(live, Relaxed);
}

fn allocated(bytes: usize) {
    grew(bytes);
    LIVE_ALLOCATIONS.fetch_add(1, Relaxed);
    TOTAL_ALLOCATIONS.fetch_add(1, Relaxed);
}

// SAFETY: every method forwards to `System` unchanged; the counters never
// influence the returned pointers or layouts.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Relaxed);
        LIVE_ALLOCATIONS.fetch_sub(1, Relaxed);
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            if new_size >= layout.size() {
                grew(new_size - layout.size());
            } else {
                LIVE_BYTES.fetch_sub((layout.size() - new_size) as u64, Relaxed);
            }
        }
        resized
    }
}
