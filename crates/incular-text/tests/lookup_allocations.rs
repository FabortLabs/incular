//! Allocations made by layout requests the engine already retains.
//! Run this test binary alone with --nocapture for the measured numbers.

use incular_text::{TextAlign, TextEngine, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct CountingAllocator;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const LABELS: usize = 2000;

fn allocations_during(pass: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Relaxed);
    pass();
    ALLOCATIONS.load(Relaxed) - before
}

#[test]
fn retained_layouts_are_served_without_allocating() {
    let style = TextStyle::default();
    let mut engine = TextEngine::new();
    let labels: Vec<String> = (0..LABELS).map(|index| format!("Button {index}")).collect();
    // A multi-paragraph text exercises the per-paragraph lookups as well.
    let document = labels[..200].join("\n");
    for label in &labels {
        black_box(engine.layout(label, &style, Some(400.), TextAlign::Start));
    }
    black_box(engine.layout(&document, &style, Some(400.), TextAlign::Start));

    let hits = allocations_during(|| {
        for label in &labels {
            black_box(engine.layout(label, &style, Some(400.), TextAlign::Start));
        }
    });
    // A resize gives every label a new width its single line still fits:
    // served by cross-width reuse, never reshaped.
    let resizes = allocations_during(|| {
        for width in [401., 402., 403.] {
            for label in &labels {
                black_box(engine.layout(label, &style, Some(width), TextAlign::Start));
            }
        }
    });
    let edited = format!("{document}!");
    black_box(engine.layout(&edited, &style, Some(400.), TextAlign::Start));
    let reused_document = allocations_during(|| {
        black_box(engine.layout(&edited, &style, Some(400.), TextAlign::Start));
    });
    println!(
        "LOOKUP_ALLOCATIONS labels={LABELS} hit_pass={hits} resize_passes={resizes} \
         unchanged_document={reused_document}"
    );
    assert_eq!(hits, 0, "cache hits must not allocate");
    assert_eq!(resizes, 0, "cross-width reuse must not allocate");
    assert_eq!(
        reused_document, 0,
        "an unchanged document must not allocate"
    );
}
