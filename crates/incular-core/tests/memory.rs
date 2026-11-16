//! Heap accounting through an installed `CountingAllocator`.

use incular_core::memory::{CountingAllocator, heap_stats};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const BLOCK: usize = 8 * 1024 * 1024;

// One test: the counters are process-wide, so concurrent tests would see
// each other's allocations.
#[test]
fn live_bytes_follow_allocations_and_the_peak_stays() {
    let before = heap_stats().expect("the test binary installs the allocator");
    assert!(before.live_bytes > 0 && before.live_allocations > 0);
    assert!(before.peak_bytes >= before.live_bytes);

    let block = vec![1_u8; BLOCK];
    let held = heap_stats().expect("allocator installed");
    assert!(
        held.live_bytes >= before.live_bytes + BLOCK as u64 / 2,
        "an {BLOCK} byte block is live: {before:?} -> {held:?}"
    );
    assert!(held.peak_bytes >= held.live_bytes);
    assert!(held.total_allocations > before.total_allocations);

    // Growing in place or moving, the block is counted once at its new size.
    let mut block = block;
    block.resize(2 * BLOCK, 2);
    let grown = heap_stats().expect("allocator installed");
    assert!(grown.live_bytes >= held.live_bytes + BLOCK as u64 / 2);

    drop(block);
    let released = heap_stats().expect("allocator installed");
    assert!(
        released.live_bytes + BLOCK as u64 <= grown.live_bytes,
        "the block's bytes were released: {grown:?} -> {released:?}"
    );
    assert!(
        released.peak_bytes >= grown.live_bytes,
        "the peak remembers the block"
    );
}
