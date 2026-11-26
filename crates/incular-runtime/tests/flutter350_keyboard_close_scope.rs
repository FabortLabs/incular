use incular_config::Constraints;
use incular_core::Size;
use incular_platform::TextInputCommand;
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    Column, EditableText, FocusNode, FocusScope, FocusScopeNode, TextDirection, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap();
}

#[test]
fn host_keyboard_close_clears_scope_history_and_focuses_that_scope() {
    let scope = FocusScopeNode::new();
    let [one, two] = [FocusNode::new(), FocusNode::new()];
    let editor = |node: &FocusNode, text| -> Widget {
        EditableText::new(TextEditingController::with_text(text))
            .focus_node(node.clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(180., 40.))
            .into()
    };
    let content = Column::new([editor(&one, "one"), editor(&two, "two")]);
    let mut runtime = Runtime::new(FocusScope::new(content).node(scope.clone()).into()).unwrap();
    frame(&mut runtime);
    let _ = one.request_focus();
    frame(&mut runtime);
    assert!(one.has_primary_focus());
    runtime.take_text_input_commands();
    let _ = two.request_focus();
    frame(&mut runtime);
    assert!(two.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(two.clone()));
    let old_client = runtime
        .take_text_input_commands()
        .iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("second client attached");

    assert!(runtime.text_input_connection_closed(old_client));
    frame(&mut runtime);
    assert!(!one.has_focus());
    assert!(!two.has_focus());
    assert!(scope.has_primary_focus());
    assert_eq!(scope.focused_child(), None);
    assert!(runtime.take_text_input_commands().is_empty());

    let _ = one.request_focus();
    frame(&mut runtime);
    assert!(one.has_primary_focus());
    assert!(!scope.has_primary_focus());
    assert!(runtime.take_text_input_commands().iter().any(|command| {
        matches!(command, TextInputCommand::SetClient { configuration, state, .. }
            if configuration.client != old_client && state.text == "one")
    }));
}
