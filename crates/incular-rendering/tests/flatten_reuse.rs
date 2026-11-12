//! `LayerTree::flatten` hands an unchanged scene out again without walking
//! it, and records a changed one without disturbing lists already handed out.

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::*;

fn row() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(100., 20.))
}

fn picture(color: Color) -> DisplayList {
    let mut list = DisplayList::new();
    list.push(PaintCommand::Rect { rect: row(), color });
    list
}

/// A root transform over an opacity group holding two moved pictures.
struct Scene {
    tree: LayerTree,
    root: LayerId,
    opacity: LayerId,
    placed: [LayerId; 2],
    pictures: [LayerId; 2],
}

impl Scene {
    fn new() -> Self {
        let mut tree = LayerTree::new();
        let pictures = [
            tree.create_picture(picture(Color::WHITE), row()),
            tree.create_picture(picture(Color::BLACK), row()),
        ];
        let placed = [
            tree.create_transform(Transform::translation(Offset::new(0., 0.))),
            tree.create_transform(Transform::translation(Offset::new(0., 20.))),
        ];
        tree.set_children(placed[0], vec![pictures[0]]);
        tree.set_children(placed[1], vec![pictures[1]]);
        let opacity = tree.create_opacity(0.5);
        tree.set_children(opacity, placed.to_vec());
        let root = tree.create_transform(Transform::IDENTITY);
        tree.set_children(root, vec![opacity]);
        tree.set_root(root);
        Self {
            tree,
            root,
            opacity,
            placed,
            pictures,
        }
    }

    /// Applies `change` and reports whether the next flatten differs from
    /// the one before it.
    fn changes(&mut self, change: impl FnOnce(&mut Self)) -> bool {
        let before = self.tree.flatten();
        change(self);
        let after = self.tree.flatten();
        !after.shares_buffer(&before) && after != before
    }
}

#[test]
fn an_unchanged_scene_is_handed_out_again() {
    let mut scene = Scene::new();
    let first = scene.tree.flatten();
    assert_eq!(scene.tree.flattened_pictures().len(), 2);
    let second = scene.tree.flatten();
    assert!(second.shares_buffer(&first));
    // What the first flatten observed still describes the scene.
    assert_eq!(scene.tree.flattened_pictures().len(), 2);
}

#[test]
fn an_update_that_writes_the_same_value_keeps_the_list() {
    let mut scene = Scene::new();
    let first = scene.tree.flatten();
    let placed = scene.placed[0];
    assert!(
        !scene
            .tree
            .update_transform(placed, Transform::translation(Offset::new(0., 0.)))
    );
    assert!(!scene.tree.update_opacity(scene.opacity, 0.5));
    assert!(scene.tree.flatten().shares_buffer(&first));
}

#[test]
fn every_kind_of_change_is_flattened() {
    let mut scene = Scene::new();
    assert!(scene.changes(|scene| {
        let placed = scene.placed[1];
        scene
            .tree
            .update_transform(placed, Transform::translation(Offset::new(4., 20.)));
    }));
    // Opacity changes deliberately leave the content generation alone.
    assert!(scene.changes(|scene| {
        scene.tree.update_opacity(scene.opacity, 0.25);
    }));
    assert!(scene.changes(|scene| {
        let layer = scene.pictures[0];
        scene
            .tree
            .update_picture(layer, picture(Color::rgba(1, 2, 3, 255)), row());
    }));
    assert!(scene.changes(|scene| {
        let reversed = vec![scene.placed[1], scene.placed[0]];
        scene.tree.set_children(scene.opacity, reversed);
    }));
    assert!(scene.changes(|scene| {
        let removed = scene.placed[0];
        scene
            .tree
            .set_children(scene.opacity, vec![scene.placed[1]]);
        scene.tree.remove(removed);
    }));
    assert!(scene.changes(|scene| {
        let layer = scene.tree.create_picture(picture(Color::WHITE), row());
        scene
            .tree
            .set_children(scene.root, vec![scene.opacity, layer]);
    }));
    assert!(scene.changes(|scene| {
        let blur = scene.tree.create_blur(GaussianBlur::new(2., 2.));
        scene.tree.set_children(blur, vec![scene.opacity]);
        scene.tree.set_root(blur);
    }));
}

#[test]
fn a_changed_scene_leaves_a_held_list_as_it_was() {
    let mut scene = Scene::new();
    let held = scene.tree.flatten();
    let snapshot = held.commands().to_vec();
    for step in 1..=4 {
        let placed = scene.placed[0];
        scene
            .tree
            .update_transform(placed, Transform::translation(Offset::new(step as f32, 0.)));
        let next = scene.tree.flatten();
        assert_ne!(next, held);
        assert_eq!(held.commands(), snapshot.as_slice());
    }
}

#[test]
fn buffers_are_recycled_without_leaking_into_published_lists() {
    let mut scene = Scene::new();
    // No reader holds a list across frames here, so each flatten records
    // into the buffer the one before last published.
    let mut expected_x = 0.;
    for step in 1..=6 {
        let placed = scene.placed[0];
        expected_x = step as f32;
        scene
            .tree
            .update_transform(placed, Transform::translation(Offset::new(expected_x, 0.)));
        let list = scene.tree.flatten();
        let moved = list.commands().iter().any(|command| {
            matches!(
                command,
                PaintCommand::PushTransform { transform }
                    if *transform == Transform::translation(Offset::new(expected_x, 0.))
            )
        });
        assert!(moved, "step {step} publishes its own transform");
        assert_eq!(list, scene.tree.flatten());
    }
    assert!(expected_x > 0.);
}

#[test]
fn reapplied_partition_markers_keep_the_list() {
    let mut scene = Scene::new();
    let partition = SurfacePartitionId::from_parts(3, 1);
    let layer = scene.placed[1];
    scene.tree.set_surface_partition(layer, Some(partition));
    assert!(scene.tree.has_surface_partitions());
    let first = scene.tree.flatten();
    assert!(first.commands().iter().any(
        |command| matches!(command, PaintCommand::PushSurfacePartition { id } if *id == partition)
    ));
    // A host clears and reapplies its markers before every flatten.
    scene.tree.clear_surface_partitions();
    assert!(!scene.tree.has_surface_partitions());
    scene.tree.set_surface_partition(layer, Some(partition));
    assert!(scene.tree.flatten().shares_buffer(&first));

    // A different marker, a moved marker and a dropped marker all change it.
    let other = SurfacePartitionId::from_parts(4, 1);
    assert!(scene.changes(|scene| {
        scene.tree.clear_surface_partitions();
        scene.tree.set_surface_partition(layer, Some(other));
    }));
    assert!(scene.changes(|scene| {
        scene.tree.clear_surface_partitions();
        let moved = scene.placed[0];
        scene.tree.set_surface_partition(moved, Some(other));
    }));
    assert!(scene.changes(|scene| scene.tree.clear_surface_partitions()));
    assert!(
        !scene
            .tree
            .flatten()
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushSurfacePartition { .. }))
    );
}

#[test]
fn a_scene_with_a_follower_is_resolved_on_every_flatten() {
    let mut scene = Scene::new();
    let link = LayerLink::new();
    let follower = scene.tree.create_follower(FollowerLayer {
        link: link.clone(),
        show_when_unlinked: true,
        offset: Offset::ZERO,
        target_anchor: LayerAnchor::new(0., 0.),
        follower_anchor: LayerAnchor::new(0., 0.),
        size: Size::new(10., 10.),
    });
    let content = scene.tree.create_picture(picture(Color::WHITE), row());
    scene.tree.set_children(follower, vec![content]);
    scene
        .tree
        .set_children(scene.root, vec![scene.opacity, follower]);
    let unlinked = scene.tree.flatten();

    // The leader lives in the same tree here, but a link is shared state:
    // it may equally be published from elsewhere between two flattens.
    let leader = scene.tree.create_leader(link, Size::new(10., 10.));
    let moved = scene
        .tree
        .create_transform(Transform::translation(Offset::new(50., 60.)));
    scene.tree.set_children(moved, vec![leader]);
    scene
        .tree
        .set_children(scene.root, vec![scene.opacity, moved, follower]);
    let linked = scene.tree.flatten();
    assert_ne!(linked, unlinked);
    assert_eq!(scene.tree.flatten(), linked);
}

#[test]
fn lists_share_a_buffer_only_while_neither_records() {
    let list = picture(Color::WHITE);
    let mut copy = list.clone();
    assert!(copy.shares_buffer(&list));
    assert_eq!(copy, list);
    copy.push(PaintCommand::PopClip);
    assert!(!copy.shares_buffer(&list));
    assert_ne!(copy, list);
    assert!(!DisplayList::new().shares_buffer(&DisplayList::new()));
    assert_eq!(DisplayList::new(), DisplayList::new());
}
