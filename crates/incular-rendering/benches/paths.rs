//! Cost of building path geometry: a shape rebuilt while an equal one is
//! alive (every repaint of unchanged content), a shape nothing owns any more
//! (an animated path), and comparing two recordings of the same picture.
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{CornerRadii, DisplayList, FillRule, PaintCommand, Path};

/// A closed outline of `curves` cubic segments, nudged by `phase` so two
/// phases never share geometry.
fn outline(curves: usize, phase: f32) -> Path {
    let mut builder = Path::builder();
    builder.move_to(Offset::new(phase, 0.));
    for step in 0..curves {
        let x = step as f32 * 3. + phase;
        builder.cubic_to(
            Offset::new(x + 1., 4.),
            Offset::new(x + 2., -4.),
            Offset::new(x + 3., 0.),
        );
    }
    builder.close();
    builder.build()
}

fn rounded(size: f32) -> Path {
    let mut builder = Path::builder();
    builder.rounded_rect(
        Rect::from_origin_size(Offset::ZERO, Size::new(size, size * 0.5)),
        CornerRadii::uniform(6.),
    );
    builder.build()
}

fn superellipse(size: f32) -> Path {
    let mut builder = Path::builder();
    builder.rounded_superellipse(
        Rect::from_origin_size(Offset::ZERO, Size::new(size, size * 0.5)),
        CornerRadii::uniform(10.),
    );
    builder.build()
}

/// One repaint of `shapes` bordered boxes: every shape is built again.
fn record(shapes: usize) -> DisplayList {
    let mut list = DisplayList::new();
    for index in 0..shapes {
        list.push(PaintCommand::FillPath {
            path: Arc::new(rounded(40. + index as f32)),
            brush: Color::WHITE.into(),
            fill_rule: FillRule::NonZero,
        });
    }
    list
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_build");
    for curves in [8_usize, 64, 512] {
        // An equal path is alive, as during the repaint of unchanged content.
        let live = outline(curves, 0.);
        group.bench_with_input(BenchmarkId::new("rebuild_live", curves), &curves, |b, _| {
            b.iter(|| std::hint::black_box(outline(curves, 0.)));
        });
        drop(live);
        // Nothing owns the geometry: each build is new, as for an animated
        // path that changes every frame.
        let mut phase = 0.;
        group.bench_with_input(BenchmarkId::new("build_new", curves), &curves, |b, _| {
            b.iter(|| {
                phase += 1.;
                std::hint::black_box(outline(curves, phase))
            });
        });
    }
    let live = (rounded(48.), superellipse(48.));
    group.bench_function("rebuild_live/rounded_rect", |b| {
        b.iter(|| std::hint::black_box(rounded(48.)));
    });
    group.bench_function("rebuild_live/rounded_superellipse", |b| {
        b.iter(|| std::hint::black_box(superellipse(48.)));
    });
    let icon = outline(24, 0.5);
    let fit = Transform::translation(Offset::new(2., 2.)).then(Transform::scale(0.75));
    let fitted = icon.transformed(fit);
    group.bench_function("transformed_live/icon", |b| {
        b.iter(|| std::hint::black_box(icon.transformed(fit)));
    });
    drop((live, fitted));
    group.finish();

    let mut group = c.benchmark_group("picture_repaint");
    for shapes in [16_usize, 256] {
        let retained = record(shapes);
        // Record the same picture again and compare it with the retained
        // one, as a repaint boundary does before replacing its layer.
        group.bench_with_input(
            BenchmarkId::new("record_and_compare", shapes),
            &shapes,
            |b, _| {
                b.iter(|| {
                    let next = record(shapes);
                    std::hint::black_box(next == retained)
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
