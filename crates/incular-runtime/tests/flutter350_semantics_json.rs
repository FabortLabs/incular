use incular_config::TextDirection;
use incular_core::{ArenaId, Offset, Rect, Size};
use incular_runtime::{flutter_semantics_data_json, flutter_semantics_node_json};
use incular_semantics::{Role, SemanticActionKind, SemanticNode, SemanticNodeId, SemanticState};
use serde_json::json;

fn button() -> SemanticNode {
    SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(7, 3)),
        role: Role::Button,
        label: Some("Test Label".into()),
        value: Some("Test Value".into()),
        description: Some("Test Hint".into()),
        bounds: Rect::from_origin_size(Offset::ZERO, Size::new(100., 50.)),
        state: SemanticState {
            text_direction: Some(TextDirection::Ltr),
            ..SemanticState::default()
        },
        actions: Vec::new(),
        children: Vec::new(),
    }
}

#[test]
fn data_and_node_json_share_the_actual_retained_semantics_fields() {
    let node = button();
    let expected = json!({
        "label": "Test Label",
        "value": "Test Value",
        "hint": "Test Hint",
        "tooltip": "",
        "increasedValue": "",
        "decreasedValue": "",
        "flags": ["isButton"],
        "actions": [],
        "rect": { "left": 0.0, "top": 0.0, "width": 100.0, "height": 50.0 },
    });
    let data = flutter_semantics_data_json(&node);
    assert_eq!(data, expected);
    let mut node_json = flutter_semantics_node_json(&node, &[]);
    let fields = node_json.as_object_mut().unwrap();
    assert_eq!(fields.remove("id"), Some(json!((3_u64 << 32) | 7)));
    assert_eq!(fields.remove("childrenInTraversalOrder"), Some(json!([])));
    assert_eq!(fields.remove("childrenInHitTestOrder"), Some(json!([])));
    assert_eq!(node_json, expected);
}

#[test]
fn obscured_debug_values_are_not_published_and_actions_keep_protocol_order() {
    let mut node = button();
    node.role = Role::TextField;
    node.state.obscured = true;
    node.state.tooltip = Some("Password field".into());
    node.actions = vec![
        SemanticActionKind::Focus,
        SemanticActionKind::SetText,
        SemanticActionKind::Activate,
    ];
    let data = flutter_semantics_node_json(&node, &[]);
    assert_eq!(data["value"], "");
    assert_eq!(data["tooltip"], "Password field");
    assert_eq!(data["flags"], json!(["isTextField", "isObscured"]));
    assert_eq!(data["actions"], json!(["tap", "setText", "focus"]));
    assert!(!data.to_string().contains("Test Value"));
}
