//! Focused allocation census for a populated text-layout cache.
//! Run this test binary alone with --nocapture; the output is diagnostic evidence.

use incular_text::{TextAlign, TextEngine, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};

struct CountingAllocator;
static LIVE: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as isize, Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size() as isize, Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size() as isize, Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, new_size) };
        if !result.is_null() {
            LIVE.fetch_add(new_size as isize - layout.size() as isize, Relaxed);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn retained_cache_allocation_census() {
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let _ = engine.layout("font warmup", &style, None, TextAlign::Start);
    let texts: Vec<String> = (0..512)
        .map(|index| format!("{index:04} unique label with enough characters to expose cloned key capacity and retained text allocation"))
        .collect();

    let before = LIVE.load(Relaxed);
    for text in &texts {
        black_box(engine.layout(text, &style, None, TextAlign::Start));
    }
    let after = LIVE.load(Relaxed);
    let cold_misses = engine.diagnostics().cache_misses;
    for text in &texts {
        black_box(engine.layout(text, &style, None, TextAlign::Start));
    }
    assert_eq!(engine.diagnostics().cache_misses, cold_misses);
    assert_eq!(engine.diagnostics().cache_hits, 512);
    println!(
        "CACHE_KEY_CENSUS entries=512 retained_delta_bytes={} hits=512",
        after - before
    );
}
