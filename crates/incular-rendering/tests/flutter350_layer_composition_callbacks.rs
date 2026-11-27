use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{DisplayList, LayerId, LayerTree, PaintCommand};

fn layer_tree() -> (LayerTree, [LayerId; 4]) {
    let mut tree = LayerTree::new();
    let ids: [LayerId; 4] = std::array::from_fn(|_| tree.create_transform(Transform::IDENTITY));
    let [root, first, second, descendant] = ids;
    tree.set_children(root, vec![first, second]);
    tree.set_children(first, vec![descendant]);
    let mut content = DisplayList::new();
    let box_bounds = Rect::from_origin_size(Offset::ZERO, Size::new(1., 1.));
    content.push(PaintCommand::Rect {
        rect: box_bounds,
        color: Color::WHITE,
    });
    let picture = tree.create_picture(content, box_bounds);
    tree.set_children(descendant, vec![picture]);
    tree.set_root(root);
    (tree, ids)
}

fn has_callbacks(tree: &LayerTree, ids: [LayerId; 4]) -> [bool; 4] {
    ids.map(|id| tree.subtree_has_composition_callbacks(id))
}

#[test]
fn subtree_presence_follows_addition_and_cancellation() {
    let (mut tree, [root, first, second, descendant]) = layer_tree();
    let ids = [root, first, second, descendant];
    assert_eq!(has_callbacks(&tree, ids), [false; 4]);
    let root_observer = tree.add_composition_callback(root, |_| {}).unwrap();
    assert_eq!(has_callbacks(&tree, ids), [true, false, false, false]);
    let leaf_observer = tree.add_composition_callback(descendant, |_| {}).unwrap();
    assert_eq!(has_callbacks(&tree, ids), [true, true, false, true]);
    root_observer.cancel();
    assert_eq!(has_callbacks(&tree, ids), [true, true, false, true]);
    leaf_observer.cancel();
    assert_eq!(has_callbacks(&tree, ids), [false; 4]);
}

#[test]
fn removal_from_the_parent_keeps_the_detached_observer_but_not_its_ancestor_counts() {
    let (mut tree, [root, first, second, descendant]) = layer_tree();
    let ids = [root, first, second, descendant];
    assert_eq!(has_callbacks(&tree, ids), [false; 4]);
    let observer = tree.add_composition_callback(descendant, |_| {}).unwrap();
    assert_eq!(has_callbacks(&tree, ids), [true, true, false, true]);
    tree.set_children(first, vec![]);
    assert_eq!(has_callbacks(&tree, ids), [false, false, false, true]);
    assert!(observer.is_active());
    tree.remove(descendant);
    assert!(!observer.is_active());
}

#[test]
fn a_cancelled_callback_does_not_observe_publication() {
    let (mut tree, [_, _, _, descendant]) = layer_tree();
    let observer = tree
        .add_composition_callback(descendant, |_| panic!("cancelled callback ran"))
        .unwrap();
    observer.cancel();
    let _ = tree.flatten();
    assert!(!observer.is_active());
}

#[test]
fn fresh_composition_observers_receive_the_published_layer_once() {
    let (mut tree, [_, _, _, descendant]) = layer_tree();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _observer = tree
        .add_composition_callback(descendant, {
            let seen = seen.clone();
            move |id| seen.borrow_mut().push(id)
        })
        .unwrap();
    assert!(seen.borrow().is_empty());
    let scene = tree.flatten();
    assert_eq!(seen.borrow().as_slice(), &[descendant]);
    assert!(!scene.commands().is_empty());
}

#[test]
fn reused_scene_buffers_still_notify_composition_observers() {
    let (mut tree, [_, _, _, descendant]) = layer_tree();
    let first = tree.flatten();
    let calls = Rc::new(Cell::new(0));
    let observer = tree
        .add_composition_callback(descendant, {
            let calls = calls.clone();
            move |id| {
                assert_eq!(id, descendant);
                calls.set(calls.get() + 1);
            }
        })
        .unwrap();
    assert_eq!(calls.get(), 0);
    let second = tree.flatten();
    assert!(second.shares_buffer(&first));
    assert_eq!(calls.get(), 1);
    drop(observer);
    assert!(tree.flatten().shares_buffer(&second));
    assert_eq!(calls.get(), 1);
}
