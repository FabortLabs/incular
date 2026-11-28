//! Direct retained compositor proof, independent of widget paint suppression.
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Canvas, LayerTree, PaintCommand};

#[test]
fn invalid_geometry_has_no_visual_bounds_and_never_submits_cached_paint() {
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(30., 20.));
    let mut canvas = Canvas::default();
    canvas.rect(bounds, Color::WHITE);
    let mut layers = LayerTree::new();
    let picture = layers.create_picture(canvas.finish(), bounds);
    let transform = layers.create_transform(Transform::IDENTITY);
    layers.set_children(transform, vec![picture]);
    let root = layers.create_clip_rect(bounds);
    layers.set_children(root, vec![transform]);
    layers.set_root(root);

    let visible = layers.flatten();
    assert!(
        visible
            .commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::Rect { .. }))
    );
    assert_eq!(layers.scene_bounds(), Some(bounds));
    assert_eq!(layers.flattened_pictures()[0].layer, picture);

    for hidden in [
        Transform::scale(0.),
        Transform::from_coefficients([1., 0., 0., 1., f64::NAN, 0.]),
        Transform::from_coefficients([f64::INFINITY, 0., 0., 1., 0., 0.]),
        // Matrix inputs can be finite and still overflow when projected
        // into the renderer's f32 world-space coordinates.
        Transform::scale(f32::MAX * 0.5),
        Transform::from_coefficients([1., 0., 0., 1., 1.0e40, 0.]),
    ] {
        assert!(layers.update_transform(transform, hidden));
        assert_eq!(layers.scene_bounds(), None);
        assert!(layers.debug_tree().contains("non-drawable transform"));
        assert!(
            !layers
                .flatten()
                .commands()
                .iter()
                .any(|op| matches!(op, PaintCommand::Rect { .. }))
        );
        assert!(layers.flattened_pictures().is_empty());
        assert_eq!(layers.layer(transform).unwrap().1, [picture]);
        assert!(layers.update_transform(transform, Transform::IDENTITY));
        assert_eq!(layers.flatten(), visible);
        assert_eq!(layers.scene_bounds(), Some(bounds));
        assert_eq!(layers.flattened_pictures()[0].layer, picture);
    }
}

#[test]
fn oversized_intermediate_space_does_not_cull_a_representable_composed_transform() {
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(30., 20.));
    let mut canvas = Canvas::default();
    canvas.rect(bounds, Color::WHITE);
    let mut layers = LayerTree::new();
    let picture = layers.create_picture(canvas.finish(), bounds);
    let large = layers.create_transform(Transform::from_coefficients([
        1.0e39, 0., 0., 1.0e39, 0., 0.,
    ]));
    let small = layers.create_transform(Transform::from_coefficients([
        1.0e-39, 0., 0., 1.0e-39, 0., 0.,
    ]));
    layers.set_children(large, vec![picture]);
    layers.set_children(small, vec![large]);
    let root = layers.create_clip_rect(bounds);
    layers.set_children(root, vec![small]);
    layers.set_root(root);
    assert_eq!(layers.scene_bounds(), Some(bounds));
    let display = layers.flatten();
    assert!(
        display
            .commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::Rect { .. }))
    );
    assert_eq!(layers.flattened_pictures()[0].layer, picture);
    assert_eq!(layers.flattened_pictures()[0].world_bounds, bounds);
}

#[test]
fn a_sibling_union_out_of_range_disables_culling_without_losing_visible_paint() {
    let mut layers = LayerTree::new();
    let mut picture = |x, width| {
        let bounds = Rect::from_origin_size(Offset::new(x, 0.), Size::new(width, 20.));
        let mut canvas = Canvas::default();
        canvas.rect(bounds, Color::WHITE);
        layers.create_picture(canvas.finish(), bounds)
    };
    let before = picture(-2.0e38, 1.0e37);
    let live = picture(0., 30.);
    let after = picture(2.0e38, 1.0e37);
    let group = layers.create_transform(Transform::IDENTITY);
    layers.set_children(group, vec![before, live, after]);
    layers.set_root(group);
    assert_eq!(layers.scene_bounds(), None);

    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(30., 20.));
    let clip = layers.create_clip_rect(bounds);
    layers.set_children(clip, vec![group]);
    layers.set_root(clip);
    let _ = layers.flatten();
    let [only] = layers.flattened_pictures() else {
        panic!("only the marker inside the clip is visible");
    };
    assert_eq!(only.layer, live);
    assert_eq!(only.world_bounds, bounds);
}
