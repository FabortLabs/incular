//! `TextStyle::default()` must not allocate: it runs for every `Text::new`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use incular_text::TextStyle;

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: forwards every request unchanged to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` was allocated by `System` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn default_text_style_does_not_allocate() {
    // First construction may initialise shared statics.
    drop(TextStyle::default());
    let before = ALLOCATIONS.with(Cell::get);
    for _ in 0..1000 {
        std::hint::black_box(TextStyle::default());
    }
    let allocations = ALLOCATIONS.with(Cell::get) - before;
    assert_eq!(
        allocations, 0,
        "1000 default styles allocated {allocations} times"
    );
}
