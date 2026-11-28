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

#[test]
fn replacing_an_attached_leaders_link_then_removing_it_releases_both_publications() {
    use incular_rendering::LayerLink;

    let [original, changed] = std::array::from_fn(|_| LayerLink::new());
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let size = Size::new(24., 18.);
    let leader = tree.create_leader(original.clone(), size);
    tree.set_children(root, vec![leader]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert_eq!(original.leader_size(), Some(size));
    assert!(!changed.is_linked());

    assert!(tree.update_leader(leader, changed.clone(), size));
    assert!(!original.is_linked());
    let _ = tree.flatten();
    assert_eq!(changed.leader_size(), Some(size));
    tree.remove(leader);
    let _ = tree.flatten();
    assert!(!original.is_linked() && !changed.is_linked());
    assert!(tree.layer(leader).is_none());
}

#[test]
fn attaching_two_leaders_then_removing_the_second_before_the_first_clears_the_link() {
    use incular_rendering::LayerLink;

    let link = LayerLink::new();
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let first = tree.create_leader(link.clone(), Size::new(24., 18.));
    let second = tree.create_leader(link.clone(), Size::new(11., 12.));
    tree.set_children(root, vec![first]);
    tree.set_root(root);
    let _ = tree.flatten();
    assert!(link.is_linked());

    tree.set_children(root, vec![first, second]);
    let _ = tree.flatten();
    assert!(link.is_linked());
    tree.remove(second);
    let _ = tree.flatten();
    assert_eq!(link.leader_size(), Some(Size::new(24., 18.)));
    tree.remove(first);
    let _ = tree.flatten();
    assert!(!link.is_linked());
}

#[test]
fn connecting_and_disconnecting_followers_does_not_invalidate_the_leader_picture() {
    use incular_rendering::{FollowerLayer, LayerAnchor, LayerLink};

    let mut tree = LayerTree::new();
    let link = LayerLink::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let offset = Offset::new(31., 17.);
    let leader = tree.create_leader_at(link.clone(), unit().size, offset);
    let paint = picture(&mut tree);
    tree.set_children(leader, vec![paint]);
    tree.set_children(root, vec![leader]);
    tree.set_root(root);
    let _ = tree.flatten();
    let generation = link.leader_generation();
    let prior_repaints = tree.diagnostics().picture_layers_repainted;
    assert_ne!(generation, 0);
    let follower = |tree: &mut LayerTree, x| {
        let layer = tree.create_follower(FollowerLayer {
            link: link.clone(),
            show_when_unlinked: false,
            offset: Offset::new(x, 0.),
            target_anchor: LayerAnchor::TOP_LEFT,
            follower_anchor: LayerAnchor::TOP_LEFT,
            size: unit().size,
        });
        let paint = picture(tree);
        tree.set_children(layer, vec![paint]);
        layer
    };
    let a = follower(&mut tree, 10.);
    let b = follower(&mut tree, 20.);
    for (children, origins) in [
        (vec![leader, a], vec![offset, Offset::new(41., 17.)]),
        (
            vec![leader, a, b],
            vec![offset, Offset::new(41., 17.), Offset::new(51., 17.)],
        ),
        (vec![leader, a], vec![offset, Offset::new(41., 17.)]),
        (vec![leader], vec![offset]),
    ] {
        tree.set_children(root, children);
        let _ = tree.flatten();
        assert_eq!(link.leader_generation(), generation);
        assert_eq!(tree.diagnostics().picture_layers_repainted, prior_repaints);
        assert_eq!(
            tree.flattened_pictures()
                .iter()
                .map(|picture| picture.world_bounds.origin)
                .collect::<Vec<_>>(),
            origins
        );
        assert_eq!(tree.layer(leader).unwrap().1, [paint]);
    }
}

#[test]
fn a_retained_leader_still_projects_its_offset_after_its_parent_repaints() {
    use incular_rendering::LayerLink;

    let mut tree = LayerTree::new();
    let link = LayerLink::new();
    let [outer, inner] = std::array::from_fn(|_| tree.create_transform(Transform::IDENTITY));
    let offset = Offset::new(12., 23.);
    let leader = tree.create_leader_at(link.clone(), unit().size, offset);
    let leader_picture = picture(&mut tree);
    let outer_picture = picture(&mut tree);
    tree.set_children(outer, vec![outer_picture, inner]);
    tree.set_children(inner, vec![leader]);
    tree.set_children(leader, vec![leader_picture]);
    tree.set_root(outer);
    let _ = tree.flatten();
    let before = link.leader_generation();
    let expected = Transform::translation(offset);
    assert_eq!(link.leader_transform(), Some(expected));

    let mut rebuilt = DisplayList::new();
    rebuilt.push(PaintCommand::Rect {
        rect: unit(),
        color: Color::BLACK,
    });
    tree.update_picture(outer_picture, rebuilt, unit());
    let display = tree.flatten();
    assert!(display.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::BLACK,
            ..
        }
    )));
    assert_eq!(link.leader_transform(), Some(expected));
    assert_eq!(link.leader_generation(), before);
    let retained = tree
        .flattened_pictures()
        .iter()
        .find(|picture| picture.layer == leader_picture)
        .unwrap();
    assert_eq!(retained.world_bounds.origin, offset);
}

#[test]
fn a_hidden_leader_cannot_claim_the_link_before_a_visible_one() {
    use incular_rendering::LayerLink;

    let mut tree = LayerTree::new();
    let link = LayerLink::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let hidden = tree.create_opacity(0.0);
    let placed = tree.create_leader_at(link.clone(), unit().size, Offset::new(10., 10.));
    let fallback = tree.create_leader_at(link.clone(), unit().size, Offset::new(30., 30.));
    tree.set_children(root, vec![hidden, fallback]);
    tree.set_children(hidden, vec![placed]);
    tree.set_root(root);
    for (alpha, origin) in [
        (0.0, Offset::new(30., 30.)),
        (0.001, Offset::new(30., 30.)),
        (0.5, Offset::new(10., 10.)),
        (0.0, Offset::new(30., 30.)),
    ] {
        tree.update_opacity(hidden, alpha);
        let _ = tree.flatten();
        assert_eq!(
            link.leader_transform(),
            Some(Transform::translation(origin))
        );
    }
}
