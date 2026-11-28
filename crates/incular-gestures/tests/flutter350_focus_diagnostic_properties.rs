use std::rc::Rc;

use incular_core::{DiagnosticLevel, DiagnosticsNode, DiagnosticsSerializationOptions};
use incular_gestures::{FocusNode, FocusScopeNode, TraversalEdgeBehavior};

fn descriptions(nodes: &[DiagnosticsNode]) -> Vec<String> {
    nodes.iter().map(ToString::to_string).collect()
}

#[test]
fn detached_node_and_scope_report_the_same_six_properties_in_reference_order() {
    let node = FocusNode::new();
    node.set_debug_label(Some(Rc::from("Label")));
    let scope = FocusScopeNode::new();
    scope
        .scope_focus_node()
        .set_debug_label(Some(Rc::from("Scope Label")));
    for properties in [
        node.diagnostic_properties(None),
        scope.diagnostic_properties(None),
    ] {
        assert_eq!(
            descriptions(&properties),
            [
                "context: null",
                "descendantsAreFocusable: true",
                "descendantsAreTraversable: true",
                "canRequestFocus: true",
                "hasFocus: false",
                "hasPrimaryFocus: false",
            ]
        );
        let tree = DiagnosticsNode::new("focus", "FocusNode").with_properties(properties);
        assert_eq!(tree.to_string_deep(DiagnosticLevel::Debug), "focus\n");
    }
    assert!(node.to_string_short().ends_with("(Label)"));
    assert!(
        scope
            .scope_focus_node()
            .to_string_short()
            .starts_with("FocusScopeNode#")
    );
}

#[test]
fn focus_diagnostics_report_live_changes_and_omit_stale_scope_history() {
    let scope = FocusScopeNode::new();
    scope.set_traversal_edge_behavior(TraversalEdgeBehavior::Stop);
    let anchor = scope.scope_focus_node();
    let first = FocusNode::new();
    let second = FocusNode::new();
    first.set_debug_label(Some(Rc::from("first")));
    second.set_debug_label(Some(Rc::from("second")));
    first.set_host_enclosing_scope(Some(&scope));
    second.set_host_enclosing_scope(Some(&scope));
    scope.record_host_focused_child(&first);
    scope.record_host_focused_child(&second);
    assert!(second.set_host_focus_state_silent(true, false));
    assert!(anchor.set_host_focus_state_silent(false, true));
    scope.set_descendants_are_traversable(false);

    let properties = scope.diagnostic_properties(Some("FocusScope"));
    assert_eq!(
        descriptions(&properties),
        [
            "context: FocusScope".to_owned(),
            "descendantsAreFocusable: true".to_owned(),
            "DESCENDANTS UNTRAVERSABLE".to_owned(),
            "canRequestFocus: true".to_owned(),
            "IN FOCUS PATH".to_owned(),
            "hasPrimaryFocus: false".to_owned(),
            format!(
                "focusedChildren: {}, {}",
                second.to_string_short(),
                first.to_string_short()
            ),
            "traversalEdgeBehavior: TraversalEdgeBehavior.stop".to_owned(),
        ]
    );
    assert!(anchor.to_string_short().ends_with("([IN FOCUS PATH])"));
    assert!(
        second
            .to_string_short()
            .ends_with("(second [PRIMARY FOCUS])")
    );
    assert_eq!(
        descriptions(&second.diagnostic_properties(Some("Focus")))[5],
        "PRIMARY FOCUS"
    );

    second.set_host_enclosing_scope(None);
    let retained = scope.diagnostic_properties(Some("FocusScope"));
    assert_eq!(
        retained[6].to_string(),
        format!("focusedChildren: {}", first.to_string_short())
    );
    let snapshot = retained[6].to_json(DiagnosticsSerializationOptions::default(), &());
    let values = snapshot["values"].as_array().unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].as_str(), Some(first.to_string_short().as_str()));
    drop(first);
    assert_eq!(scope.diagnostic_properties(None).len(), 6);
}
