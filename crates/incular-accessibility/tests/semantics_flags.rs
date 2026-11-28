//! Flutter semantics flags and properties without a dedicated role reach
//! AccessKit: toggled, hidden, live region, tooltip, link URL, identifier.

use accesskit::{Live, NodeId, Toggled};
use incular_accessibility::{
    AccessKitProjection, SemanticNode, SemanticNodeId, SemanticState, SemanticsTree,
};
use incular_core::{ArenaId, Rect};
use incular_semantics::{CheckedState, Role};

fn project(role: Role, state: SemanticState) -> accesskit::Node {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role,
        label: Some("node".into()),
        value: None,
        description: None,
        bounds: Rect::default(),
        state,
        actions: vec![],
        children: vec![],
    });
    tree.set_root(Some(id));
    let mut projection = AccessKitProjection::new();
    projection.activate();
    let update = projection
        .sync(&tree, 1.0)
        .expect("update")
        .into_accesskit();
    let native = NodeId(projection.native_node_id(id).expect("native id"));
    update
        .nodes
        .into_iter()
        .find(|(id, _)| *id == native)
        .expect("node")
        .1
}

#[test]
fn defaults_leave_the_flags_unset() {
    let node = project(Role::Button, SemanticState::default());
    assert_eq!(node.toggled(), None);
    assert!(!node.is_hidden());
    assert_eq!(node.live(), None);
    assert_eq!(node.tooltip(), None);
    assert_eq!(node.url(), None);
    assert_eq!(node.author_id(), None);
}

#[test]
fn flags_and_properties_are_projected() {
    let node = project(
        Role::Link,
        SemanticState {
            toggled: Some(true),
            hidden: true,
            live_region: true,
            tooltip: Some("Opens the docs".into()),
            link_url: Some("https://example.com/docs".into()),
            identifier: Some("docs-link".into()),
            ..SemanticState::default()
        },
    );
    assert_eq!(node.toggled(), Some(Toggled::True));
    assert!(node.is_hidden());
    // Flutter's live regions are polite (Android `ACCESSIBILITY_LIVE_REGION_POLITE`).
    assert_eq!(node.live(), Some(Live::Polite));
    assert_eq!(node.tooltip(), Some("Opens the docs"));
    assert_eq!(node.url(), Some("https://example.com/docs"));
    assert_eq!(node.author_id(), Some("docs-link"));
}

#[test]
fn a_checked_state_wins_over_toggled() {
    let node = project(
        Role::Checkbox,
        SemanticState {
            checked: Some(CheckedState::Indeterminate),
            toggled: Some(false),
            ..SemanticState::default()
        },
    );
    assert_eq!(node.toggled(), Some(Toggled::Mixed));
}

#[test]
fn typed_validation_projects_invalid_without_confusing_valid_or_unspecified() {
    use incular_semantics::SemanticValidationResult::{Invalid, None, Valid};

    for (validation_result, legacy_invalid, expected) in [
        (None, false, false),
        (Valid, false, false),
        (Invalid, false, true),
        (Valid, true, true),
    ] {
        let node = project(
            Role::TextField,
            SemanticState {
                validation_result,
                invalid: legacy_invalid,
                ..SemanticState::default()
            },
        );
        assert_eq!(node.invalid().is_some(), expected);
    }
}

#[test]
fn selection_presence_updates_one_stable_native_node() {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role: Role::Tab,
        label: Some("existing tab".into()),
        value: None,
        description: None,
        bounds: Rect::default(),
        state: SemanticState::default(),
        actions: vec![],
        children: vec![],
    });
    tree.set_root(Some(id));
    let mut projection = AccessKitProjection::new();
    projection.activate();
    let mut stable = None;
    for (index, declared) in [None, Some(false), Some(true), None]
        .into_iter()
        .enumerate()
    {
        if index != 0 {
            let mut updated = tree.node(id).unwrap().clone();
            updated.state.has_selected_state = declared.is_some();
            updated.state.selected = declared.unwrap_or(false);
            assert!(tree.update(id, updated), "the declaration itself is dirty");
        }
        let update = projection
            .sync(&tree, 1.)
            .expect("native selection update")
            .into_accesskit();
        let native = NodeId(projection.native_node_id(id).unwrap());
        if let Some(previous) = stable {
            assert_eq!(native, previous);
        }
        stable = Some(native);
        let (_, node) = update.nodes.iter().find(|(id, _)| *id == native).unwrap();
        assert_eq!(node.is_selected(), declared);
    }
}

#[test]
fn menu_bar_and_checkable_menu_items_keep_their_distinct_native_roles() {
    use accesskit::Role as NativeRole;
    assert_eq!(
        project(Role::MenuBar, SemanticState::default()).role(),
        NativeRole::MenuBar
    );
    for (role, expected, checked) in [
        (Role::MenuItemCheckbox, NativeRole::MenuItemCheckBox, false),
        (Role::MenuItemRadio, NativeRole::MenuItemRadio, true),
    ] {
        let node = project(
            role,
            SemanticState {
                checked: Some(checked.into()),
                ..SemanticState::default()
            },
        );
        assert_eq!(node.role(), expected);
        assert_eq!(
            node.toggled(),
            Some(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
        );
    }
}
