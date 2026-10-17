//! `layout_flex` allocates only the returned child-layout vector.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use incular_layout::{Constraints, Flex, FlexChild, MainAxisSize, Size, layout_flex};

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
fn flex_layout_allocates_only_its_result() {
    // Before the change each call allocated a temporary size vector besides
    // the result vector: two allocations per call.
    const BEFORE: usize = 2;
    let children: Vec<FlexChild> = (0..64)
        .map(|index| {
            if index % 3 == 0 {
                FlexChild::expanded(Size::new(10., 20.), 1)
            } else {
                FlexChild::new(Size::new(30., 10. + index as f32))
            }
        })
        .collect();
    let config = Flex::row().with_main_axis_size(MainAxisSize::Max);
    let constraints = Constraints::tight(Size::new(800., 600.));
    let _ = layout_flex(constraints, &children, config);
    let start = ALLOCATIONS.with(Cell::get);
    let result = layout_flex(constraints, &children, config);
    let after = ALLOCATIONS.with(Cell::get) - start;
    println!("layout_flex allocations: before {BEFORE}, after {after}");
    assert_eq!(result.children.len(), children.len());
    assert!(after < BEFORE, "allocated {after} times");
}
