//! Retained bytes per laid-out label.
//! Run this test binary alone with --nocapture for the measured numbers.

use incular_rendering::GlyphPosition;
use incular_text::{TextAlign, TextClusterSpan, TextEngine, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::mem::size_of;
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

const LABELS: usize = 512;

#[test]
fn cluster_spans_cost_no_more_than_glyphs() {
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let _ = engine.layout("font warmup", &style, None, TextAlign::Start);
    // About 65 glyphs each, like a long button or list label.
    let labels: Vec<String> = (0..LABELS)
        .map(|index| format!("{index:04} label text of a typical width for a list row or button"))
        .collect();

    let before = LIVE.load(Relaxed);
    let mut glyphs = 0;
    for label in &labels {
        glyphs += engine
            .layout(label, &style, None, TextAlign::Start)
            .glyph_count();
    }
    let retained = (LIVE.load(Relaxed) - before) as usize;
    black_box(&labels);
    println!(
        "LABEL_MEMORY labels={LABELS} glyphs_per_label={} bytes_per_label={} \
         cluster_span_bytes={} glyph_bytes={}",
        glyphs / LABELS,
        retained / LABELS,
        size_of::<TextClusterSpan>(),
        size_of::<GlyphPosition>()
    );
    assert!(size_of::<TextClusterSpan>() <= size_of::<GlyphPosition>());
}
