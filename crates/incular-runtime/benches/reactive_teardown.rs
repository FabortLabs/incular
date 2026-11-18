//! Removing owners among unrelated reactive graph nodes. Batched setup and
//! final runtime destruction are outside timing; the measured removal uses
//! the real retained update and frame cleanup path.

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::{Effect, Memo, Runtime, Signal};
use incular_widgets::internal::{ElementId, Key};
use incular_widgets::{Column, Widget};

fn leaf(key: u64) -> Widget {
    Widget::box_(Size::new(1., 1.), Color::WHITE).with_key(Key::Value(key))
}

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100., 100.))
}

struct Removal {
    runtime: Runtime,
    root: ElementId,
    retained: Widget,
    // Keep effect handles alive after builders disappear: disposal is work
    // the runtime must do, independent of application handle lifetimes.
    effects: Vec<Effect>,
}

fn prepared(removed: usize, memos: usize, owned_effects: bool) -> Removal {
    let source = Signal::new(0_u32);
    let retained = leaf(u64::MAX);
    let mut children = vec![retained.clone()];
    children.extend((0..removed).map(|id| leaf(id as u64)));
    let mut runtime = Runtime::new(Column::new(children).into()).expect("runtime");
    let root = runtime.tree().root().expect("root");
    let ids = runtime.tree().children(root).expect("children").to_vec();
    if memos != 0 {
        let values: Vec<_> = (0..memos)
            .map(|_| {
                let source = source.clone();
                Memo::new(move || source.get())
            })
            .collect();
        let retained = retained.clone();
        runtime
            .register_builder(ids[0], move || {
                for value in &values {
                    std::hint::black_box(value.get());
                }
                retained.clone()
            })
            .expect("retained memo consumer");
    }
    let mut effects = Vec::new();
    if owned_effects {
        for (key, &id) in ids[1..].iter().enumerate() {
            let effect = Effect::new({
                let source = source.clone();
                move || {
                    std::hint::black_box(source.get());
                }
            });
            let mounted = effect.clone();
            runtime
                .register_builder(id, move || {
                    assert!(mounted.mount());
                    leaf(key as u64)
                })
                .expect("effect owner");
            effects.push(effect);
        }
    }
    runtime.run_frame(constraints()).expect("warm frame");
    Removal {
        runtime,
        root,
        retained,
        effects,
    }
}

fn remove(state: &mut Removal) {
    state
        .runtime
        .schedule_update(state.root, Column::new([state.retained.clone()]).into())
        .expect("remove subtree");
    std::hint::black_box(
        state
            .runtime
            .run_frame(constraints())
            .expect("removal frame"),
    );
}

fn mount_effects(state: &mut Removal) {
    let source = Signal::new(0_u32);
    let ids = state.runtime.tree().children(state.root).expect("owners")[1..].to_vec();
    for (key, id) in ids.into_iter().enumerate() {
        let effect = Effect::new({
            let source = source.clone();
            move || {
                std::hint::black_box(source.get());
            }
        });
        let mounted = effect.clone();
        state
            .runtime
            .register_builder(id, move || {
                assert!(mounted.mount());
                leaf(key as u64)
            })
            .expect("new effect owner");
        state.effects.push(effect);
    }
    std::hint::black_box(state.runtime.run_frame(constraints()).expect("mount frame"));
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("reactive_teardown");
    for removed in [100_usize, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("effect_mount_control", removed),
            &removed,
            |b, &removed| {
                b.iter_batched_ref(
                    || prepared(removed, 0, false),
                    mount_effects,
                    BatchSize::LargeInput,
                );
            },
        );
        for (name, memos, effects) in [
            ("no_graph_control", 0, false),
            ("unrelated_memos", 1_000, false),
            ("owned_effects", 0, true),
        ] {
            group.bench_with_input(BenchmarkId::new(name, removed), &removed, |b, &removed| {
                b.iter_batched_ref(
                    || prepared(removed, memos, effects),
                    remove,
                    BatchSize::LargeInput,
                );
            });
        }
    }

    // Control: ordinary graph recomputation with no removals. The ownership
    // index is touched on effect mount, not on every dependency subscription.
    group.bench_function("memo_updates_control/100", |b| {
        let source = Signal::new(0_u32);
        let values: Vec<_> = (0..100)
            .map(|_| {
                let source = source.clone();
                Memo::new(move || source.get())
            })
            .collect();
        let mut runtime = Runtime::new(leaf(0)).expect("runtime");
        let root = runtime.tree().root().expect("root");
        runtime
            .register_builder(root, move || {
                for value in &values {
                    std::hint::black_box(value.get());
                }
                leaf(0)
            })
            .expect("memo builder");
        runtime.run_frame(constraints()).expect("warm frame");
        let mut generation = 0_u32;
        b.iter(|| {
            generation = generation.wrapping_add(1);
            source.set(std::hint::black_box(generation));
            std::hint::black_box(runtime.run_frame(constraints()).expect("updated frame"));
        });
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
