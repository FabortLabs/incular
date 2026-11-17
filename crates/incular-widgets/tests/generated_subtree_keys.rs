//! Generated sibling validation must account for keys inside skipped subtrees.

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::{Key, TreeError, WidgetTree};
use incular_widgets::{GlobalKey, ListWheelViewport, Padding, WheelChildDelegate, Widget};

fn frame() -> Constraints {
    Constraints::tight(Size::new(100.0, 100.0))
}

fn leaf() -> Widget {
    Widget::box_(Size::new(10.0, 10.0), Color::WHITE)
}

fn padded(child: Widget, padding: f32) -> Widget {
    Padding::new(EdgeInsets::all(padding), child).into()
}

fn wheel(controller: &ScrollController, children: Vec<Widget>) -> Widget {
    ListWheelViewport::new(
        controller.clone(),
        20.0,
        WheelChildDelegate::children(children),
    )
    .into()
}

fn duplicate_key_is_rejected_before_retaking_a_skipped_descendant(reuse_wrapper: bool) {
    let key = GlobalKey::new();
    let shared = leaf().with_key(key);
    let first = padded(shared.clone(), 2.0);
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(wheel(&controller, vec![first.clone(), padded(leaf(), 2.0)]))
        .expect("mount wheel");
    tree.layout(frame()).expect("materialize valid children");
    let before_children = tree.children(root).expect("wheel children").to_vec();
    assert_eq!(before_children.len(), 2);
    let before_descendants: Vec<_> = before_children
        .iter()
        .map(|&child| tree.children(child).expect("wrapper child").to_vec())
        .collect();
    let keyed = tree
        .element_with_key(&Key::Global(key))
        .expect("original keyed child");
    let original_parent = tree.parent(keyed);
    let before = tree.diagnostics();

    // Each generated child is individually valid. Together they repeat a
    // nested global key. The unchanged first branch would skip reconciliation
    // and must not let the second branch take its retained child away.
    let next_first = if reuse_wrapper {
        first
    } else {
        padded(shared.clone(), 2.0)
    };
    tree.update(
        root,
        wheel(&controller, vec![next_first, padded(shared, 2.0)]),
    )
    .expect("generated children validate during layout");
    let error = tree
        .layout(frame())
        .expect_err("duplicate key across generated siblings must fail");
    let TreeError::InvalidGeneratedChild { source, .. } = error else {
        panic!("expected a contextual generated-child failure: {error:?}");
    };
    assert!(matches!(
        *source,
        TreeError::DuplicateGlobalKey { key: Key::Global(found) } if found == key
    ));
    assert_eq!(
        tree.children(root).expect("retained children"),
        before_children.as_slice()
    );
    for (&child, descendants) in before_children.iter().zip(&before_descendants) {
        assert_eq!(
            tree.children(child).expect("retained wrapper"),
            descendants.as_slice()
        );
    }
    assert_eq!(tree.element_with_key(&Key::Global(key)), Some(keyed));
    assert_eq!(tree.parent(keyed), original_parent);
    assert_eq!(tree.diagnostics().mounts, before.mounts);
    assert_eq!(tree.diagnostics().unmounts, before.unmounts);
    tree.verify_invariants()
        .expect("failed update kept ownership");
}

#[test]
fn fresh_generated_wrapper_cannot_hide_a_duplicate_global_key() {
    duplicate_key_is_rejected_before_retaking_a_skipped_descendant(false);
}

#[test]
fn identical_generated_wrapper_cannot_hide_a_duplicate_global_key() {
    duplicate_key_is_rejected_before_retaking_a_skipped_descendant(true);
}

#[test]
fn generated_siblings_can_reuse_local_keys_in_separate_subtrees() {
    let controller = ScrollController::new();
    let globally_keyed = leaf().with_key(GlobalKey::new());
    let shared = leaf().with_key(Key::Value(7));
    let children = |padding| {
        vec![
            // Subsequent key-free branches finish validation with an already
            // nonempty global-key set; they remain independently cacheable.
            padded(globally_keyed.clone(), padding),
            padded(shared.clone(), padding),
            padded(shared.clone(), padding),
        ]
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(wheel(&controller, children(2.0)))
        .expect("mount wheel");
    tree.layout(frame()).expect("distinct local-key scopes");
    let wrappers = tree.children(root).expect("wheel children").to_vec();
    assert_eq!(wrappers.len(), 3);
    let first = tree.children(wrappers[1]).expect("first local child")[0];
    let second = tree.children(wrappers[2]).expect("second local child")[0];
    assert_ne!(first, second, "local keys never alias between parents");

    tree.update(root, wheel(&controller, children(3.0)))
        .expect("update wrappers");
    tree.layout(frame()).expect("local keys remain independent");

    assert_eq!(
        tree.children(root).expect("retained wrappers"),
        wrappers.as_slice()
    );
    assert_eq!(
        tree.children(wrappers[1]).expect("first retained child"),
        &[first]
    );
    assert_eq!(
        tree.children(wrappers[2]).expect("second retained child"),
        &[second]
    );
    tree.verify_invariants()
        .expect("local-key scopes remain valid");
}
