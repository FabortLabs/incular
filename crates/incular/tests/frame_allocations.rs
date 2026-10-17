//! Per-frame allocation budgets for a large retained UI. This binary installs
//! a counting allocator, so it holds only allocation-sensitive tests.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use incular::prelude::*;
use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::Role;

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

const BUTTONS: usize = 400;

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

    /// Button labels and bounds in arena order.
    fn buttons(&self) -> Vec<(Option<String>, Rect)> {
        self.runtime
            .tree()
            .semantics()
            .iter()
            .filter(|(_, node)| node.role == Role::Button)
            .map(|(_, node)| (node.label.clone(), node.bounds))
            .collect()
    }

    /// Releases the retained semantic graph and collects it from scratch.
    fn recollect(&mut self) -> Vec<(Option<String>, Rect)> {
        self.runtime.tree_mut().set_semantics_enabled(false);
        self.runtime.tree_mut().set_semantics_enabled(true);
        self.frame();
        let mut buttons = self.buttons();
        buttons.sort_by(|a, b| a.0.cmp(&b.0));
        buttons
    }
}

#[test]
fn idle_frame_with_semantics_does_no_semantic_work() {
    let mut harness = Harness::new(None);
    let revision = harness.runtime.tree().semantics().revision();
    let allocations = allocations_during(|| harness.frame());
    assert_eq!(harness.runtime.tree().semantics().revision(), revision);
    // Collecting the tree again would allocate several times per button.
    assert!(
        allocations < 64,
        "idle frame allocated {allocations} times for {BUTTONS} buttons"
    );
}

#[test]
fn resize_frame_allocations_do_not_scale_with_layout_nodes() {
    let mut harness = Harness::new(None);
    harness.runtime.tree_mut().set_semantics_enabled(false);
    harness.frame();
    harness.resize(810.);
    let allocations = allocations_during(|| harness.resize(830.));
    // Each button is about a dozen render objects. Copied child lists,
    // offset vectors and re-recorded pictures cost about 30 allocations per
    // button; what remains is text layout lookups.
    assert!(
        allocations < BUTTONS * 6,
        "resize frame allocated {allocations} times for {BUTTONS} buttons"
    );
}

#[test]
fn incremental_semantics_match_a_full_collection() {
    let controller = ScrollController::new();
    let mut harness = Harness::new(Some(controller.clone()));
    harness.resize(700.);
    controller.jump_to(240.);
    harness.frame();
    harness.resize(760.);
    let mut incremental = harness.buttons();
    incremental.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(incremental.len(), BUTTONS);
    assert_eq!(incremental, harness.recollect());
}
