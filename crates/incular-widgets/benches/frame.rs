//! Per-frame retained work on a large, otherwise idle tree.
use criterion::{Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Column, Text, Widget};
use std::time::Instant;

fn bench(c: &mut Criterion) {
    let rows: Vec<Widget> = (0..10_000)
        .map(|i| Text::new(format!("a moderately long row label number {i}")).into())
        .collect();
    let mut tree = WidgetTree::default();
    tree.mount(Column::new(rows).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(600., 100_000.)))
        .unwrap();
    let _ = tree.paint();
    c.bench_function("frame/update_compositor_10k", |b| {
        b.iter(|| tree.update_compositor(Instant::now()).unwrap())
    });
    let root = tree.root().unwrap();
    c.bench_function("frame/repaint_10k", |b| {
        b.iter(|| {
            tree.mark_paint(root).unwrap();
            std::hint::black_box(tree.paint())
        })
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
