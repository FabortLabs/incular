//! Retained memory across edited versions of a long multi-paragraph text.
//! Run this test binary alone with --nocapture for the measured numbers.

use incular_text::{TextAlign, TextEngine, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
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

const PARAGRAPHS: usize = 2000;
const EDITS: usize = 40;

#[test]
fn edited_document_versions_are_not_retained() {
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let _ = engine.layout("font warmup", &style, None, TextAlign::Start);
    let mut lines: Vec<String> = (0..PARAGRAPHS)
        .map(|index| format!("{index:04}: a line of source text long enough to wrap nothing"))
        .collect();

    let before = LIVE.load(Relaxed);
    drop(engine.layout(&lines.join("\n"), &style, None, TextAlign::Start));
    let one_document = LIVE.load(Relaxed) - before;

    // Each keystroke edits one line and lays the whole text out again; no
    // caller keeps the returned layouts.
    let mut edit = |engine: &mut TextEngine, version: usize| {
        lines[version * 997 % PARAGRAPHS].push('x');
        drop(engine.layout(&lines.join("\n"), &style, None, TextAlign::Start));
    };
    // Fill the few compositions retained for prefix carry.
    for version in 0..4 {
        edit(&mut engine, version);
    }
    let warm = LIVE.load(Relaxed);
    for version in 4..EDITS {
        edit(&mut engine, version);
    }
    let growth = LIVE.load(Relaxed) - warm;
    println!(
        "DOCUMENT_VERSIONS paragraphs={PARAGRAPHS} one_document_bytes={one_document} \
         growth_over_{}_edits_bytes={growth}",
        EDITS - 4
    );
    // Only the edited paragraphs' own layouts may accumulate.
    assert!(
        growth < one_document / 8,
        "{} edited versions retained {growth} bytes; one document is {one_document}",
        EDITS - 4
    );
}
