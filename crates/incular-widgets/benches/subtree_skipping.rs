//! Update-only measurements for immutable-subtree reuse.
//!
//! Mount and initial layout are outside the timed loop. Descriptor creation
//! is included where the case calls for fresh descriptors; subsequent layout,
//! paint, and GPU work are excluded. Every changed-content case alternates its
//! color on every iteration, so it cannot settle into unchanged updates.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ActionSurface, ElementId, Key, WidgetTree};
use incular_widgets::{Column, Padding, SizedBox, Widget};
use std::hint::black_box;

fn leaf(key: u64, generation: u64) -> Widget {
    let color = if generation & 1 == 0 {
        Color::rgba(40, 100, 180, 255)
    } else {
        Color::rgba(180, 100, 40, 255)
    };
    Widget::box_(Size::new(1., 1.), color).with_key(Key::Value(key))
}

fn subtree(items: usize, generation: u64) -> Widget {
    Column::new(
        (0..items)
            .map(|index| leaf(index as u64, generation))
            .collect::<Vec<_>>(),
    )
    .into()
}

fn with_changed_leaf(cached: &Widget, generation: u64) -> Widget {
    Column::new(vec![leaf(u64::MAX, generation), cached.clone()]).into()
}

fn prepared(widget: Widget, items: usize) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount benchmark tree");
    tree.layout(Constraints::tight(Size::new(80., items as f32 * 2. + 20.)))
        .expect("initial layout");
    (tree, root)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("subtree_skipping");
    for items in [1_000_usize, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("reused_immutable_root", items),
            &items,
            |b, &items| {
                let cached = subtree(items, 0);
                let (mut tree, root) = prepared(cached.clone(), items);
                b.iter(|| {
                    tree.update(root, black_box(cached.clone()))
                        .expect("reuse immutable root");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fresh_equal_wrapper_cached_subtree", items),
            &items,
            |b, &items| {
                let cached = subtree(items, 0);
                let (mut tree, root) = prepared(Padding::all(4., cached.clone()).into(), items);
                b.iter(|| {
                    let next = Padding::all(black_box(4.), cached.clone()).into();
                    tree.update(root, next).expect("replace equal wrapper");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("changed_leaf_cached_sibling", items),
            &items,
            |b, &items| {
                let cached = subtree(items, 0);
                let (mut tree, root) = prepared(with_changed_leaf(&cached, 0), items);
                let mut generation = 0_u64;
                b.iter(|| {
                    generation = generation.wrapping_add(1);
                    let next = with_changed_leaf(&cached, black_box(generation));
                    tree.update(root, next).expect("change one leaf");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fresh_whole_tree_all_leaves_changed", items),
            &items,
            |b, &items| {
                let (mut tree, root) = prepared(subtree(items, 0), items);
                let mut generation = 0_u64;
                b.iter(|| {
                    generation = generation.wrapping_add(1);
                    let next = subtree(items, black_box(generation));
                    tree.update(root, next).expect("replace all descriptors");
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("action_ids_one_button", items),
            &items,
            |b, &items| {
                // Callback pruning after a rebuild asks for the live action
                // set; one bound button among `items` plain leaves.
                let button: Widget = ActionSurface::with_child(SizedBox::shrink())
                    .on_press(|| {})
                    .into();
                let tree = Column::new(vec![button, subtree(items, 0)]).into();
                let (tree, _) = prepared(tree, items);
                b.iter(|| black_box(tree.action_ids()));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
