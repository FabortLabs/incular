use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use incular_core::{Offset, Rect, Size};
use incular_gestures::reading_order_by;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("focus_reading_order");
    group.bench_function("single_focus_dialog", |b| {
        let button = Rect::from_origin_size(Offset::ZERO, Size::new(80., 32.));
        b.iter_batched(
            || vec![button],
            |fields| black_box(reading_order_by(fields, |rect| *rect)),
            BatchSize::SmallInput,
        )
    });
    // A tab traversal group with four fields per row in reverse registration
    // order. The staggered case overlaps rows of different heights.
    for (name, rows, staggered) in [
        ("form_256_rows", 256, false),
        ("overlapping_32_rows", 32, true),
    ] {
        let fields: Vec<_> = (0..rows)
            .flat_map(|row| {
                (0..4).rev().map(move |col| {
                    let offset = if staggered { col * 5 } else { 0 };
                    Rect::from_origin_size(
                        Offset::new((col * 112) as f32, (row * 32 + offset) as f32),
                        Size::new(100., if staggered { 36. } else { 24. }),
                    )
                })
            })
            .collect();
        group.bench_function(name, |b| {
            b.iter_batched(
                || fields.clone(),
                |fields| black_box(reading_order_by(fields, |rect| *rect)),
                BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
