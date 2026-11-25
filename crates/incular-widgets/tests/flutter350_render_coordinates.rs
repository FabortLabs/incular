use incular_config::Constraints;
use incular_core::{Offset, Size, Transform as Affine};
use incular_widgets::{
    Center, SizedBox, Stack, Transform, Widget,
    internal::{Key, TreeError, WidgetTree},
};

const A: Key = Key::Value(182);
const B: Key = Key::Value(183);
const BOUNDS: Size = Size {
    width: 800.,
    height: 600.,
};

fn leaf(key: Key) -> Widget {
    Widget::from(SizedBox::square(25.)).with_key(key)
}

#[test]
fn target_coordinates_reject_a_foreign_tree_even_if_its_arena_slots_match() {
    let mut tree = WidgetTree::new();
    let from = tree.mount(leaf(A)).unwrap();
    let mut foreign = WidgetTree::new();
    let to = foreign.mount(leaf(B)).unwrap();
    assert_eq!(from, to, "fixture exposes the arena alias hazard");
    assert_eq!(
        tree.element_transform_to(from, foreign.element_address(to)),
        Err(TreeError::UnrelatedRenderTrees)
    );
}

#[test]
fn relatives_and_self_share_identity_in_a_tree_without_paint_transforms() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Stack::new([leaf(A), leaf(B)]).into()).unwrap();
    tree.layout(Constraints::tight(BOUNDS)).unwrap();
    let a = tree.element_with_key(&A).unwrap();
    let b = tree.element_with_key(&B).unwrap();
    for (source, target) in [
        (a, b),
        (root, a),
        (root, b),
        (a, root),
        (b, root),
        (root, root),
        (a, a),
        (b, b),
    ] {
        assert_eq!(
            tree.element_transform_to(source, tree.element_address(target)),
            Ok(Affine::IDENTITY)
        );
    }
}

#[test]
fn relative_paint_transform_stops_at_shared_ancestor_and_handles_singular_target() {
    let scale = Affine::scale_non_uniform(9., 4.);
    let a = scale.then(Affine::translation(Offset::new(8., 16.)));
    let b = scale.then(Affine::translation(Offset::new(32., 64.)));
    let origin_transform =
        |transform, child: Widget| Transform::new(transform, child).origin(Offset::ZERO);
    let scene = |second| {
        origin_transform(
            Affine::scale_non_uniform(2., 3.),
            Stack::new([
                Widget::from(origin_transform(a, leaf(A))),
                Widget::from(origin_transform(second, leaf(B))),
            ])
            .into(),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(b)).unwrap();
    tree.layout(Constraints::tight(BOUNDS)).unwrap();
    let from = tree.element_with_key(&A).unwrap();
    let target = tree.element_address(tree.element_with_key(&B).unwrap());
    let relative = tree.element_transform_to(from, target).unwrap();
    let expected = Affine::translation(Offset::new(-216., -192.)).coefficients();
    for (actual, goal) in relative.coefficients().into_iter().zip(expected) {
        assert!((actual - goal).abs() < 1e-9, "{actual} vs {goal}");
    }

    tree.update(root, scene(Affine::scale(0.))).unwrap();
    tree.layout(Constraints::tight(BOUNDS)).unwrap();
    assert_eq!(
        tree.element_transform_to(from, target),
        Ok(Affine::scale(0.))
    );
    let point = tree
        .element_local_to_global(from, Offset::ZERO, target)
        .unwrap();
    assert!(point.x.is_nan() && point.y.is_nan());
}

#[test]
fn local_to_global_with_an_ancestor_excludes_its_outer_centering() {
    let inner = Widget::from(SizedBox::square(50.)).with_key(A);
    let outer = Widget::from(SizedBox::square(100.).child(Center::new(inner))).with_key(B);
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(outer).into()).unwrap();
    tree.layout(Constraints::tight(BOUNDS)).unwrap();
    let source = tree.element_with_key(&A).unwrap();
    let outer = tree.element_with_key(&B).unwrap();
    let relative = tree
        .element_local_to_global(source, Offset::ZERO, tree.element_address(outer))
        .unwrap();
    assert_eq!(relative, Offset::new(25., 25.));
    assert_eq!(
        tree.element_local_to_global(source, Offset::ZERO, None)
            .unwrap(),
        Offset::new(375., 275.)
    );
}
