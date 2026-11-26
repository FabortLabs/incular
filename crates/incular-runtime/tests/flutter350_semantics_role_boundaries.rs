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

fn mounted(child: Semantics) -> Runtime {
    let mut runtime = Runtime::new(child.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

#[test]
fn an_alert_dialog_keeps_its_role_inside_a_dialog() {
    let inner = Semantics::new(Placeholder::new()).role(Role::AlertDialog);
    let runtime = mounted(Semantics::new(inner).role(Role::Dialog));
    let tree = runtime.tree().semantics();
    let (id, dialog) = tree
        .iter()
        .find(|(_, node)| node.role == Role::Dialog)
        .unwrap();
    let (alert_id, _) = tree
        .iter()
        .find(|(_, node)| node.role == Role::AlertDialog)
        .unwrap();
    assert_ne!(id, alert_id);
    assert_eq!(dialog.children, [alert_id]);
}

fn assert_single_root_role(view: Semantics, role: Role) {
    let runtime = mounted(view);
    let tree = runtime.tree().semantics();
    assert_eq!(tree.roots().len(), 1);
    assert_eq!(tree.node(tree.roots()[0]).unwrap().role, role);
}

#[test]
fn time_sensitive_information_keeps_the_alert_role() {
    assert_single_root_role(
        Semantics::new(Placeholder::new()).role(Role::Alert),
        Role::Alert,
    );
}

#[test]
fn advisory_information_keeps_the_status_role() {
    assert_single_root_role(
        Semantics::new(Placeholder::new()).role(Role::Status),
        Role::Status,
    );
}

#[test]
fn a_childless_form_keeps_its_declared_role() {
    assert_single_root_role(
        Semantics::default().role(Role::Form).container(true),
        Role::Form,
    );
}

#[test]
fn a_list_and_its_item_keep_distinct_role_nodes() {
    let item = Semantics::new(Placeholder::new())
        .role(Role::ListItem)
        .container(true);
    let runtime = mounted(Semantics::new(item).role(Role::List).container(true));
    let tree = runtime.tree().semantics();
    let list = tree.node(tree.roots()[0]).unwrap();
    assert_eq!(list.role, Role::List);
    assert_eq!(list.children.len(), 1);
    assert_eq!(tree.node(list.children[0]).unwrap().role, Role::ListItem);
}
