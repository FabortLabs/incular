//! Settled motion sites while no, one, or every sibling changes a live value.
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_animation::MotionValue;
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_widgets::extensions::{AnimateExt, AnimateRepeat};
use incular_widgets::internal::WidgetTree;
use incular_widgets::motion::MotionExt;
use incular_widgets::{Column, Padding, TickerMode, Widget};
use std::time::Duration;

fn prepared(count: usize, depth: usize) -> (WidgetTree, Vec<MotionValue>, Instant) {
    let values: Vec<_> = (0..count).map(|_| MotionValue::new(0.)).collect();
    let root = Column::new(values.iter().map(|value| {
        let mut child = Widget::box_(Size::new(8., 8.), Color::WHITE)
            .motion(|motion| motion.rest(|pose| pose.x(value)));
        for _ in 0..depth {
            child = Padding::all(0., child).into();
        }
        child
    }));
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).expect("mount benchmark tree");
    tree.layout(Constraints::tight(Size::new(100., count as f32 * 8.)))
        .expect("layout");
    let now = Instant::now();
    tree.tick_animations(now);
    tree.update_compositor(now).expect("compositor");
    let _ = tree.paint();
    (tree, values, now)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_motion");
    for count in [100_usize, 1_000] {
        for (shape, depth) in [("flat", 0), ("deep", 32)] {
            for (activity, active_count) in [("idle", 0), ("one_active", 1), ("all_active", count)]
            {
                group.bench_with_input(
                    BenchmarkId::new(format!("{activity}_{shape}"), count),
                    &(count, depth, active_count),
                    |b, &(count, depth, active_count)| {
                        let (mut tree, values, mut now) = prepared(count, depth);
                        let mut offset = 1.;
                        b.iter(|| {
                            // Each active value changes every iteration. A
                            // fixed value would only exercise the idle path
                            // after the first frame.
                            offset = if offset == 1. { 2. } else { 1. };
                            for value in &values[..active_count] {
                                value.set(offset);
                            }
                            now += Duration::from_micros(1);
                            std::hint::black_box(tree.tick_animations(now));
                            std::hint::black_box(tree.update_compositor(now).expect("compositor"));
                        });
                    },
                );
            }
        }

        // Timeline controls exercise delegates with repeatable activity
        // probes, including those that stay pending while their mode is muted.
        for enabled in [true, false] {
            let activity = if enabled {
                "timeline_active"
            } else {
                "timeline_muted"
            };
            group.bench_with_input(BenchmarkId::new(activity, count), &count, |b, &count| {
                let root = Column::new((0..count).map(|_| {
                    Widget::from(
                        Widget::box_(Size::new(8., 8.), Color::WHITE)
                            .animate()
                            .ms(100)
                            .move_x(0., 10.)
                            .repeat(AnimateRepeat::PingPong),
                    )
                }));
                let mut tree = WidgetTree::new();
                tree.mount(TickerMode::new(enabled, root).into())
                    .expect("mount timelines");
                tree.layout(Constraints::tight(Size::new(100., count as f32 * 8.)))
                    .expect("layout timelines");
                let mut now = Instant::now();
                tree.tick_animations(now);
                tree.update_compositor(now).expect("initial compositor");
                b.iter(|| {
                    now += Duration::from_millis(1);
                    std::hint::black_box(tree.tick_animations(now));
                    std::hint::black_box(tree.update_compositor(now).expect("timeline compositor"));
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
