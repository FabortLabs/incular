use incular_core::memory::{CountingAllocator, heap_stats};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::Widget;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn completed_child_scopes_release_their_allocations_during_parent_lifetime() {
    let runtime = Runtime::new(Widget::box_(Size::new(16., 16.), Color::WHITE)).unwrap();
    let spawner = runtime.spawner();
    let active = spawner.scope();
    let cycle = || {
        let scopes: Vec<_> = (0..50).map(|_| spawner.scope()).collect();
        drop(scopes);
    };
    for _ in 0..3 {
        cycle();
    }
    let baseline = heap_stats().unwrap().live_bytes;
    for _ in 0..50 {
        cycle();
    }
    assert!(heap_stats().unwrap().live_bytes <= baseline + 16_384);
    assert!(!active.is_cancelled());
}
