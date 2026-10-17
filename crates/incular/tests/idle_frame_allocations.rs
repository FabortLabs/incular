//! Idle and resize frame allocation counts for a large scrolling UI. This
//! binary installs a counting allocator.

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

const BUTTONS: usize = 300;

struct Harness {
    runtime: Runtime,
    constraints: Constraints,
    now: Instant,
}

impl Harness {
    fn new(controller: Option<ScrollController>) -> Self {
        let app = Application::new(move |_| {
            let mut view = SingleChildScrollView::new(Column::new((0..BUTTONS).map(|index| {
                Widget::from(
                    incular_controls::Button::new(format!("Button {index}")).on_click(|| {}),
                )
            })));
            if let Some(controller) = &controller {
                view = view.controller(controller.clone());
            }
            Directionality::new(TextDirection::Ltr, view).into()
        })
        .expect("application");
        let mut harness = Self {
            runtime: app.into_runtime(),
            constraints: Constraints::tight(Size::new(800., 600.)),
            now: Instant::now(),
        };
        for _ in 0..4 {
            harness.frame();
        }
        harness
    }

    fn frame(&mut self) {
        self.now += Duration::from_millis(16);
        self.runtime
            .run_frame_at(self.constraints, self.now)
            .expect("frame");
    }

    fn resize(&mut self, width: f32) {
        self.constraints = Constraints::tight(Size::new(width, 600.));
        self.frame();
    }
}

/// Recorded on the code before display lists were shared and the scroll-view
/// and flex scratch vectors were removed (300 buttons, scroll view).
const IDLE_BEFORE: usize = 12;
const RESIZE_BEFORE: usize = 22;

#[test]
fn idle_and_resize_frames_allocate_less() {
    let mut harness = Harness::new(None);
    for _ in 0..3 {
        harness.frame();
    }
    let idle = allocations_during(|| harness.frame());
    harness.resize(810.);
    harness.resize(820.);
    let resize = allocations_during(|| harness.resize(821.));
    println!("idle frame: before {IDLE_BEFORE}, after {idle}");
    println!("1px resize frame: before {RESIZE_BEFORE}, after {resize}");
    assert!(idle < IDLE_BEFORE, "idle frame allocated {idle}");
    assert!(resize < RESIZE_BEFORE, "resize frame allocated {resize}");
}
