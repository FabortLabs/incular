//! Retained notification cost on unchanged, compositor-driven and changing
//! trees. All cases use real scroll sources; no-listener layouts are a control.
//! Initial mount/layout/paint stay outside the timed loop. The motion case
//! measures tick + layout + compositor, excluding scene flattening and GPU work.

use std::{hint::black_box, time::Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::{ElementId, Key, NotificationListener, ScrollView, WidgetTree};
use incular_widgets::motion::{MotionExt, MotionValue, ms};
use incular_widgets::{Column, SizedBox, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100., 200.))
}

fn content(items: usize, generation: u64) -> Widget {
    let color = if generation & 1 == 0 {
        Color::WHITE
    } else {
        Color::BLACK
    };
    Column::new(
        (0..items).map(|index| {
            Widget::box_(Size::new(100., 1.), color).with_key(Key::Value(index as u64))
        }),
    )
    .into()
}

fn listener(child: Widget) -> Widget {
    NotificationListener::new(child)
        .on_notification(|_| false)
        .into()
}

fn prepared(root: Widget) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(root).expect("mount benchmark tree");
    tree.layout(constraints()).expect("initial layout");
    tree.update_compositor(Instant::now())
        .expect("initial compositor");
    let _ = tree.paint();
    (tree, root)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("notification_retention");
    for items in [1_000_usize, 10_000] {
        for with_listener in [false, true] {
            let name = if with_listener {
                "cached_layout_listener"
            } else {
                "cached_layout_control"
            };
            group.bench_with_input(BenchmarkId::new(name, items), &items, |b, &items| {
                let source = ScrollView::vertical(ScrollController::new(), content(items, 0));
                let root = if with_listener {
                    listener(source)
                } else {
                    source
                };
                let (mut tree, _) = prepared(root);
                b.iter(|| {
                    tree.layout(black_box(constraints()))
                        .expect("cached layout")
                });
            });
        }

        group.bench_with_input(
            BenchmarkId::new("motion_tick_layout_compositor", items),
            &items,
            |b, &items| {
                let x = MotionValue::new(0.);
                let bound = x.clone();
                let moving = ScrollView::vertical(ScrollController::new(), content(items, 0))
                    .motion(move |m| m.rest(|p| p.x(&bound)));
                let (mut tree, _) = prepared(listener(moving));
                let mut now = Instant::now();
                let mut generation = 0_u64;
                b.iter(|| {
                    generation = generation.wrapping_add(1);
                    x.set(black_box((generation & 1) as f32));
                    now += ms(16);
                    tree.tick_animations(now);
                    tree.layout(constraints()).expect("motion layout");
                    tree.update_compositor(now).expect("motion compositor");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("changing_topology_cached_content", items),
            &items,
            |b, &items| {
                let cached = ScrollView::vertical(ScrollController::new(), content(items, 0));
                let root = |extra| {
                    let mut children = vec![
                        Widget::from(
                            SizedBox::from_size(Size::new(100., 100.)).child(cached.clone()),
                        )
                        .with_key("viewport"),
                    ];
                    if extra {
                        children.push(
                            Widget::box_(Size::new(100., 1.), Color::WHITE).with_key("extra"),
                        );
                    }
                    listener(Column::new(children).into())
                };
                let (mut tree, id) = prepared(root(false));
                let mut extra = false;
                b.iter(|| {
                    extra = !extra;
                    tree.update(id, root(black_box(extra)))
                        .expect("change topology");
                    tree.layout(constraints()).expect("changed topology layout");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fresh_tree_all_leaves_changed", items),
            &items,
            |b, &items| {
                let controller = ScrollController::new();
                let root = |generation| {
                    listener(ScrollView::vertical(
                        controller.clone(),
                        content(items, generation),
                    ))
                };
                let (mut tree, id) = prepared(root(0));
                let mut generation = 0_u64;
                b.iter(|| {
                    generation = generation.wrapping_add(1);
                    tree.update(id, root(black_box(generation)))
                        .expect("replace fresh tree");
                    tree.layout(constraints()).expect("changed content layout");
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
