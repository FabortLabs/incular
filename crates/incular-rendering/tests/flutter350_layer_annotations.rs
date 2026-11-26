use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{
    Annotation, AnnotationEntry, AnnotationSearchOptions, Canvas, FillRule, FollowerLayer,
    LayerAnchor, LayerId, LayerLink, LayerTree, Path, RRect,
};
use std::sync::Arc;

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

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn clip_stack(make_clip: impl FnOnce(&mut LayerTree) -> LayerId) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let clip = make_clip(&mut tree);
    // Flutter's test-only annotated layer uses its own size and offset for
    // containment but reports the unadjusted point. A covering native layer
    // keeps those positions while isolating the requested clip.
    let foreground =
        tree.create_annotated_region(Annotation::new(1_i32), true, Size::new(20., 20.));
    assert!(tree.set_annotation_search_options(
        foreground,
        AnnotationSearchOptions {
            opaque: true,
            ..Default::default()
        }
    ));
    tree.set_children(root, vec![background, clip]);
    tree.set_children(clip, vec![foreground]);
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

#[test]
fn offset_layer_hits_use_its_child_local_coordinates() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let shifted = tree.create_transform(Transform::translation(Offset::new(-10., 0.)));
    let foreground =
        tree.create_annotated_region(Annotation::new(1_i32), true, Size::new(10., 10.));
    let _ = tree.set_annotation_search_options(
        foreground,
        AnnotationSearchOptions {
            opaque: true,
            ..Default::default()
        },
    );
    tree.set_children(root, vec![background, shifted]);
    tree.set_children(shifted, vec![foreground]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(
        tree.find_annotation_entries(Offset::new(-5., 5.)),
        [entry(1, Offset::new(5., 5.))]
    );
    assert_eq!(
        tree.find_annotation_entries(Offset::new(5., 5.)),
        [entry(1000, Offset::new(5., 5.))]
    );
}

#[test]
fn rectangular_clips_exclude_points_outside_and_on_the_trailing_edge() {
    let tree = clip_stack(|tree| tree.create_clip_rect(rect(10., 10., 5., 5.)));
    for (position, expected) in [
        (Offset::new(11., 11.), 1),
        (Offset::new(19., 19.), 1000),
        (Offset::new(15., 12.), 1000),
        (Offset::new(12., 15.), 1000),
        (Offset::new(10., 10.), 1),
    ] {
        assert_eq!(
            tree.find_annotation_entries(position),
            [entry(expected, position)]
        );
    }
}

#[test]
fn rounded_clip_does_not_let_a_corner_hide_background_metadata() {
    let tree =
        clip_stack(|tree| tree.create_clip_rrect(RRect::uniform(rect(10., 10., 10., 10.), 4.)));
    for (position, expected) in [(Offset::new(12., 12.), 1), (Offset::new(11., 11.), 1000)] {
        assert_eq!(
            tree.find_annotation_entries(position),
            [entry(expected, position)]
        );
    }
}

#[test]
fn triangular_clip_uses_the_path_and_keeps_its_diagonal_boundary() {
    let mut path = Path::builder();
    path.move_to(Offset::new(10., 10.));
    path.line_to(Offset::new(12., 10.));
    path.line_to(Offset::new(10., 12.));
    path.close();
    let tree = clip_stack(|tree| tree.create_clip_path(Arc::new(path.build()), FillRule::NonZero));
    for (position, expected) in [
        (Offset::new(11., 11.), 1),
        (Offset::new(12., 12.), 1000),
        (Offset::new(11.5, 11.5), 1000),
        (Offset::new(10.5, 10.5), 1),
    ] {
        assert_eq!(
            tree.find_annotation_entries(position),
            [entry(expected, position)]
        );
    }
}

#[test]
fn singular_transform_does_not_capture_annotations() {
    let tree = clip_stack(|tree| tree.create_transform(Transform::scale_non_uniform(1., 0.)));
    assert_eq!(
        tree.find_annotation_entries(Offset::ZERO),
        [entry(1000, Offset::ZERO)]
    );
}

#[test]
fn an_exact_polygon_edge_is_a_hit_but_a_retraced_ray_does_not_become_fill() {
    let mut path = Path::builder();
    path.move_to(Offset::ZERO);
    path.line_to(Offset::new(2., 0.));
    path.line_to(Offset::new(3., 1.));
    path.line_to(Offset::new(2., 0.));
    path.line_to(Offset::new(0., 2.));
    path.close();
    let polygon = path.build();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        assert!(polygon.contains(Offset::new(1., 1.), rule));
        assert!(!polygon.contains(Offset::new(2.5, 0.5), rule));
        assert!(!polygon.contains(Offset::new(1., 1.01), rule));
    }
}

fn coverage_tree(front_opaque: bool, empty: bool) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let foreground = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let child = if empty {
        tree.create_annotated_region(Annotation::new(2_i32), true, Size::ZERO)
    } else {
        annotated(&mut tree, 2, front_opaque)
    };
    tree.set_children(root, vec![background, foreground]);
    tree.set_children(foreground, vec![child]);
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

#[test]
fn container_propagates_an_opaque_child_hit_across_its_own_siblings() {
    assert_eq!(
        coverage_tree(true, false).find_annotation_entries(Offset::ZERO),
        [entry(2, Offset::ZERO)]
    );
}

#[test]
fn container_passes_through_when_its_child_is_transparent() {
    assert_eq!(
        coverage_tree(false, false).find_annotation_entries(Offset::ZERO),
        [entry(2, Offset::ZERO), entry(1000, Offset::ZERO)]
    );
}

#[test]
fn container_does_not_absorb_when_its_child_misses() {
    assert_eq!(
        coverage_tree(false, true).find_annotation_entries(Offset::ZERO),
        [entry(1000, Offset::ZERO)]
    );
}

fn active_parent(opaque: bool) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let parent = annotated(&mut tree, 1, opaque);
    let child = annotated(&mut tree, 2, false);
    tree.set_children(root, vec![background, parent]);
    tree.set_children(parent, vec![child]);
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

#[test]
fn transparent_annotated_parent_appends_its_value_then_continues_to_its_background() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        active_parent(false).find_annotation_entries(point),
        [entry(2, point), entry(1, point), entry(1000, point)]
    );
}

#[test]
fn opaque_annotated_parent_appends_its_value_then_stops_at_its_background() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        active_parent(true).find_annotation_entries(point),
        [entry(2, point), entry(1, point)]
    );
}

#[test]
fn annotated_regions_are_transparent_by_default() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        active_parent(false).find_annotation_entries(point),
        [entry(2, point), entry(1, point), entry(1000, point)]
    );
}

fn missed_parent(parent_opaque: bool, child_opaque: bool) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let parent = tree.create_annotated_region(Annotation::new(1_i32), true, Size::ZERO);
    assert_eq!(
        tree.set_annotation_search_options(
            parent,
            AnnotationSearchOptions {
                opaque: parent_opaque,
                ..Default::default()
            }
        ),
        parent_opaque
    );
    let child = annotated(&mut tree, 2, child_opaque);
    tree.set_children(root, vec![background, parent]);
    tree.set_children(parent, vec![child]);
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

#[test]
fn a_missed_opaque_parent_does_not_hide_a_transparent_child_or_the_background() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        missed_parent(true, false).find_annotation_entries(point),
        [entry(2, point), entry(1000, point)]
    );
}

#[test]
fn a_missed_parent_does_not_prevent_its_child_from_absorbing() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        missed_parent(false, true).find_annotation_entries(point),
        [entry(2, point)]
    );
}

fn offset_parent(child_origin: Offset) -> LayerTree {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let parent = tree.create_annotated_region(Annotation::new(1_i32), true, Size::new(20., 20.));
    assert!(tree.set_annotation_search_options(
        parent,
        AnnotationSearchOptions {
            offset: Offset::new(90., 90.),
            ..Default::default()
        }
    ));
    // The upstream custom child reports its raw input point even when its
    // own bounds start elsewhere. Reproduce that with an ordinary clip.
    let child_clip =
        tree.create_clip_rect(Rect::from_origin_size(child_origin, Size::new(110., 110.)));
    let child = annotated(&mut tree, 2, false);
    tree.set_children(root, vec![background, parent]);
    tree.set_children(parent, vec![child_clip]);
    tree.set_children(child_clip, vec![child]);
    tree.set_root(root);
    let _ = tree.flatten();
    tree
}

#[test]
fn a_hit_subtracts_the_annotations_own_offset_only_from_its_result() {
    let point = Offset::new(100., 100.);
    assert_eq!(
        offset_parent(Offset::new(20., 20.)).find_annotation_entries(point),
        [
            entry(2, point),
            entry(1, Offset::new(10., 10.)),
            entry(1000, point)
        ]
    );
}

#[test]
fn a_point_inside_the_unshifted_size_is_not_a_hit_when_the_annotation_is_shifted_away() {
    let point = Offset::new(10., 10.);
    assert_eq!(
        offset_parent(Offset::ZERO).find_annotation_entries(point),
        [entry(2, point), entry(1000, point)]
    );
}

#[test]
fn nonuniform_transform_reports_the_inverse_mapped_annotation_point() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let transform = Transform::from_coefficients([2., 0., 0., 4., 10., 20.]);
    let shifted = tree.create_transform(transform);
    let local_bounds = tree.create_clip_rect(rect(10., 10., 10., 10.));
    let foreground = annotated(&mut tree, 1, true);
    tree.set_children(root, vec![background, shifted]);
    tree.set_children(shifted, vec![local_bounds]);
    tree.set_children(local_bounds, vec![foreground]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(
        tree.find_annotation_entries(Offset::new(40., 80.)),
        [entry(1, Offset::new(15., 15.))]
    );
    assert_eq!(
        tree.find_annotation_entries(Offset::new(20., 40.)),
        [entry(1000, Offset::new(20., 40.))]
    );
}

#[test]
fn leader_offset_translates_annotation_placement_and_returns_local_positions() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let background = annotated(&mut tree, 1000, false);
    let size = Size::new(10., 10.);
    let link = LayerLink::new();
    let leader = tree.create_leader_at(link.clone(), size, Offset::new(-10., 0.));
    let foreground = tree.create_annotated_region(Annotation::new(1_i32), true, size);
    assert!(tree.set_annotation_search_options(
        foreground,
        AnnotationSearchOptions {
            opaque: true,
            ..Default::default()
        }
    ));
    tree.set_children(root, vec![background, leader]);
    tree.set_children(leader, vec![foreground]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(
        tree.find_annotation_entries(Offset::new(-5., 5.)),
        [entry(1, Offset::new(5., 5.))]
    );
    assert_eq!(
        tree.find_annotation_entries(Offset::new(5., 5.)),
        [entry(1000, Offset::new(5., 5.))]
    );
    assert_eq!(
        link.leader_transform(),
        Some(Transform::translation(Offset::new(-10., 0.)))
    );
}

#[test]
fn a_parent_contributes_opacity_only_for_its_own_matching_annotation() {
    let point = Offset::new(5., 5.);
    assert_eq!(
        active_parent(false).find_annotation_entries(point),
        [entry(2, point), entry(1, point), entry(1000, point)]
    );
    assert_eq!(
        active_parent(true).find_annotation_entries(point),
        [entry(2, point), entry(1, point)]
    );
}

fn white_picture(tree: &mut LayerTree, size: Size) -> LayerId {
    let mut canvas = Canvas::default();
    let rect = Rect::from_origin_size(Offset::ZERO, size);
    canvas.rect(rect, Color::WHITE);
    tree.create_picture(canvas.finish(), rect)
}

#[test]
fn leader_paint_follower_and_scene_bounds_agree_on_its_offset_before_and_after_update() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::scale_non_uniform(2., 3.));
    let link = LayerLink::new();
    let size = Size::new(4., 4.);
    let leader = tree.create_leader_at(link.clone(), size, Offset::new(5., 6.));
    let follower = tree.create_follower(FollowerLayer {
        link: link.clone(),
        show_when_unlinked: false,
        offset: Offset::new(1., 2.),
        target_anchor: LayerAnchor::TOP_LEFT,
        follower_anchor: LayerAnchor::TOP_LEFT,
        size,
    });
    let leader_picture = white_picture(&mut tree, size);
    let follower_picture = white_picture(&mut tree, size);
    tree.set_children(root, vec![follower, leader]);
    tree.set_children(leader, vec![leader_picture]);
    tree.set_children(follower, vec![follower_picture]);
    tree.set_root(root);

    for (offset, origin) in [
        (Offset::new(5., 6.), Offset::new(10., 18.)),
        (Offset::new(-3., 2.), Offset::new(-6., 6.)),
    ] {
        assert_eq!(tree.update_leader_offset(leader, offset), offset.x < 0.);
        let _ = tree.flatten();
        assert_eq!(
            link.leader_transform()
                .unwrap()
                .transform_point(Offset::ZERO),
            origin
        );
        let world = |layer| {
            tree.flattened_pictures()
                .iter()
                .find(|picture| picture.layer == layer)
                .unwrap()
                .world_bounds
        };
        assert_eq!(
            world(leader_picture),
            Rect::from_origin_size(origin, Size::new(8., 12.))
        );
        assert_eq!(
            world(follower_picture),
            Rect::from_origin_size(origin + Offset::new(2., 6.), Size::new(8., 12.))
        );
        assert_eq!(
            tree.scene_bounds(),
            Some(Rect::from_origin_size(origin, Size::new(10., 18.)))
        );
    }
}

#[test]
fn moving_a_leader_invalidates_cached_clip_culling() {
    let mut tree = LayerTree::new();
    let root = tree.create_clip_rect(rect(10., 10., 5., 5.));
    let size = Size::new(2., 2.);
    let kid = white_picture(&mut tree, size);
    let leader = tree.create_leader_at(LayerLink::new(), size, Offset::new(12., 12.));
    tree.set_children(root, vec![leader]);
    tree.set_children(leader, vec![kid]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(
        tree.flattened_pictures()[0].world_bounds,
        rect(12., 12., 2., 2.)
    );
    assert!(tree.update_leader_offset(leader, Offset::new(30., 30.)));
    let _ = tree.flatten();
    assert!(tree.flattened_pictures().is_empty());
    assert!(tree.update_leader_offset(leader, Offset::new(11., 11.)));
    let _ = tree.flatten();
    assert_eq!(
        tree.flattened_pictures()[0].world_bounds,
        rect(11., 11., 2., 2.)
    );
}
