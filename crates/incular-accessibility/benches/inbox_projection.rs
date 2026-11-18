use criterion::{Criterion, criterion_group, criterion_main};
use incular_accessibility::*;
use incular_core::{ArenaId, Offset, Rect, Size};

fn node(label: &str, children: Vec<SemanticNodeId>) -> SemanticNode {
    SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role: Role::Button,
        label: Some(label.into()),
        value: None,
        description: None,
        bounds: Rect::from_origin_size(Offset::new(2.0, 3.0), Size::new(20.0, 10.0)),
        state: SemanticState::default(),
        actions: vec![],
        children,
    }
}

fn bench(c: &mut Criterion) {
    let mut tree = SemanticsTree::new();
    let leaves: Vec<_> = (0..10_000)
        .map(|i| tree.insert(node(&format!("message {i}"), vec![])))
        .collect();
    let root = tree.insert(node("inbox", leaves.clone()));
    tree.set_root(Some(root));
    let mut projection = AccessKitProjection::new();
    projection.sync(&tree, 1.).unwrap();
    c.bench_function("inbox/one_changed", |b| {
        b.iter(|| {
            let id = leaves[8500];
            let mut next = tree.node(id).unwrap().clone();
            next.state.selected = !next.state.selected;
            tree.update(id, next);
            std::hint::black_box(projection.sync(&tree, 1.).unwrap());
        })
    });
    c.bench_function("inbox/unchanged", |b| {
        b.iter(|| std::hint::black_box(projection.sync(&tree, 1.)))
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
