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

#[test]
fn detaching_the_root_notifies_descendants_then_retains_their_subscriptions() {
    let (mut tree, [root, first, second, descendant]) = layer_tree();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let subscriptions: Vec<_> = [root, first, second, descendant]
        .into_iter()
        .map(|id| {
            let seen = seen.clone();
            tree.add_composition_callback(id, move |id| seen.borrow_mut().push(id))
                .unwrap()
        })
        .collect();

    tree.set_root(root);
    assert!(seen.borrow().is_empty());
    assert_eq!(tree.detach_root(), Some(root));
    assert_eq!(*seen.borrow(), [descendant, first, second, root]);
    assert!(
        subscriptions
            .iter()
            .all(|subscription| subscription.is_active())
    );
    assert!(tree.subtree_has_composition_callbacks(root));
    assert_eq!(tree.detach_root(), None);
    assert!(tree.flatten().is_empty());
    assert_eq!(seen.borrow().len(), 4);

    seen.borrow_mut().clear();
    tree.set_root(root);
    assert!(seen.borrow().is_empty());
    let _ = tree.flatten();
    assert_eq!(*seen.borrow(), [root, first, descendant, second]);
}

#[test]
fn two_observers_on_one_layer_keep_it_observed_until_both_are_removed() {
    let (mut tree, [root, first, _, _]) = layer_tree();
    assert!(!tree.subtree_has_composition_callbacks(root));
    assert!(!tree.subtree_has_composition_callbacks(first));
    let one = tree.add_composition_callback(first, |_| {}).unwrap();
    let two = tree.add_composition_callback(first, |_| {}).unwrap();
    assert!(tree.subtree_has_composition_callbacks(root));
    assert!(tree.subtree_has_composition_callbacks(first));
    one.cancel();
    assert!(tree.subtree_has_composition_callbacks(root));
    assert!(tree.subtree_has_composition_callbacks(first));
    two.cancel();
    assert!(!tree.subtree_has_composition_callbacks(root));
    assert!(!tree.subtree_has_composition_callbacks(first));
}

#[test]
fn cancellation_after_layer_disposal_is_safe() {
    let (mut tree, [root, _, _, _]) = layer_tree();
    let observer = tree.add_composition_callback(root, |_| {}).unwrap();
    tree.remove(root);
    observer.cancel();
    assert!(!observer.is_active());
    assert!(!tree.subtree_has_composition_callbacks(root));
}

#[test]
fn repeated_cancellation_is_idempotent_in_the_rust_subscription_api() {
    let (mut tree, [root, _, _, _]) = layer_tree();
    let observer = tree
        .add_composition_callback(root, |_| panic!("cancelled"))
        .unwrap();
    observer.cancel();
    observer.cancel();
    assert!(!observer.is_active());
    let _ = tree.flatten();
}

#[test]
fn removing_an_attached_branch_notifies_it_once_before_other_scenes_continue() {
    let (mut tree, [root, branch, sibling, child]) = layer_tree();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let subscriptions: Vec<_> = [root, branch, sibling, child]
        .into_iter()
        .map(|id| {
            let seen = seen.clone();
            tree.add_composition_callback(id, move |id| seen.borrow_mut().push(id))
                .unwrap()
        })
        .collect();

    let _ = tree.flatten();
    assert_eq!(seen.borrow().as_slice(), &[root, branch, child, sibling]);
    seen.borrow_mut().clear();

    tree.set_children(root, vec![sibling, branch]);
    assert!(
        seen.borrow().is_empty(),
        "order changed, but no branch detached"
    );
    tree.set_children(root, vec![sibling]);
    assert_eq!(seen.borrow().as_slice(), &[child, branch]);
    assert!(subscriptions.iter().all(|listener| listener.is_active()));

    seen.borrow_mut().clear();
    let _ = tree.flatten();
    assert_eq!(seen.borrow().as_slice(), &[root, sibling]);
    seen.borrow_mut().clear();
    tree.set_children(root, vec![branch, sibling]);
    assert!(seen.borrow().is_empty());
    let _ = tree.flatten();
    assert_eq!(seen.borrow().as_slice(), &[root, branch, child, sibling]);
}

#[test]
fn atomic_adoption_within_a_scene_and_unattached_staging_do_not_detach() {
    let (mut tree, [root, _, sibling, child]) = layer_tree();
    let count = Rc::new(Cell::new(0));
    let _observer = tree
        .add_composition_callback(child, {
            let count = count.clone();
            move |_| count.set(count.get() + 1)
        })
        .unwrap();
    tree.set_children(sibling, vec![child]);
    assert_eq!(count.get(), 0);
    let _ = tree.flatten();
    assert_eq!(count.get(), 1);
    tree.set_children(sibling, vec![]);
    assert_eq!(count.get(), 2);

    let staging = tree.create_transform(Transform::IDENTITY);
    tree.set_children(staging, vec![child]);
    tree.set_children(staging, vec![]);
    assert_eq!(
        count.get(),
        2,
        "an unpublished staging branch never detached"
    );
    let _ = tree.flatten();
    assert_eq!(count.get(), 2);
    assert_eq!(tree.root(), Some(root));
}

#[test]
fn moving_a_live_layer_into_a_detached_tree_is_a_real_detachment() {
    let (mut tree, [root, first, sibling, child]) = layer_tree();
    let count = Rc::new(Cell::new(0));
    let observer = tree
        .add_composition_callback(child, {
            let count = count.clone();
            move |_| count.set(count.get() + 1)
        })
        .unwrap();
    let _ = tree.flatten();
    assert_eq!(count.get(), 1);

    let staging = tree.create_transform(Transform::IDENTITY);
    tree.set_children(staging, vec![child]);
    assert_eq!(count.get(), 2);
    assert!(observer.is_active());
    assert!(!tree.subtree_has_composition_callbacks(root));

    tree.set_children(root, vec![first, sibling, staging]);
    assert_eq!(count.get(), 2);
    let _ = tree.flatten();
    assert_eq!(count.get(), 3);
}
