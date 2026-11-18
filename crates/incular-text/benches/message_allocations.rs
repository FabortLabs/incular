use incular_text::{TextAlign, TextEngine, TextLayoutOptions, TextStyle};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

struct Count;
static CALLS: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forward the supplied pointer and layout to the system allocator.
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        CALLS.fetch_add(1, Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOC: Count = Count;

fn main() {
    let labels: Vec<_> = (0..2500).map(|i| format!("Message {i}")).collect();
    let mut engine = TextEngine::new();
    let options = TextLayoutOptions::new(Some(320.), TextAlign::Start);
    let style = TextStyle::default();
    engine.layout_with_options("Read", &style, options);
    for repeated in [false, true] {
        if repeated {
            engine.layout_with_options("Read", &style, options);
        }
        let before = CALLS.load(Relaxed);
        for label in &labels {
            let text = if repeated { "Read" } else { label };
            std::hint::black_box(engine.layout_with_options(text, &style, options));
        }
        println!(
            "repeated={repeated} allocations={}",
            CALLS.load(Relaxed) - before
        );
    }
}
