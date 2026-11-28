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
