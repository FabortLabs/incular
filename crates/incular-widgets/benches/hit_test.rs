//! Pointer hit testing. Every pointer event walks the retained render tree,
//! so its cost is paid at the pointer's report rate, not the frame rate.
use criterion::{Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_widgets::internal::{Sliver, WidgetTree};
use incular_widgets::{
    Column, CustomScrollView, Directionality, PinnedHeaderSliver, SliverList, Text, TextDirection,
    Widget,
};
use std::time::Instant;

const ROW_HEIGHT: f32 = 20.;

/// A scroll view over a list of `rows` boxes, tall enough to materialize all
/// of them, under a pinned header when `pinned` holds.
fn sliver_tree(rows: usize, pinned: bool) -> WidgetTree {
    let mut slivers: Vec<Box<dyn Sliver>> = Vec::new();
    if pinned {
        slivers.push(Box::new(PinnedHeaderSliver::new(Widget::box_(
            Size::new(100., ROW_HEIGHT),
            Color::WHITE,
        ))));
    }
    slivers.push(Box::new(SliverList::builder(rows, |_| {
        Widget::box_(Size::new(100., ROW_HEIGHT), Color::BLACK)
    })));
    let mut tree = WidgetTree::default();
    tree.mount(CustomScrollView::new(slivers).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(
        100.,
        ROW_HEIGHT * (rows as f32 + 2.),
    )))
    .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let _ = tree.paint();
    tree
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("hit_test");
    for rows in [100_usize, 2_000] {
        // The pointer rests on the last row: every sibling before it is
        // rejected by its bounds.
        let point = Offset::new(50., ROW_HEIGHT * rows as f32 - 1.);
        for (name, pinned) in [("sliver_viewport", false), ("pinned_sliver_viewport", true)] {
            let tree = sliver_tree(rows, pinned);
            assert!(tree.hit_test(point).is_some(), "the pointer is over a row");
            group.bench_function(format!("{name}_{rows}/target"), |b| {
                b.iter(|| std::hint::black_box(tree.hit_test(std::hint::black_box(point))))
            });
            group.bench_function(format!("{name}_{rows}/pointer"), |b| {
                b.iter(|| std::hint::black_box(tree.pointer_hit_test(std::hint::black_box(point))))
            });
        }
    }

    let rows: Vec<Widget> = (0..10_000)
        .map(|i| Text::new(format!("row {i}")).into())
        .collect();
    let mut tree = WidgetTree::default();
    tree.mount(Directionality::new(TextDirection::Ltr, Column::new(rows)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(600., 400_000.)))
        .unwrap();
    let _ = tree.paint();
    let point = Offset::new(10., 5.);
    group.bench_function("column_10k/pointer", |b| {
        b.iter(|| std::hint::black_box(tree.pointer_hit_test(std::hint::black_box(point))))
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
