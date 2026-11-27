use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, LayerId, LayerTree, PaintCommand};

fn unit() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(4., 4.))
}

fn picture(tree: &mut LayerTree) -> LayerId {
    let mut commands = DisplayList::new();
    commands.push(PaintCommand::Rect {
        rect: unit(),
        color: Color::WHITE,
    });
    tree.create_picture(commands, unit())
}

#[test]
fn reparenting_a_layer_moves_its_paint_instead_of_leaving_a_ghost() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let old = tree.create_transform(Transform::IDENTITY);
    let destination = tree.create_transform(Transform::translation(Offset::new(20., 0.)));
    let paint = picture(&mut tree);
    tree.set_children(root, vec![old, destination]);
    tree.set_children(old, vec![paint]);
    tree.set_root(root);

    let before = tree.flatten();
    assert_eq!(tree.flattened_pictures().len(), 1);
    assert_eq!(
        tree.flattened_pictures()[0].world_bounds.origin,
        Offset::ZERO
    );
    assert_eq!(
        before
            .commands()
            .iter()
            .filter(|op| matches!(
                op,
                PaintCommand::Rect {
                    color: Color::WHITE,
                    ..
                }
            ))
            .count(),
        1
    );

    // Adoption is atomic for native callers. The old parent is not told
    // to emit its current children again after this move.
    tree.set_children(destination, vec![paint]);
    let after = tree.flatten();
    assert_eq!(tree.flattened_pictures().len(), 1);
    assert_eq!(tree.flattened_pictures()[0].layer, paint);
    assert_eq!(
        tree.flattened_pictures()[0].world_bounds.origin,
        Offset::new(20., 0.)
    );
    assert!(tree.layer(old).unwrap().1.is_empty());
    assert_eq!(
        after
            .commands()
            .iter()
            .filter(|op| matches!(
                op,
                PaintCommand::Rect {
                    color: Color::WHITE,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(!after.shares_buffer(&before));
}

#[test]
fn depth_first_traversal_follows_sibling_order_and_drops_a_removed_branch() {
    let mut tree = LayerTree::new();
    let [a, b, c, d, e, f, g] = std::array::from_fn(|_| tree.create_transform(Transform::IDENTITY));
    let [h, i, j] = std::array::from_fn(|_| picture(&mut tree));
    tree.set_children(a, vec![b, c]);
    tree.set_children(b, vec![d, e, f]);
    tree.set_children(d, vec![h, i]);
    tree.set_children(c, vec![g]);
    tree.set_children(g, vec![j]);
    assert_eq!(tree.depth_first_children(a), [b, d, h, i, e, f, c, g, j]);
    tree.remove(d);
    assert_eq!(tree.depth_first_children(a), [b, e, f, c, g, j]);
}

#[test]
fn clip_bounds_describe_each_local_shape_not_the_parent_transform() {
    use incular_rendering::{CornerRadii, FillRule, Path, RRect};
    use std::sync::Arc;

    let mut tree = LayerTree::new();
    let box_bounds = Rect::from_origin_size(Offset::new(10., 10.), Size::new(10., 10.));
    let parent = tree.create_transform(Transform::translation(Offset::new(20., -5.)));
    assert_eq!(tree.describe_clip_bounds(parent), None);
    let rect = tree.create_clip_rect(box_bounds);
    let rounded = tree.create_clip_rrect(RRect::uniform(box_bounds, 2.));
    let superellipse = tree.create_clip_rsuperellipse(box_bounds, CornerRadii::uniform(2.));
    let mut square = Path::builder();
    square
        .move_to(Offset::new(10., 10.))
        .line_to(Offset::new(20., 10.))
        .line_to(Offset::new(20., 20.))
        .line_to(Offset::new(10., 20.))
        .close();
    let path = tree.create_clip_path(Arc::new(square.build()), FillRule::NonZero);
    let clips = [rect, rounded, superellipse, path];
    tree.set_children(parent, clips.to_vec());
    for layer in clips {
        assert_eq!(tree.describe_clip_bounds(layer), Some(box_bounds));
    }

    let mut curve = Path::builder();
    curve
        .move_to(Offset::ZERO)
        .quadratic_to(Offset::new(10., 20.), Offset::new(20., 0.));
    let curve = Arc::new(curve.build());
    let curve_id = tree.create_clip_path(curve.clone(), FillRule::NonZero);
    assert_eq!(curve.tight_bounds().unwrap().size.height, 10.);
    assert_eq!(
        tree.describe_clip_bounds(curve_id).unwrap().size.height,
        20.
    );
}
