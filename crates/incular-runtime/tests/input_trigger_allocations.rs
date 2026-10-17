//! Pointer input must not format a diagnostic trigger string per event.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use incular_core::{
    Offset,
    input::{InputEvent, PointerPhase},
};
use incular_runtime::Runtime;
use incular_widgets::SizedBox;

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

const EVENTS: usize = 200;

fn move_to(index: usize) -> InputEvent {
    InputEvent::Pointer {
        phase: PointerPhase::Move,
        position: Offset::new(index as f32 * 0.5, 3.0),
    }
}

#[test]
fn pointer_moves_do_not_allocate_a_trigger_label() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).expect("runtime");
    for index in 0..8 {
        let _ = runtime.handle_input(move_to(index));
    }
    let events: Vec<InputEvent> = (0..EVENTS).map(move_to).collect();
    let after = allocations_during(|| {
        for event in events.iter().cloned() {
            let _ = runtime.handle_input(event);
        }
    });

    // Previous behaviour: every event formatted its label into a new String.
    let events: Vec<InputEvent> = (0..EVENTS).map(move_to).collect();
    let before = allocations_during(|| {
        for event in events.iter().cloned() {
            if let InputEvent::Pointer { phase, position } = &event {
                runtime.set_diagnostic_trigger(format!(
                    "pointer({phase:?}, {:.1}, {:.1})",
                    position.x, position.y
                ));
            }
            let _ = runtime.handle_input(event);
        }
    });
    println!("allocations over {EVENTS} moves: lazy={after}, formatted label added={before}");
    assert!(
        before >= after + EVENTS,
        "label formatting should add at least one allocation per event"
    );
    assert_eq!(after, 0, "pointer move allocated {after} times");
}
