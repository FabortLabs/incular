//! Unchanged updates must reuse shared widget descriptors instead of copying
//! them. This binary installs a counting allocator, so it holds one test only.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{Column, Text, Widget};

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
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocations_during(f: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);
    f();
    ALLOCATIONS.with(Cell::get) - before
}

#[test]
fn unchanged_update_does_not_copy_leaf_descriptors() {
    const LEAVES: usize = 1_000;
    let children: Vec<Widget> = (0..LEAVES)
        .map(|index| {
            Widget::from(Text::new(format!("row {index}"))).with_key(Key::Value(index as u64))
        })
        .collect();
    let mut tree = WidgetTree::default();
    let root = tree
        .mount(Column::new(children.clone()).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(600., 6_000.)))
        .expect("layout");

    let next: Widget = Column::new(children).into();
    let allocations = allocations_during(|| tree.update(root, next).expect("update"));

    // Child-list bookkeeping allocates a handful of buffers per update; a
    // per-leaf descriptor copy would allocate at least once per leaf.
    assert!(
        allocations < LEAVES / 4,
        "unchanged update allocated {allocations} times for {LEAVES} leaves"
    );
}
