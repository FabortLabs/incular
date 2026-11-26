use incular_core::{Offset, Size, Transform};
use incular_rendering::{Annotation, AnnotationEntry, AnnotationSearchOptions, LayerId, LayerTree};

fn annotated(tree: &mut LayerTree, value: i32, opaque: bool) -> LayerId {
    let layer = tree.create_annotated_region(Annotation::new(value), false, Size::ZERO);
    if opaque {
        assert!(tree.set_annotation_search_options(
            layer,
            AnnotationSearchOptions {
                opaque: true,
                ..Default::default()
            }
        ));
    }
    layer
}

fn flat_stack(opaque_middle: bool) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let children = [
        annotated(&mut tree, 1, false),
        annotated(&mut tree, 2, opaque_middle),
        annotated(&mut tree, 3, false),
    ];
    tree.set_children(root, children.to_vec());
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

fn entry(annotation: i32, local_position: Offset) -> AnnotationEntry<i32> {
    AnnotationEntry {
        annotation,
        local_position,
    }
}

#[test]
fn container_returns_all_matching_children_in_front_to_back_order() {
    let tree = flat_stack(false);
    assert_eq!(
        tree.find_annotation_entries(Offset::ZERO),
        [
            entry(3, Offset::ZERO),
            entry(2, Offset::ZERO),
            entry(1, Offset::ZERO),
        ]
    );
}

#[test]
fn first_annotation_is_the_frontmost_matching_child() {
    assert_eq!(
        flat_stack(false).find_annotation::<i32>(Offset::ZERO),
        Some(3)
    );
}

#[test]
fn all_annotations_of_an_absent_type_is_empty() {
    assert!(
        flat_stack(false)
            .find_annotation_entries::<f64>(Offset::ZERO)
            .is_empty()
    );
}

#[test]
fn first_annotation_of_an_absent_type_is_none() {
    assert_eq!(flat_stack(false).find_annotation::<f64>(Offset::ZERO), None);
}

#[test]
fn opaque_matching_child_stops_the_search_before_covered_siblings() {
    let tree = flat_stack(true);
    assert_eq!(
        tree.find_annotation_entries(Offset::ZERO),
        [entry(3, Offset::ZERO), entry(2, Offset::ZERO),]
    );
}

#[test]
fn opaque_child_still_reports_annotated_ancestors_and_never_blocks_other_types() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let behind = annotated(&mut tree, 1000, false);
    let other_type =
        tree.create_annotated_region(Annotation::new("visible metadata"), false, Size::ZERO);
    let parent = annotated(&mut tree, 1, false);
    let covered = annotated(&mut tree, 10, false);
    let front = annotated(&mut tree, 2, true);
    tree.set_children(parent, vec![covered, front]);
    tree.set_children(root, vec![behind, other_type, parent]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(tree.find_annotations::<i32>(Offset::ZERO), [2, 1]);
    assert_eq!(
        tree.find_annotations::<&str>(Offset::ZERO),
        ["visible metadata"]
    );
    assert_eq!(tree.find_annotation::<i32>(Offset::ZERO), Some(2));
}

#[test]
fn annotation_offset_sets_local_position_without_translating_children_or_including_trailing_edges()
{
    let mut tree = LayerTree::new();
    let translation = tree.create_transform(Transform::translation(Offset::new(50., 10.)));
    let parent = tree.create_annotated_region(Annotation::new(1_i32), true, Size::new(20., 20.));
    let child = annotated(&mut tree, 2, false);
    assert!(tree.set_annotation_search_options(
        parent,
        AnnotationSearchOptions {
            offset: Offset::new(90., 90.),
            opaque: false,
        }
    ));
    tree.set_children(translation, vec![parent]);
    tree.set_children(parent, vec![child]);
    tree.set_root(translation);
    let _ = tree.flatten();

    assert_eq!(
        tree.find_annotation_entries(Offset::new(150., 110.)),
        [
            entry(2, Offset::new(100., 100.)),
            entry(1, Offset::new(10., 10.)),
        ]
    );
    assert_eq!(
        tree.find_annotations::<i32>(Offset::new(159.99, 110.)),
        [2, 1]
    );
    assert_eq!(tree.find_annotations::<i32>(Offset::new(160., 110.)), [2]);
    assert_eq!(tree.find_annotations::<i32>(Offset::new(150., 120.)), [2]);
    assert_eq!(tree.find_annotations::<i32>(Offset::new(60., 20.)), [2]);
}
