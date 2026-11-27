use incular_core::{Offset, Rect, Size, Transform};
use incular_rendering::{Annotation, LayerId, LayerTree, RRect};

fn region(tree: &mut LayerTree, value: i32, size: Option<Size>) -> LayerId {
    tree.create_annotated_region(
        Annotation::new(value),
        size.is_some(),
        size.unwrap_or(Size::ZERO),
    )
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn publish(tree: &mut LayerTree, root: LayerId) {
    tree.set_root(root);
    let _ = tree.flatten();
}

fn assert_integer_results(tree: &LayerTree, point: Offset, expected: &[i32]) {
    assert_eq!(
        tree.find_annotation::<i32>(point),
        expected.first().copied(),
        "find at {point:?}"
    );
    assert_eq!(
        tree.find_annotations::<i32>(point),
        expected,
        "findAllAnnotations at {point:?}"
    );
}

fn make_stack(
    root_transform: Transform,
    mut child: impl FnMut(&mut LayerTree, f32) -> (LayerId, Option<Size>),
) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(root_transform);
    let children = (0..3)
        .map(|index| {
            let (layer, size) = child(&mut tree, index as f32 * 100.);
            let annotation = region(&mut tree, index, size);
            tree.set_children(layer, vec![annotation]);
            layer
        })
        .collect();
    tree.set_children(root, children);
    publish(&mut tree, root);
    tree
}

#[test]
fn offset_layers_use_their_sized_region_at_each_vertical_translation() {
    let tree = make_stack(Transform::IDENTITY, |tree, y| {
        (
            tree.create_transform(Transform::translation(Offset::new(0., y))),
            Some(Size::new(200., 100.)),
        )
    });
    for (y, expected) in [(1., 0), (101., 1), (201., 2)] {
        assert_integer_results(&tree, Offset::new(0., y), &[expected]);
    }
    assert_integer_results(&tree, Offset::new(0., 100.), &[1]);
    assert_integer_results(&tree, Offset::new(200., 1.), &[]);
}

#[test]
fn rectangular_clips_limit_unsized_regions_to_their_stripe() {
    let tree = make_stack(Transform::IDENTITY, |tree, y| {
        (tree.create_clip_rect(rect(0., y, 100., 100.)), None)
    });
    for (y, expected) in [(1., 0), (101., 1), (201., 2)] {
        assert_integer_results(&tree, Offset::new(0., y), &[expected]);
    }
    assert_integer_results(&tree, Offset::new(0., 100.), &[1]);
    assert_integer_results(&tree, Offset::new(100., 1.), &[]);
}

#[test]
fn rounded_clips_limit_unsized_regions_to_their_stripe_and_shape() {
    let tree = make_stack(Transform::IDENTITY, |tree, y| {
        (
            tree.create_clip_rrect(RRect::uniform(rect(0., y, 100., 100.), 4.)),
            None,
        )
    });
    for (y, expected) in [(5., 0), (105., 1), (205., 2)] {
        assert_integer_results(&tree, Offset::new(5., y), &[expected]);
        assert_integer_results(&tree, Offset::new(0., y - 5.), &[]);
    }
}

#[test]
fn transform_and_offset_layers_invert_in_the_right_order() {
    let tree = make_stack(Transform::scale(2.625), |tree, y| {
        (
            tree.create_transform(Transform::translation(Offset::new(0., y))),
            Some(Size::new(100., 100.)),
        )
    });
    for (y, expected) in [(100., 0), (200., 0), (270., 1), (400., 1), (530., 2)] {
        assert_integer_results(&tree, Offset::new(0., y), &[expected]);
    }
}

#[test]
fn different_types_and_unrelated_small_parents_do_not_consume_matches() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let front = region(&mut tree, 1, None);
    let back = tree.create_annotated_region(Annotation::new("hello"), false, Size::ZERO);
    tree.set_children(root, vec![back, front]);
    publish(&mut tree, root);
    assert_eq!(tree.find_annotation::<&str>(Offset::ZERO), Some("hello"));
    assert_eq!(tree.find_annotations::<&str>(Offset::ZERO), ["hello"]);

    let sized = tree.create_annotated_region(Annotation::new("hello"), true, Size::new(10., 10.));
    tree.set_children(root, vec![sized]);
    tree.set_children(sized, vec![front]);
    publish(&mut tree, root);
    let outside = Offset::new(100., 100.);
    assert_integer_results(&tree, outside, &[1]);
    assert!(tree.find_annotations::<&str>(outside).is_empty());
}

#[test]
fn multiple_overlapping_subtrees_return_descendants_then_parents_in_paint_order() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let size = Some(Size::new(100., 100.));
    let back = region(&mut tree, 0, size);
    let front = region(&mut tree, 1, size);
    let back_child = region(&mut tree, 2, size);
    let front_child = region(&mut tree, 3, size);
    tree.set_children(back, vec![back_child]);
    tree.set_children(front, vec![front_child]);
    tree.set_children(root, vec![back, front]);
    publish(&mut tree, root);

    assert_integer_results(&tree, Offset::ZERO, &[3, 1, 2, 0]);
}

#[test]
fn a_singular_root_transform_stays_unhittable_until_the_new_frame() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::scale_non_uniform(0., 1.));
    let child = region(&mut tree, 1, None);
    tree.set_children(root, vec![child]);
    publish(&mut tree, root);

    assert_integer_results(&tree, Offset::ZERO, &[]);
    assert!(tree.update_transform(root, Transform::IDENTITY));
    assert_integer_results(&tree, Offset::ZERO, &[]);
    publish(&mut tree, root);
    assert_integer_results(&tree, Offset::ZERO, &[1]);
}

#[test]
fn reflected_clip_and_region_can_share_an_included_edge() {
    let mut tree = LayerTree::new();
    let reflection = Transform::scale_non_uniform(-1., 1.);
    let root = tree.create_transform(reflection);
    let clip = tree.create_clip_rect(rect(0., 0., 10., 10.));
    let mirror = tree.create_transform(reflection);
    let child = region(&mut tree, 1, Some(Size::new(10., 10.)));
    tree.set_children(root, vec![clip]);
    tree.set_children(clip, vec![mirror]);
    tree.set_children(mirror, vec![child]);
    publish(&mut tree, root);

    assert_integer_results(&tree, Offset::new(0., 5.), &[1]);
    assert_integer_results(&tree, Offset::new(-0.125, 5.), &[]);
    assert_integer_results(&tree, Offset::new(0.125, 5.), &[]);
    assert_integer_results(&tree, Offset::new(0., 10.), &[]);
}

#[test]
fn opposing_clips_can_share_an_included_edge_without_absorbing_neighbors() {
    let mut tree = LayerTree::new();
    let reflection = Transform::scale_non_uniform(-1., 1.);
    let root = tree.create_transform(reflection);
    let outer_clip = tree.create_clip_rect(rect(0., 0., 10., 10.));
    let mirror = tree.create_transform(reflection);
    let inner_clip = tree.create_clip_rect(rect(0., 0., 10., 10.));
    let child = region(&mut tree, 1, None);
    tree.set_children(root, vec![outer_clip]);
    tree.set_children(outer_clip, vec![mirror]);
    tree.set_children(mirror, vec![inner_clip]);
    tree.set_children(inner_clip, vec![child]);
    publish(&mut tree, root);

    assert_integer_results(&tree, Offset::new(0., 5.), &[1]);
    assert_integer_results(&tree, Offset::new(-0.125, 5.), &[]);
    assert_integer_results(&tree, Offset::new(0.125, 5.), &[]);
    assert_integer_results(&tree, Offset::new(0., 10.), &[]);
}

#[test]
fn an_annotated_root_reports_children_before_its_own_value() {
    let mut tree = LayerTree::new();
    let container = tree.create_transform(Transform::IDENTITY);
    let parent = region(&mut tree, 1, None);
    let child = region(&mut tree, 2, None);
    tree.set_children(container, vec![parent]);
    tree.set_children(parent, vec![child]);
    publish(&mut tree, parent);
    assert_integer_results(&tree, Offset::ZERO, &[2, 1]);

    let middle = region(&mut tree, 3, None);
    let front = region(&mut tree, 4, None);
    tree.set_children(parent, vec![child, middle, front]);
    publish(&mut tree, parent);
    assert_integer_results(&tree, Offset::ZERO, &[4, 3, 2, 1]);
}
