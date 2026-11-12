//! Per-frame retained work on a large, otherwise idle tree.
use criterion::{Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, LayerTree, PaintCommand, RRect};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Column, Directionality, Text, TextDirection, Widget};
use std::time::Instant;

fn bench(c: &mut Criterion) {
    let rows: Vec<Widget> = (0..10_000)
        .map(|i| Text::new(format!("a moderately long row label number {i}")).into())
        .collect();
    let mut tree = WidgetTree::default();
    tree.mount(Directionality::new(TextDirection::Ltr, Column::new(rows)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(600., 100_000.)))
        .unwrap();
    let _ = tree.paint();
    c.bench_function("frame/update_compositor_10k", |b| {
        b.iter(|| tree.update_compositor(Instant::now()).unwrap())
    });
    // Nothing is dirty: the cost of handing the retained scene out again.
    c.bench_function("frame/paint_idle_10k", |b| {
        b.iter(|| std::hint::black_box(tree.paint()))
    });
    let root = tree.root().unwrap();
    c.bench_function("frame/repaint_10k", |b| {
        b.iter(|| {
            tree.mark_paint(root).unwrap();
            std::hint::black_box(tree.paint())
        })
    });
}
/// A retained scene of `rows` moved pictures, three commands each.
fn layer_scene(rows: usize) -> (LayerTree, incular_rendering::LayerId) {
    let mut tree = LayerTree::new();
    let mut children = Vec::with_capacity(rows);
    let mut first = None;
    for row in 0..rows {
        let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(600., 20.));
        let mut picture = DisplayList::new();
        picture.push(PaintCommand::Rect {
            rect: bounds,
            color: Color::rgba(240, 240, 240, 255),
        });
        picture.push(PaintCommand::RRect {
            rrect: RRect::uniform(bounds, 4.),
            brush: Color::rgba(20, 90, 200, 255).into(),
        });
        picture.push(PaintCommand::Rect {
            rect: Rect::from_origin_size(Offset::new(8., 8.), Size::new(100., 4.)),
            color: Color::rgba(0, 0, 0, 255),
        });
        let picture = tree.create_picture(picture, bounds);
        let placed =
            tree.create_transform(Transform::translation(Offset::new(0., row as f32 * 20.)));
        tree.set_children(placed, vec![picture]);
        first.get_or_insert(placed);
        children.push(placed);
    }
    let root = tree.create_transform(Transform::IDENTITY);
    tree.set_children(root, children);
    tree.set_root(root);
    (tree, first.unwrap())
}

fn bench_compositor(c: &mut Criterion) {
    let (mut tree, first) = layer_scene(10_000);
    let _ = tree.flatten();
    c.bench_function("compositor/flatten_unchanged_10k", |b| {
        b.iter(|| std::hint::black_box(tree.flatten()))
    });
    // One layer moves each frame: the whole scene is recorded again.
    let mut step = 0_u32;
    c.bench_function("compositor/flatten_one_moved_10k", |b| {
        b.iter(|| {
            step = step.wrapping_add(1);
            tree.update_transform(
                first,
                Transform::translation(Offset::new((step % 7) as f32 + 1., 0.)),
            );
            std::hint::black_box(tree.flatten())
        })
    });
    // The same, while the previous frame's list is still held by a reader.
    let mut held = tree.flatten();
    c.bench_function("compositor/flatten_one_moved_held_10k", |b| {
        b.iter(|| {
            step = step.wrapping_add(1);
            tree.update_transform(
                first,
                Transform::translation(Offset::new((step % 7) as f32 + 1., 0.)),
            );
            held = tree.flatten();
        })
    });
}
criterion_group!(benches, bench, bench_compositor);
criterion_main!(benches);
