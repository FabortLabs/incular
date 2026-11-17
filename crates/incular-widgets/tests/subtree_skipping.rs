//! Unchanged descriptors skip retained work without losing live descendants.

use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_semantics::{Role, SemanticActionKind};
use incular_widgets::internal::{ExplicitSemantics, GestureCallbacks, Key, WidgetTree};
use incular_widgets::{Builder, Column, GestureDetector, GlobalKey, Padding, Semantics, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100.0, 100.0))
}

fn box_widget() -> Widget {
    Widget::box_(Size::new(10.0, 10.0), Color::WHITE)
}

fn padded(child: Widget) -> Widget {
    Padding::new(EdgeInsets::all(2.0), child).into()
}

#[test]
fn fresh_wrapper_around_shared_subtree_skips_build_layout_paint_and_semantics_work() {
    let shared: Widget = Column::new([
        box_widget().semantics(ExplicitSemantics::new(Role::Image).label("retained image")),
        box_widget(),
    ])
    .into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(padded(shared.clone())).expect("mount");
    tree.layout(constraints()).expect("initial layout");
    let before_paint = tree.paint();
    tree.update_semantics();
    let before = tree.diagnostics();
    let before_semantics = tree.semantics_diagnostics();
    let child = tree.children(root).expect("root child")[0];
    let before_bounds = tree.element_bounds(child);

    tree.mark_build(root).expect("mark parent");
    tree.update(root, padded(shared))
        .expect("unchanged wrapper");
    tree.layout(constraints()).expect("cached layout");
    let after_paint = tree.paint();
    tree.update_semantics();

    let after = tree.diagnostics();
    assert_eq!(after.rebuilds - before.rebuilds, 0);
    assert_eq!(after.child_list_scans - before.child_list_scans, 0);
    assert_eq!(after.layouts - before.layouts, 0);
    assert_eq!(after.paints - before.paints, 0);
    assert_eq!(tree.semantics_diagnostics(), before_semantics);
    assert_eq!(after_paint.commands(), before_paint.commands());
    assert_eq!(tree.children(root).expect("retained child"), &[child]);
    assert_eq!(tree.element_bounds(child), before_bounds);
    assert!(
        !tree.is_build_dirty(root),
        "the parent's update was consumed"
    );
}

#[test]
fn identical_parent_update_clears_only_its_own_build_mark() {
    let descriptor = padded(box_widget());
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor.clone()).expect("mount");
    let child = tree.children(root).expect("child")[0];
    tree.mark_build(root).expect("mark parent");
    tree.mark_build(child).expect("mark child");
    let before = tree.diagnostics();

    tree.update(root, descriptor).expect("identical descriptor");

    assert!(!tree.is_build_dirty(root));
    assert!(
        tree.is_build_dirty(child),
        "the child owns independent work"
    );
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().child_list_scans, before.child_list_scans);
}

#[test]
fn equal_fresh_leaf_update_clears_its_build_mark() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(box_widget()).expect("mount");
    tree.mark_build(root).expect("mark leaf");
    let before = tree.diagnostics();

    tree.update(root, box_widget()).expect("equal fresh leaf");

    assert!(!tree.is_build_dirty(root));
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
}

#[test]
fn fresh_wrapper_can_skip_a_deep_shared_subtree_without_recursive_equality() {
    const DEPTH: usize = 4_096;
    let mut shared = box_widget();
    for _ in 0..DEPTH {
        shared = padded(shared);
    }
    let mut tree = WidgetTree::new();
    let root = tree.mount(padded(shared.clone())).expect("mount deep tree");
    let before = tree.diagnostics();

    tree.update(root, padded(shared))
        .expect("skip deep shared tree");

    assert_eq!(tree.element_count(), DEPTH + 2);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().child_list_scans, before.child_list_scans);
    tree.verify_invariants()
        .expect("retained deep tree invariants");
}

#[test]
fn changed_wrapper_properties_still_update_layout_around_shared_children() {
    let child = box_widget();
    let mut tree = WidgetTree::new();
    let root = tree.mount(padded(child.clone())).expect("mount");
    tree.layout(constraints()).expect("initial layout");
    let child_id = tree.children(root).expect("child")[0];
    assert_eq!(tree.element_bounds(child_id).expect("bounds").origin.x, 2.0);

    tree.update(root, Padding::new(EdgeInsets::all(9.0), child).into())
        .expect("changed wrapper");
    tree.layout(constraints()).expect("updated layout");

    assert_eq!(tree.children(root).expect("retained child"), &[child_id]);
    assert_eq!(tree.element_bounds(child_id).expect("bounds").origin.x, 9.0);
}

#[test]
fn changed_semantic_metadata_is_not_hidden_by_shared_children() {
    let child = box_widget();
    let descriptor = |label: &str| {
        padded(child.clone()).semantics(ExplicitSemantics::new(Role::Image).label(label))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor("before")).expect("mount");
    tree.layout(constraints()).expect("initial layout");
    tree.update_semantics();
    let child_id = tree.children(root).expect("child")[0];

    tree.update(root, descriptor("after"))
        .expect("new semantic label");
    tree.layout(constraints()).expect("layout");
    tree.update_semantics();

    assert_eq!(tree.children(root).expect("retained child"), &[child_id]);
    assert!(
        tree.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("after"))
    );
    assert!(
        !tree
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("before"))
    );
}

#[test]
fn changed_gesture_callback_is_not_hidden_by_shared_children() {
    let child = box_widget();
    let calls = Rc::new(Cell::new(0));
    let descriptor = |value| {
        let calls = calls.clone();
        Widget::from(
            GestureDetector::new(child.clone()).callbacks(GestureCallbacks {
                on_tap: Some(Rc::new(move || calls.set(value))),
                ..GestureCallbacks::default()
            }),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor(1)).expect("mount");
    tree.semantic_action_callback(root, SemanticActionKind::Activate)
        .expect("initial gesture action")();
    assert_eq!(calls.get(), 1);
    let child_id = tree.children(root).expect("child")[0];

    tree.update(root, descriptor(2))
        .expect("new gesture callback");
    tree.semantic_action_callback(root, SemanticActionKind::Activate)
        .expect("replacement gesture action")();

    assert_eq!(calls.get(), 2);
    assert_eq!(tree.children(root).expect("retained child"), &[child_id]);
}

#[test]
fn changed_semantic_callback_is_not_hidden_by_shared_children() {
    let child = box_widget();
    let calls = Rc::new(Cell::new(0));
    let descriptor = |value| {
        let calls = calls.clone();
        Widget::from(Semantics::new(padded(child.clone())).on_tap(move || calls.set(value)))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor(1)).expect("mount");
    tree.semantic_action_callback(root, SemanticActionKind::Activate)
        .expect("initial semantic action")();
    assert_eq!(calls.get(), 1);

    tree.update(root, descriptor(2))
        .expect("new semantic callback");
    tree.semantic_action_callback(root, SemanticActionKind::Activate)
        .expect("replacement semantic action")();

    assert_eq!(calls.get(), 2);
}

#[test]
fn moving_the_same_global_keyed_descriptor_rebinds_its_inherited_dependencies() {
    let key = GlobalKey::new();
    let seen = Rc::new(Cell::new(0));
    let builds = Rc::new(Cell::new(0));
    let moving = {
        let seen = seen.clone();
        let builds = builds.clone();
        Widget::from(Builder::new(move |context| {
            seen.set(context.depend_on::<u32>().expect("inherited value"));
            builds.set(builds.get() + 1);
            box_widget()
        }))
        .with_key(key)
    };
    let descriptor = |left| {
        let (a, b) = if left {
            (vec![moving.clone()], Vec::new())
        } else {
            (Vec::new(), vec![moving.clone()])
        };
        Widget::from(Column::new([
            Widget::environment_scope(1_u32, Column::new(a).into()),
            Widget::environment_scope(2_u32, Column::new(b).into()),
        ]))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(descriptor(true)).expect("mount");
    tree.layout(constraints()).expect("initial layout");
    let element = tree
        .element_with_key(&Key::Global(key))
        .expect("keyed child");
    let render = tree.render_id(element);
    assert_eq!(seen.get(), 1);
    let before_builds = builds.get();

    tree.update(root, descriptor(false))
        .expect("move shared descriptor");
    tree.layout(constraints()).expect("layout after move");

    assert_eq!(tree.element_with_key(&Key::Global(key)), Some(element));
    assert_eq!(tree.render_id(element), render);
    assert_eq!(seen.get(), 2);
    assert!(
        builds.get() > before_builds,
        "moving invalidates inherited reads"
    );
    tree.verify_invariants()
        .expect("reparented tree invariants");
}
