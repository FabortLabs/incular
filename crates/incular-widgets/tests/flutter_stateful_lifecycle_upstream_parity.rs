//! Retained lifecycle counterparts from Flutter 3.50.0.
//! A public stateful builder owns a Rust closure and takes a revision, not a
//! Dart State object; compare retained identity and cleanup, not Dart hooks.

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Builder, Column, SizedBox, Widget,
    internal::{ElementId, WidgetTree},
};
use std::{cell::Cell, rc::Rc};

fn pump(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .expect("build retained lifecycle scene");
}

fn element(tree: &WidgetTree, key: &str) -> ElementId {
    tree.element_with_key(&key.into()).expect("keyed element")
}

/// stateful_component_test.dart:60 "Don't rebuild subwidgets".
#[test]
fn replacing_the_active_child_does_not_rebuild_the_outgoing_child() {
    let left_builds = Rc::new(Cell::new(0));
    let left = Widget::from(Builder::new({
        let left_builds = left_builds.clone();
        move |_| {
            left_builds.set(left_builds.get() + 1);
            SizedBox::square(10.).into()
        }
    }))
    .with_key("left");
    let right = Widget::from(SizedBox::square(10.)).with_key("right");
    let show_left = Rc::new(Cell::new(true));
    let revision = Rc::new(Cell::new(0_u64));
    let root = Widget::stateful_builder(revision.clone(), {
        let show_left = show_left.clone();
        move |_| {
            if show_left.get() {
                left.clone()
            } else {
                right.clone()
            }
        }
    })
    .with_key("rebuild test");
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount flip builder");
    pump(&mut tree);
    let left = element(&tree, "left");
    assert_eq!(left_builds.get(), 1);
    assert!(tree.element_with_key(&"right".into()).is_none());

    show_left.set(false);
    revision.set(1);
    pump(&mut tree);
    assert_eq!(left_builds.get(), 1);
    assert!(!tree.element_exists(left));
    let _ = element(&tree, "right");
}

/// stateful_components_test.dart:47 "resync stateful widget".
#[test]
fn same_key_retains_nested_builder_element_and_render_across_parent_rebuilds() {
    let outer_revision = Rc::new(Cell::new(0_u64));
    let outer_builds = Rc::new(Cell::new(0));
    let inner = Widget::stateful_builder(Rc::new(Cell::new(0)), |_| {
        Widget::from(SizedBox::square(10.)).with_key("leaf")
    })
    .with_key("inner");
    let outer = || {
        let (outer_builds, inner) = (outer_builds.clone(), inner.clone());
        Widget::stateful_builder(outer_revision.clone(), move |_| {
            outer_builds.set(outer_builds.get() + 1);
            inner.clone()
        })
        .with_key("outer")
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(outer()).expect("mount nested builders");
    pump(&mut tree);
    let inner = element(&tree, "inner");
    let render = tree.render_id(inner).expect("inner render object");
    let leaf = element(&tree, "leaf");
    assert_eq!(element(&tree, "outer"), root);
    assert_eq!(tree.element_for_render(render), Some(inner));

    tree.update(root, outer())
        .expect("replace parent descriptor");
    pump(&mut tree);
    assert_eq!(element(&tree, "outer"), root);
    assert_eq!(element(&tree, "inner"), inner);
    assert_eq!(tree.render_id(inner), Some(render));
    assert_eq!(tree.element_for_render(render), Some(inner));
    assert_eq!(element(&tree, "leaf"), leaf);

    let prior_builds = outer_builds.get();
    outer_revision.set(1);
    pump(&mut tree);
    assert_eq!(outer_builds.get(), prior_builds + 1);
    assert_eq!(element(&tree, "inner"), inner);
    assert_eq!(tree.element_for_render(render), Some(inner));
    assert_eq!(element(&tree, "leaf"), leaf);
}

/// framework_test.dart:1776 "Widget and State properties are nulled out when
/// unmounted". Observe the Rust lifetime counterpart with stale IDs and a
/// weak reference, without keeping the owned widget descriptor alive.
#[test]
fn unmount_releases_owned_builder_capture_and_invalidates_stale_ids() {
    let capture = Rc::new(Cell::new(0_u32));
    let retained_capture = Rc::downgrade(&capture);
    let child = Widget::stateful_builder(Rc::new(Cell::new(0)), move |_| {
        capture.set(capture.get() + 1);
        SizedBox::square(10.).into()
    })
    .with_key("stateful leaf");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Column::new(vec![child]).into())
        .expect("mount owned builder");
    pump(&mut tree);
    let stale = element(&tree, "stateful leaf");
    let render = tree.render_id(stale).expect("live render identity");
    assert_eq!(
        retained_capture
            .upgrade()
            .expect("live builder capture")
            .get(),
        1
    );

    tree.update(
        root,
        Column::new(vec![
            Widget::from(SizedBox::square(10.)).with_key("replacement"),
        ])
        .into(),
    )
    .expect("remove stateful builder");
    pump(&mut tree);

    assert!(tree.element_with_key(&"stateful leaf".into()).is_none());
    assert!(!tree.element_exists(stale));
    assert_eq!(tree.render_id(stale), None);
    assert_eq!(tree.element_for_render(render), None);
    assert!(
        retained_capture.upgrade().is_none(),
        "tree does not retain removed closure"
    );
    let _ = element(&tree, "replacement");
}
