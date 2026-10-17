//! Geometry-only frames (resize, scroll) must not rebuild button semantics.
//! This binary installs a counting allocator, so it holds only
//! allocation-sensitive tests.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use incular::prelude::*;
use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticNodeId, SemanticsTree};

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

const BUTTONS: usize = 500;
/// Allocations per frame measured before geometry-only semantics existed.
const BASELINE_PER_FRAME: usize = 3023;

struct Harness {
    runtime: Runtime,
    controller: ScrollController,
    width: f32,
    now: Instant,
}

impl Harness {
    fn new(width: f32) -> Self {
        let controller = ScrollController::new();
        let scroll = controller.clone();
        let app = Application::new(move |_| {
            let view = SingleChildScrollView::new(Column::new((0..BUTTONS).map(|index| {
                Widget::from(
                    incular_controls::Button::new(format!("Button {index}")).on_click(|| {}),
                )
            })))
            .controller(scroll.clone());
            Directionality::new(TextDirection::Ltr, view).into()
        })
        .expect("application");
        let mut harness = Self {
            runtime: app.into_runtime(),
            controller,
            width,
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
            .run_frame_at(Constraints::tight(Size::new(self.width, 600.)), self.now)
            .expect("frame");
    }

    fn step(&mut self, scroll_to: f32) {
        self.width += 1.;
        self.controller.jump_to(scroll_to);
        self.frame();
    }
}

#[derive(Debug, PartialEq)]
struct Dumped {
    role: Role,
    label: Option<String>,
    value: Option<String>,
    description: Option<String>,
    bounds: Rect,
    actions: usize,
    focusable: bool,
    children: Vec<Dumped>,
}

fn dump(tree: &SemanticsTree, id: SemanticNodeId) -> Dumped {
    let node = tree.node(id).expect("live node");
    Dumped {
        role: node.role,
        label: node.label.clone(),
        value: node.value.clone(),
        description: node.description.clone(),
        bounds: node.bounds,
        actions: node.actions.len(),
        focusable: node.state.focusable,
        children: node
            .children
            .iter()
            .map(|&child| dump(tree, child))
            .collect(),
    }
}

fn dump_root(harness: &Harness) -> Dumped {
    let tree = harness.runtime.tree().semantics();
    dump(tree, tree.root().expect("semantic root"))
}

#[test]
fn geometry_only_frames_do_not_rebuild_button_semantics() {
    let mut harness = Harness::new(800.);
    harness.step(3.);
    const FRAMES: usize = 10;
    let total = allocations_during(|| {
        for frame in 0..FRAMES {
            harness.step(6. + 3. * frame as f32);
        }
    });
    let per_frame = total / FRAMES;
    println!("allocations per frame: {per_frame} (baseline {BASELINE_PER_FRAME})");
    assert!(
        per_frame * 5 < BASELINE_PER_FRAME,
        "{per_frame} allocations per frame, baseline {BASELINE_PER_FRAME}"
    );
}

#[test]
fn geometry_only_semantics_match_a_full_collection() {
    let mut harness = Harness::new(800.);
    for frame in 0..12 {
        harness.step(5. * frame as f32);
    }
    let incremental = dump_root(&harness);
    let buttons = harness
        .runtime
        .tree()
        .semantics()
        .iter()
        .filter(|(_, node)| node.role == Role::Button)
        .count();
    assert_eq!(buttons, BUTTONS);

    // A fresh collection of the same tree.
    harness.runtime.tree_mut().set_semantics_enabled(false);
    harness.runtime.tree_mut().set_semantics_enabled(true);
    harness.frame();
    assert_eq!(incremental, dump_root(&harness));

    // A second application built directly at the final size and offset.
    let mut fresh = Harness::new(harness.width);
    fresh.controller.jump_to(harness.controller.offset());
    fresh.frame();
    fresh.frame();
    assert_eq!(incremental, dump_root(&fresh));
}
