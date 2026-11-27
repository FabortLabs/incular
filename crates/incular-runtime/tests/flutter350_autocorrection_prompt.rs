use incular_config::{Constraints, TextDirection};
use incular_core::{Color, InputEvent, Rect, Size};
use incular_platform::{TextInputClientId, TextInputCommand};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::{FlutterTextInputLifecycleError, Runtime};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode};
use serde_json::json;

const PROMPT: Color = Color::rgba(255, 0, 0, 255);

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 40.)))
        .unwrap()
        .0
}

fn mount(
    text: &str,
    color: Option<Color>,
) -> (Runtime, TextEditingController, FocusNode, TextInputClientId) {
    let controller = TextEditingController::with_text(text);
    let focus = FocusNode::new();
    let mut editor = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .show_cursor(false)
        .size(Size::new(180., 40.));
    if let Some(color) = color {
        editor = editor.autocorrection_text_rect_color(color);
    }
    let mut runtime = Runtime::new(editor.into()).unwrap();
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap();
    (runtime, controller, focus, client)
}

fn show(
    runtime: &mut Runtime,
    client: TextInputClientId,
    start: usize,
    end: usize,
) -> Result<Option<bool>, FlutterTextInputLifecycleError> {
    let message = json!({"method": "TextInputClient.showAutocorrectionPromptRect", "args": [client.get(), start, end]});
    runtime.handle_flutter_autocorrection_prompt_call(message.to_string().as_bytes())
}

fn prompt_rect(list: &DisplayList) -> Option<Rect> {
    list.commands().iter().find_map(|command| match command {
        PaintCommand::Rect { rect, color } if *color == PROMPT => Some(*rect),
        _ => None,
    })
}

#[test]
fn an_autocorrection_prompt_appears_only_on_demand_and_clears_on_edit_or_blur() {
    let (mut runtime, controller, focus, client) = mount("ABCDEFG", Some(PROMPT));
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
    let before = controller.selection();

    assert_eq!(show(&mut runtime, client, 0, 1).unwrap(), Some(true));
    let first = prompt_rect(&frame(&mut runtime)).unwrap();
    assert!(first.size.width > 0.);
    assert!(first.size.height > 0.);
    assert_eq!(controller.selection(), before);

    controller.set_selection(TextSelection::new(0, controller.text().len()));
    let _ = runtime.handle_input(InputEvent::Text("12345".into()));
    assert_eq!(controller.text(), "12345");
    assert!(prompt_rect(&frame(&mut runtime)).is_none());

    assert_eq!(show(&mut runtime, client, 0, 1).unwrap(), Some(true));
    assert!(prompt_rect(&frame(&mut runtime)).is_some());
    focus.unfocus();
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
    assert!(!focus.has_focus());
}

#[test]
fn prompt_range_is_a_utf16_span_and_requires_an_explicit_color_to_paint() {
    let (mut runtime, _, _, client) = mount("A😀B", Some(PROMPT));
    assert!(matches!(
        show(&mut runtime, client, 1, 2),
        Err(FlutterTextInputLifecycleError::InvalidPromptRange)
    ));
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
    assert_eq!(
        show(&mut runtime, TextInputClientId::new(client.get() + 1), 1, 3).unwrap(),
        Some(false)
    );
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
    assert_eq!(show(&mut runtime, client, 1, 3).unwrap(), Some(true));
    assert!(prompt_rect(&frame(&mut runtime)).is_some());

    let (mut runtime, _, _, client) = mount("A😀B", None);
    assert_eq!(show(&mut runtime, client, 1, 3).unwrap(), Some(true));
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
}

#[test]
fn replacing_the_editing_controller_cannot_keep_the_old_prompt() {
    let (mut runtime, _, focus, client) = mount("original", Some(PROMPT));
    assert_eq!(show(&mut runtime, client, 0, 1).unwrap(), Some(true));
    assert!(prompt_rect(&frame(&mut runtime)).is_some());

    let replacement = TextEditingController::with_text("replaced");
    let editor = EditableText::new(replacement.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .show_cursor(false)
        .autocorrection_text_rect_color(PROMPT)
        .size(Size::new(180., 40.));
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, editor.into()).unwrap();
    assert!(prompt_rect(&frame(&mut runtime)).is_none());
    assert_eq!(replacement.text(), "replaced");
}
