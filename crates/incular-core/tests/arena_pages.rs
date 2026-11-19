use incular_core::memory::{CountingAllocator, heap_stats};
use incular_core::{Arena, ArenaId};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn large_arena_limits_spare_capacity_and_preserves_cross_page_ids() {
    let before = heap_stats().unwrap().live_bytes;
    let mut arena = Arena::new();
    for i in 0..17_502 {
        arena.insert([i as u64; 16]);
    }
    let live = heap_stats().unwrap().live_bytes - before;
    // A slot holds a generation, the Option tag and a 128-byte value.
    assert!(live < 17_800 * 144 + 8192, "{live} retained bytes");
    let stale = ArenaId::from_parts(255, 0);
    assert_eq!(arena.remove(stale).unwrap()[0], 255);
    let fresh = arena.insert([999; 16]);
    assert_eq!(fresh.index(), stale.index());
    assert!(arena.get(stale).is_none());
    assert_eq!(
        arena
            .iter_slots(254..258)
            .map(|(_, value)| value[0])
            .collect::<Vec<_>>(),
        [254, 999, 256, 257]
    );
    assert_eq!(arena.iter_slots(usize::MAX..usize::MAX).count(), 0);
    assert_eq!(arena.iter_mut().count(), 17_502);
}
