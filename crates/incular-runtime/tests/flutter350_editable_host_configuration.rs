use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{FlutterTextInputConfiguration, FlutterTextInputControls};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 80.)))
        .unwrap();
}

fn host_options(config: &mut FlutterTextInputConfiguration) {
    // The embedder owns the delta opt-in; publishing framework updates must
    // not silently restore the native editor's defaults for host overrides.
    config.enable_delta_model = true;
}

#[test]
fn a_focused_editors_obscuring_and_view_changes_send_only_update_config() {
    let text = TextEditingController::with_text("Lorem");
    text.set_selection(TextSelection::collapsed(5));
    let focus = FocusNode::new();
    let build = |obscured| {
        EditableText::new(text.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .obscure_text(obscured)
    };
    let mut runtime = Runtime::new(build(true).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime.set_flutter_text_input_view_id(Some(14));
    frame(&mut runtime);
    let field = runtime.focused_element().unwrap();
    let (id, mut initial) = runtime.flutter_text_input_configuration().unwrap();
    host_options(&mut initial);
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(id, initial, |_| {}));
    let first = controls.take_platform_calls();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].method, "TextInput.setClient");
    assert_eq!(first[0].arguments[1]["viewId"], 14);
    assert_eq!(first[0].arguments[1]["obscureText"], true);
    assert_eq!(first[0].arguments[1]["enableDeltaModel"], true);
    assert!(
        !runtime
            .publish_flutter_text_input_configuration(&mut controls, host_options)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());

    runtime.set_flutter_text_input_view_id(Some(15));
    frame(&mut runtime);
    assert!(
        runtime
            .publish_flutter_text_input_configuration(&mut controls, host_options)
            .unwrap()
    );
    let update = controls.take_platform_calls();
    assert_eq!(update.len(), 1);
    assert_eq!(update[0].method, "TextInput.updateConfig");
    assert_eq!(update[0].arguments["viewId"], 15);
    assert_eq!(update[0].arguments["obscureText"], true);
    assert_eq!(update[0].arguments["enableDeltaModel"], true);

    runtime.schedule_update(field, build(false).into()).unwrap();
    frame(&mut runtime);
    assert!(
        runtime
            .publish_flutter_text_input_configuration(&mut controls, host_options)
            .unwrap()
    );
    let update = controls.take_platform_calls();
    assert_eq!(update.len(), 1);
    assert_eq!(update[0].method, "TextInput.updateConfig");
    assert_eq!(update[0].arguments["viewId"], 15);
    assert_eq!(update[0].arguments["obscureText"], false);
    assert_eq!(update[0].arguments["enableDeltaModel"], true);

    assert!(controls.is_attached(id));
    assert_eq!(runtime.flutter_text_input_configuration().unwrap().0, id);
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(focus.has_primary_focus());
    assert_eq!(text.selection(), TextSelection::collapsed(5));
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_configuration(&mut controls, host_options)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
}
