use std::sync::{Arc, Weak};

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, LayerId, LayerKind, LayerTree, PaintCommand};

fn bounds() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.))
}

fn solid(color: Color) -> DisplayList {
    let mut display = DisplayList::new();
    display.push(PaintCommand::Rect {
        rect: bounds(),
        color,
    });
    display
}

fn picture(tree: &LayerTree, id: LayerId) -> &Arc<DisplayList> {
    let (LayerKind::Picture { display_list, .. }, _) = tree.layer(id).unwrap() else {
        panic!("a mounted picture layer");
    };
    display_list
}

fn fills(display: &DisplayList) -> Vec<(Rect, Color)> {
    display
        .commands()
        .iter()
        .filter_map(|op| match op {
            PaintCommand::Rect { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

fn setup(color: Color) -> (LayerTree, LayerId, LayerId, Weak<DisplayList>) {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let child = tree.create_picture(solid(color), bounds());
    let storage = Arc::downgrade(picture(&tree, child));
    tree.set_children(root, vec![child]);
    tree.set_root(root);
    let display = tree.flatten();
    assert_eq!(fills(&display), [(bounds(), color)]);
    (tree, root, child, storage)
}

#[test]
fn clearing_a_picture_releases_its_previous_owned_display_list() {
    let (mut tree, _, child, old) = setup(Color::WHITE);
    assert!(old.upgrade().is_some());
    tree.update_picture(child, DisplayList::new(), bounds());
    assert!(old.upgrade().is_none());
    let rebuilt = tree.flatten();
    assert!(fills(&rebuilt).is_empty());
    assert!(
        tree.layer(child).is_some(),
        "clearing the payload is not removing its layer"
    );
}

#[test]
fn removing_a_picture_releases_unshared_storage_and_stale_ids_never_readopt_it() {
    let (mut tree, root, child, old) = setup(Color::WHITE);
    assert_eq!(tree.layer(root).unwrap().1, [child]);
    tree.set_children(root, vec![]);
    assert!(
        old.upgrade().is_some(),
        "an unparented arena layer is still owned"
    );
    assert!(fills(&tree.flatten()).is_empty());

    tree.remove(child);
    assert!(old.upgrade().is_none());
    assert!(tree.layer(child).is_none());
    tree.update_picture(child, solid(Color::WHITE), bounds());
    assert!(fills(&tree.flatten()).is_empty());
    let newer = tree.create_picture(solid(Color::BLACK), bounds());
    assert_ne!(newer, child);
    tree.set_children(root, vec![child, newer]);
    assert_eq!(fills(&tree.flatten()), [(bounds(), Color::BLACK)]);
}

#[test]
fn shared_picture_storage_lives_only_as_long_as_the_last_owner() {
    let (mut tree, _, id, previous) = setup(Color::WHITE);
    let owned_snapshot = Arc::clone(picture(&tree, id));
    assert_eq!(Arc::strong_count(&owned_snapshot), 2);
    tree.remove(id);
    assert!(fills(&tree.flatten()).is_empty());
    assert_eq!(Arc::strong_count(&owned_snapshot), 1);
    assert_eq!(owned_snapshot.commands(), solid(Color::WHITE).commands());
    drop(owned_snapshot);
    assert!(previous.upgrade().is_none());
}

#[test]
fn zero_opacity_excludes_the_picture_then_reuses_its_existing_storage() {
    let (mut tree, root, child, retained) = setup(Color::WHITE);
    let opacity = tree.create_opacity(1.);
    let nested_boundary = tree.create_transform(Transform::IDENTITY);
    tree.set_children(root, vec![opacity]);
    tree.set_children(opacity, vec![nested_boundary]);
    tree.set_children(nested_boundary, vec![child]);
    let _ = tree.flatten();
    assert_eq!(tree.flattened_pictures()[0].layer, child);
    let repainted = tree.diagnostics().picture_layers_repainted;

    assert!(tree.update_opacity(opacity, 0.));
    let hidden = tree.flatten();
    assert!(fills(&hidden).is_empty());
    assert!(tree.flattened_pictures().is_empty());
    assert_eq!(tree.layer(opacity).unwrap().1, [nested_boundary]);
    assert!(retained.upgrade().is_some());

    assert!(tree.update_opacity(opacity, 0.5));
    let restored = tree.flatten();
    assert_eq!(tree.flattened_pictures()[0].layer, child);
    assert!(restored.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
    assert!(Arc::ptr_eq(
        picture(&tree, child),
        &retained.upgrade().unwrap()
    ));
    assert_eq!(tree.diagnostics().picture_layers_repainted, repainted);
}
