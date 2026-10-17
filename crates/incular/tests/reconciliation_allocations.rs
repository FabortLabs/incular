//! Allocation budgets for scrolling frames that mount and validate sliver
//! children. This binary installs a counting allocator.

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

fn allocations_during(f: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);
    f();
    ALLOCATIONS.with(Cell::get) - before
}

struct Harness {
    runtime: Runtime,
    controller: ScrollController,
    now: Instant,
}

impl Harness {
    fn new(rows: usize, in_layout_builder: bool) -> Self {
        let controller = ScrollController::new();
        let list_controller = controller.clone();
        let app = Application::new(move |_| {
            let controller = list_controller.clone();
            let build_list = move || -> Widget {
                ListView::builder(rows, |index| {
                    incular_controls::Button::new(format!("Row {index}")).on_click(|| {})
                })
                .item_extent_builder(|_| 48.)
                .controller(controller.clone())
                .into()
            };
            let body: Widget = if in_layout_builder {
                LayoutBuilder::new(move |_, _| build_list()).into()
            } else {
                build_list()
            };
            Directionality::new(TextDirection::Ltr, body).into()
        })
        .expect("application");
        let mut harness = Self {
            runtime: app.into_runtime(),
            controller,
            now: Instant::now(),
        };
        for _ in 0..6 {
            harness.frame();
        }
        harness
    }

    fn frame(&mut self) {
        self.now += Duration::from_millis(16);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .expect("frame");
    }

    /// Average allocations per frame while scrolling a few pixels per frame.
    fn scroll_allocations(&mut self) -> usize {
        const FRAMES: usize = 60;
        // Warm caches (text layout, pools) before measuring.
        let mut offset = 1000.;
        for _ in 0..30 {
            offset += 50.;
            self.controller.jump_to(offset);
            self.frame();
        }
        let total = allocations_during(|| {
            for _ in 0..FRAMES {
                offset += 50.;
                self.controller.jump_to(offset);
                self.frame();
            }
        });
        total / FRAMES
    }
}

#[test]
fn scrolling_list_builder_allocations_per_frame() {
    let mut harness = Harness::new(10_000, false);
    let per_frame = harness.scroll_allocations();
    println!("ListView.builder(10000): {per_frame} allocations/frame");
    assert!(per_frame < BUDGET_LIST, "{per_frame} allocations per frame");
}

#[test]
fn scrolling_list_builder_in_layout_builder_allocations_per_frame() {
    let mut harness = Harness::new(500, true);
    let per_frame = harness.scroll_allocations();
    println!("LayoutBuilder > ListView.builder(500): {per_frame} allocations/frame");
    assert!(
        per_frame < BUDGET_LAYOUT_BUILDER,
        "{per_frame} allocations per frame"
    );
}

// Measured 763 per frame before pooling validation scratch; 616 after.
const BUDGET_LIST: usize = 680;
const BUDGET_LAYOUT_BUILDER: usize = 680;
