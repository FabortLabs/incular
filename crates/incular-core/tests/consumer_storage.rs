use incular_core::BuildContext;
use incular_core::memory::{CountingAllocator, heap_stats};
use incular_core::reactivity::DependencySource;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn one_source_fits_inline_and_larger_sets_deduplicate_and_unsubscribe() {
    let context = BuildContext::new();
    context.run(|| {});
    let sources = std::array::from_fn::<_, 12, _>(|_| DependencySource::default());
    let before = heap_stats().unwrap().total_allocations;
    context.run(|| sources[0].track());
    let calls = heap_stats().unwrap().total_allocations - before;
    assert!(calls <= 4, "{calls} requests recording one source");
    context.build(|_| {
        for _ in 0..2 {
            for source in &sources {
                source.track();
            }
        }
    });
    assert_eq!(context.dependency_count(), sources.len());
    assert!(sources.iter().all(|source| source.subscriber_count() == 1));
    sources[11].notify();
    assert!(context.take_dirty());
    context.clear_dependencies();
    assert!(sources.iter().all(|source| source.subscriber_count() == 0));
}
