//! Per-frame allocation budget for a form of many text fields. This binary
//! installs a counting allocator, so it holds only allocation-sensitive tests.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use incular::prelude::*;
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;

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

const FIELDS: usize = 50;

#[test]
fn resize_frame_allocations_stay_bounded_for_many_text_fields() {
    let app = Application::new(move |_| {
        let view = SingleChildScrollView::new(Column::new((0..FIELDS).map(|index| {
            Widget::from(incular_controls::TextField::new(
                TextEditingController::with_text(format!("Field value number {index}")),
            ))
        })));
        Directionality::new(TextDirection::Ltr, view).into()
    })
    .expect("application");
    let mut runtime: Runtime = app.into_runtime();
    let mut now = Instant::now();
    let mut width = 800.;
    let mut frame = |runtime: &mut Runtime| {
        now += Duration::from_millis(16);
        width += 1.;
        runtime
            .run_frame_at(Constraints::tight(Size::new(width, 600.)), now)
            .expect("frame");
    };
    for _ in 0..4 {
        frame(&mut runtime);
    }
    let before = ALLOCATIONS.with(Cell::get);
    const FRAMES: usize = 5;
    for _ in 0..FRAMES {
        frame(&mut runtime);
    }
    let per_frame = (ALLOCATIONS.with(Cell::get) - before) / FRAMES;
    eprintln!("allocations per resize frame: {per_frame}");
    assert!(
        per_frame < FIELDS * BUDGET_PER_FIELD,
        "resize frame allocated {per_frame} times for {FIELDS} text fields"
    );
}

const BUDGET_PER_FIELD: usize = 12;
