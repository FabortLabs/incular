use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, KeyEventResult,
    NextFocusAction, NextFocusIntent, PreviousFocusAction, PreviousFocusIntent, SizedBox,
    TraversalEdgeBehavior,
};

fn fixture() -> (Runtime, FocusNode, FocusNode) {
    let scope = FocusScopeNode::new();
    scope.set_traversal_edge_behavior(TraversalEdgeBehavior::Stop);
    let a = FocusNode::new();
    let b = FocusNode::new();
    let child = |node: &FocusNode| Focus::new(SizedBox::square(20.)).node(node.clone());
    let root = Directionality::new(
        TextDirection::Ltr,
        FocusScope::new(Column::new([child(&a), child(&b)])).node(scope),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    (runtime, a, b)
}

#[test]
fn next_focus_converts_the_real_forward_result_and_stops_at_the_edge() {
    let (mut runtime, a, b) = fixture();
    let action = NextFocusAction;
    let intent = NextFocusIntent;
    assert_eq!(
        action.to_key_event_result(&intent, true),
        KeyEventResult::Handled
    );
    assert_eq!(
        action.to_key_event_result(&intent, false),
        KeyEventResult::SkipRemainingHandlers
    );
    a.request_focus();
    runtime
        .run_frame(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    assert!(a.has_primary_focus());
    assert_eq!(
        action.to_key_event_result(&intent, action.invoke(&mut runtime, &intent)),
        KeyEventResult::Handled
    );
    assert!(b.has_primary_focus());
    assert_eq!(
        action.to_key_event_result(&intent, action.invoke(&mut runtime, &intent)),
        KeyEventResult::SkipRemainingHandlers
    );
    assert!(b.has_primary_focus());
}

#[test]
fn previous_focus_converts_the_real_reverse_result_and_stops_at_the_edge() {
    let (mut runtime, a, b) = fixture();
    let action = PreviousFocusAction;
    let intent = PreviousFocusIntent;
    assert_eq!(
        action.to_key_event_result(&intent, true),
        KeyEventResult::Handled
    );
    assert_eq!(
        action.to_key_event_result(&intent, false),
        KeyEventResult::SkipRemainingHandlers
    );
    b.request_focus();
    runtime
        .run_frame(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    assert!(b.has_primary_focus());
    assert_eq!(
        action.to_key_event_result(&intent, action.invoke(&mut runtime, &intent)),
        KeyEventResult::Handled
    );
    assert!(a.has_primary_focus());
    assert_eq!(
        action.to_key_event_result(&intent, action.invoke(&mut runtime, &intent)),
        KeyEventResult::SkipRemainingHandlers
    );
    assert!(a.has_primary_focus());
}
