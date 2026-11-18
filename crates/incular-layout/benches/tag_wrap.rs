use criterion::{Criterion, criterion_group, criterion_main};
use incular_layout::{Constraints, Size, Wrap, WrapChild, layout_wrap};
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    for (name, count) in [("tags/empty", 0), ("tags/1000", 1000)] {
        let tags: Vec<_> = (0..count)
            .map(|i| WrapChild::new(Size::new(36. + ((i * 17) % 72) as f32, 24.)))
            .collect();
        let constraints = Constraints::tight(Size::new(480., 8000.));
        c.bench_function(name, |b| {
            b.iter(|| black_box(layout_wrap(constraints, black_box(&tags), Wrap::default())))
        });
    }
}
criterion_group!(benches, bench);
criterion_main!(benches);
