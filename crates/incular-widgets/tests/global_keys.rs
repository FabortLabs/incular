//! `GlobalKey` reparenting (misc-011).
//!
//! Flutter 3.41.7 `widgets/framework.dart`: `Element.inflateWidget` calls
//! `_retakeInactiveElement` for a widget with a `GlobalKey`, which takes the
//! element (with its `State` and render object) from wherever it is,
//! deactivated or still attached to its old parent, and
//! `_updateInheritance`/`didChangeDependencies` re-resolve inherited values.
//! `BuildOwner.finalizeTree` unmounts what nobody retook, and a key used
//! twice is reported as a duplicate `GlobalKey`.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, Key, TreeError, Widget, WidgetTree};
use incular_widgets::{Builder, Column, GlobalKey};
use std::cell::RefCell;
use std::rc::Rc;

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100.0, 100.0))
}

fn keyed(key: GlobalKey) -> Widget {
    Widget::box_(Size::new(10.0, 10.0), Color::WHITE).with_key(key)
}

fn column(children: Vec<Widget>) -> Widget {
    Column::new(children).into()
}

/// The keyed element, its render object and its parent.
fn located(tree: &WidgetTree, key: GlobalKey) -> (ElementId, Option<ElementId>) {
    let id = tree
        .element_with_key(&Key::Global(key))
        .expect("keyed element is mounted");
    (id, tree.parent(id))
}

#[test]
fn moving_between_siblings_children_keeps_the_element_in_both_directions() {
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![column(vec![keyed(key)]), column(Vec::new())]))
        .expect("mount");
    tree.layout(constraints()).expect("layout");
    let (element, first_parent) = located(&tree, key);
    let render = tree.render_id(element);
    let mounts = tree.diagnostics().mounts;

    // Earlier sibling to later sibling: removed first, then taken over.
    tree.update(
        root,
        column(vec![column(Vec::new()), column(vec![keyed(key)])]),
    )
    .expect("move to the later sibling");
    tree.layout(constraints()).expect("layout");
    let (moved, second_parent) = located(&tree, key);
    assert_eq!(moved, element, "the element moved, it was not rebuilt");
    assert_eq!(tree.render_id(moved), render, "the render object moved");
    assert_ne!(second_parent, first_parent);
    assert_eq!(tree.children(first_parent.unwrap()), Some(&[][..]));

    // Later sibling to earlier sibling: taken from its still-attached parent.
    tree.update(
        root,
        column(vec![column(vec![keyed(key)]), column(Vec::new())]),
    )
    .expect("move back to the earlier sibling");
    tree.layout(constraints()).expect("layout");
    let (moved, parent) = located(&tree, key);
    assert_eq!(moved, element);
    assert_eq!(parent, first_parent);
    assert_eq!(tree.children(second_parent.unwrap()), Some(&[][..]));
    assert_eq!(
        tree.diagnostics().mounts,
        mounts,
        "no element was mounted for either move"
    );
}

#[test]
fn moving_up_and_down_a_level_keeps_the_element() {
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![column(vec![keyed(key)])]))
        .expect("mount");
    tree.layout(constraints()).expect("layout");
    let (element, _) = located(&tree, key);
    let render = tree.render_id(element);

    tree.update(root, column(vec![keyed(key)]))
        .expect("up a level");
    tree.layout(constraints()).expect("layout");
    assert_eq!(located(&tree, key), (element, Some(root)));
    assert_eq!(tree.render_id(element), render);

    tree.update(root, column(vec![column(vec![keyed(key)])]))
        .expect("down a level");
    tree.layout(constraints()).expect("layout");
    let (moved, parent) = located(&tree, key);
    assert_eq!(moved, element);
    assert_eq!(tree.parent(parent.expect("wrapped")), Some(root));
    assert_eq!(tree.render_id(element), render);
    assert_eq!(tree.element_count(), 3, "root, new wrapper, keyed element");
}

#[test]
fn moved_builder_keeps_its_element_and_resolves_inherited_values_from_its_new_place() {
    let key = GlobalKey::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let child = {
        let seen = seen.clone();
        move || {
            let seen = seen.clone();
            Widget::from(Builder::new(move |context| {
                seen.borrow_mut().push(context.depend_on::<u32>());
                Widget::box_(Size::new(10.0, 10.0), Color::WHITE)
            }))
            .with_key(key)
        }
    };
    let layout = |left: bool| {
        let (a, b) = if left {
            (vec![child()], Vec::new())
        } else {
            (Vec::new(), vec![child()])
        };
        column(vec![
            Widget::environment_scope(1_u32, column(a)),
            Widget::environment_scope(2_u32, column(b)),
        ])
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(layout(true)).expect("mount");
    tree.layout(constraints()).expect("layout");
    let (element, _) = located(&tree, key);
    assert_eq!(seen.borrow().last().copied(), Some(Some(1)));

    // The scopes build their children during layout, so the element is
    // parked by the update and taken over by the materialization.
    tree.update(root, layout(false))
        .expect("move across scopes");
    tree.layout(constraints()).expect("layout");
    assert_eq!(located(&tree, key).0, element, "the builder element moved");
    assert_eq!(
        seen.borrow().last().copied(),
        Some(Some(2)),
        "the moved builder reads the scope above its new position"
    );
    let writes = seen.borrow().len();
    tree.update(root, layout(false)).expect("same place");
    tree.layout(constraints()).expect("layout");
    assert_eq!(located(&tree, key).0, element);
    assert!(seen.borrow().len() > writes, "a new descriptor rebuilds it");
}

#[test]
fn a_removed_global_keyed_element_is_unmounted_when_layout_ends() {
    let key = GlobalKey::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![column(vec![keyed(key)])]))
        .expect("mount");
    tree.layout(constraints()).expect("layout");
    let unmounts = tree.diagnostics().unmounts;
    tree.update(root, column(vec![column(Vec::new())]))
        .expect("remove the keyed element");
    tree.layout(constraints()).expect("layout");
    assert!(tree.element_with_key(&Key::Global(key)).is_none());
    assert_eq!(tree.diagnostics().unmounts, unmounts + 1);
    assert_eq!(tree.element_count(), 2);
}

#[test]
fn one_global_key_on_two_widgets_in_one_build_is_an_error() {
    let key = GlobalKey::new();
    let duplicated = || column(vec![keyed(key), column(vec![keyed(key)])]);
    let mut tree = WidgetTree::new();
    assert!(matches!(
        tree.mount(duplicated()),
        Err(TreeError::DuplicateGlobalKey { key: Key::Global(found) }) if found == key
    ));
    assert_eq!(tree.element_count(), 0, "rejected before anything mounts");

    let root = tree.mount(column(vec![keyed(key)])).expect("mount");
    tree.layout(constraints()).expect("layout");
    let count = tree.element_count();
    assert!(matches!(
        tree.update(root, duplicated()),
        Err(TreeError::DuplicateGlobalKey { .. })
    ));
    assert_eq!(
        tree.element_count(),
        count,
        "the failed update kept the tree"
    );
}

#[test]
fn local_keys_are_unchanged_by_the_global_registry() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![column(vec![
            Widget::box_(Size::new(10.0, 10.0), Color::WHITE).with_key(7_u64),
        ])]))
        .expect("mount");
    tree.layout(constraints()).expect("layout");
    let mounts = tree.diagnostics().mounts;
    tree.update(
        root,
        column(vec![
            Widget::box_(Size::new(10.0, 10.0), Color::WHITE).with_key(7_u64),
        ]),
    )
    .expect("move");
    tree.layout(constraints()).expect("layout");
    assert!(tree.element_with_key(&Key::Value(7)).is_some());
    assert_eq!(
        tree.diagnostics().mounts,
        mounts + 1,
        "a local key does not move across parents"
    );
}
