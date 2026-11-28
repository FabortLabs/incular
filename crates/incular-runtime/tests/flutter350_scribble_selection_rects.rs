use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{FlutterTextInputControls, TextInputClientId};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{EditableText, FocusNode, Widget};
use serde_json::{Value, json};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn show(runtime: &Runtime, controls: &mut FlutterTextInputControls) -> TextInputClientId {
    let (client, config) = runtime.flutter_text_input_configuration().unwrap();
    assert!(controls.attach_existing(client, config, |_| {}));
    assert!(runtime.show_flutter_text_input(client, controls).unwrap());
    client
}

fn selection_rects(controls: &mut FlutterTextInputControls) -> Vec<Value> {
    controls
        .take_platform_calls()
        .into_iter()
        .filter(|call| call.method == "TextInput.setSelectionRects")
        .map(|call| call.arguments)
        .collect()
}

#[test]
fn selection_rects_are_sent_once_per_ios_focus_connection() {
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    incular_text::register_application_font_as("ScribbleFlutterTest", std::fs::read(font).unwrap());
    let controller = TextEditingController::with_text("Text1");
    let focus = FocusNode::new();
    let editor = Widget::from(
        EditableText::new(controller)
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None)
            .style(
                TextStyle::new()
                    .font_size(14.)
                    .font_family("ScribbleFlutterTest"),
            )
            .cursor_width(0.),
    );
    let mut runtime = Runtime::new(editor).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let mut controls = FlutterTextInputControls::new(false);
    frame(&mut runtime);
    assert!(runtime.flutter_text_input_configuration().is_none());
    assert!(
        !runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    assert!(selection_rects(&mut controls).is_empty());

    let expected = vec![json!([
        [0., 0., 14., 14., 0, 1],
        [14., 0., 14., 14., 1, 1],
        [28., 0., 14., 14., 2, 1],
        [42., 0., 14., 14., 3, 1],
        [56., 0., 14., 14., 4, 1]
    ])];
    focus.request_focus();
    frame(&mut runtime);
    let first_client = show(&runtime, &mut controls);
    assert_eq!(selection_rects(&mut controls), expected);
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    assert!(selection_rects(&mut controls).is_empty());

    focus.unfocus();
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    assert!(selection_rects(&mut controls).is_empty());
    assert!(controls.close(first_client));

    focus.request_focus();
    frame(&mut runtime);
    let second_client = show(&runtime, &mut controls);
    assert_ne!(first_client, second_client);
    assert_eq!(selection_rects(&mut controls), expected);
}
