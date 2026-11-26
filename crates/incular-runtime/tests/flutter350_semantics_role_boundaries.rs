//! Flutter 3.50.0 basic semantics: roles cannot absorb different child roles.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticAnnotationFlag};
use incular_widgets::{Placeholder, Semantics};

fn dialog_keeps_child_annotation(
    child: Semantics,
    annotation: SemanticAnnotationFlag,
    role: Option<Role>,
) {
    let root = Semantics::new(child).role(Role::Dialog);
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let tree = runtime.tree().semantics();
    let (dialog_id, dialog) = tree
        .iter()
        .find(|(_, node)| node.role == Role::Dialog)
        .unwrap();
    let (child_id, child) = tree
        .iter()
        .find(|(_, node)| node.state.annotation_flags.get(annotation) == Some(true))
        .unwrap();
    assert_ne!(
        dialog_id, child_id,
        "child annotation must not merge into the dialog"
    );
    assert_eq!(dialog.children, [child_id]);
    assert_ne!(dialog.state.annotation_flags.get(annotation), Some(true));
    if let Some(role) = role {
        assert_eq!(child.role, role);
    }
}

#[test]
fn dialog_and_text_field_have_two_distinct_nodes() {
    dialog_keeps_child_annotation(
        Semantics::new(Placeholder::new()).text_field(true),
        SemanticAnnotationFlag::TextField,
        Some(Role::TextField),
    );
}

#[test]
fn dialog_and_link_have_two_distinct_nodes() {
    dialog_keeps_child_annotation(
        Semantics::new(Placeholder::new()).link(true),
        SemanticAnnotationFlag::Link,
        Some(Role::Link),
    );
}

#[test]
fn dialog_and_image_have_two_distinct_nodes() {
    dialog_keeps_child_annotation(
        Semantics::new(Placeholder::new()).image(true),
        SemanticAnnotationFlag::Image,
        Some(Role::Image),
    );
}

#[test]
fn dialog_and_keyboard_key_have_two_distinct_nodes() {
    dialog_keeps_child_annotation(
        Semantics::new(Placeholder::new()).keyboard_key(true),
        SemanticAnnotationFlag::KeyboardKey,
        None,
    );
}
