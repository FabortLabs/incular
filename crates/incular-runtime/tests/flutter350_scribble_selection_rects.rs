use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{FlutterTextInputControls, TextInputClientId};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAlign, TextEditingController, TextStyle};
use incular_widgets::{Align, ConstrainedBox, EditableText, FocusNode, Widget};
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

fn configured_editor(
    controller: &TextEditingController,
    focus: &FocusNode,
    size: (Option<f32>, Option<f32>),
    align: TextAlign,
    scroll: Option<f32>,
    enabled: bool,
) -> Widget {
    let field = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .max_lines(None)
        .text_align(align)
        .style(
            TextStyle::new()
                .font_size(14.)
                .font_family("ScribbleFlutterTest"),
        )
        .cursor_width(0.)
        .stylus_handwriting_enabled(enabled)
        .fixed_scroll_offset(scroll)
        .allow_implicit_caret_scrolling(false);
    let constraints = Constraints::new(
        size.0.unwrap_or(0.),
        size.0.unwrap_or(f32::INFINITY),
        size.1.unwrap_or(0.),
        size.1.unwrap_or(f32::INFINITY),
    );
    Align::new(Alignment::CENTER, ConstrainedBox::new(constraints, field)).into()
}

fn register_test_font() -> bool {
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return false;
    };
    incular_text::register_application_font_as("ScribbleFlutterTest", std::fs::read(font).unwrap());
    true
}

fn horizontal(y: f32) -> Vec<Value> {
    vec![Value::Array(
        (0..5)
            .map(|i| json!([14. * i as f32, y, 14., 14., i, 1]))
            .collect(),
    )]
}

fn vertical(x: f32, y: f32, count: usize) -> Vec<Value> {
    vec![Value::Array(
        (0..count)
            .map(|i| json!([x, y + 14. * i as f32, 14., 14., i, 1]))
            .collect(),
    )]
}

#[test]
fn disabled_desktop_and_browser_editors_do_not_send_native_handwriting_rectangles() {
    for (platform, web_protocol, enabled) in [
        (TextShortcutPlatform::Ios, false, false),
        (TextShortcutPlatform::Ios, true, true),
        (TextShortcutPlatform::Android, false, true),
        (TextShortcutPlatform::MacOs, false, true),
    ] {
        let controller = TextEditingController::with_text("Text1");
        let focus = FocusNode::new();
        let editor = configured_editor(
            &controller,
            &focus,
            (None, None),
            TextAlign::Start,
            None,
            enabled,
        );
        let mut runtime = Runtime::new(editor).unwrap();
        runtime.set_text_shortcut_platform(platform);
        focus.request_focus();
        frame(&mut runtime);
        let mut controls = FlutterTextInputControls::new(web_protocol);
        show(&runtime, &mut controls);
        let calls = controls.take_platform_calls();
        assert!(calls.iter().any(|call| call.method == "TextInput.show"));
        assert!(
            !calls
                .iter()
                .any(|call| call.method == "TextInput.setSelectionRects")
        );
        frame(&mut runtime);
        assert!(
            !runtime
                .publish_flutter_text_input_rects(&mut controls)
                .unwrap()
        );
        assert!(selection_rects(&mut controls).is_empty());
    }
}

#[test]
fn rectangle_intersection_keeps_shapes_that_span_both_vertical_edges() {
    if !register_test_font() {
        return;
    }
    let controller = TextEditingController::with_text("Text1");
    let focus = FocusNode::new();
    let editor = configured_editor(
        &controller,
        &focus,
        (None, Some(13.)),
        TextAlign::Start,
        Some(0.5),
        true,
    );
    let mut runtime = Runtime::new(editor).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    focus.request_focus();
    frame(&mut runtime);
    let mut controls = FlutterTextInputControls::new(false);
    show(&runtime, &mut controls);
    assert_eq!(selection_rects(&mut controls), horizontal(-0.5));
}

#[test]
fn resized_rewrapped_clipped_and_scrolled_editor_updates_the_same_client() {
    if !register_test_font() {
        return;
    }
    let controller = TextEditingController::with_text("Text1");
    let focus = FocusNode::new();
    let editor = |width, height, align, scroll| {
        configured_editor(&controller, &focus, (width, height), align, scroll, true)
    };
    let mut runtime = Runtime::new(editor(None, None, TextAlign::Start, None)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    focus.request_focus();
    frame(&mut runtime);
    let mut controls = FlutterTextInputControls::new(false);
    let client = show(&runtime, &mut controls);
    assert_eq!(selection_rects(&mut controls), horizontal(0.));

    let publish = |runtime: &mut Runtime, controls: &mut FlutterTextInputControls, widget| {
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, widget).unwrap();
        frame(runtime);
        assert_eq!(
            runtime.flutter_text_input_configuration().unwrap().0,
            client
        );
        runtime.publish_flutter_text_input_rects(controls).unwrap();
        selection_rects(controls)
    };
    assert!(
        publish(
            &mut runtime,
            &mut controls,
            editor(None, None, TextAlign::Start, None)
        )
        .is_empty()
    );

    assert_eq!(
        publish(
            &mut runtime,
            &mut controls,
            editor(Some(20.), None, TextAlign::Start, None)
        ),
        vertical(0., 0., 5),
    );

    controller.set_text("Text1👨‍👩‍👦");
    frame(&mut runtime);
    assert!(
        runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    let edited = selection_rects(&mut controls);
    assert_eq!(edited.len(), 1);
    let edited = edited[0].as_array().unwrap();
    let initial = vertical(0., 0., 5);
    assert_eq!(&edited[..5], initial[0].as_array().unwrap());
    assert_eq!(edited.len(), 6, "the family emoji is one grapheme");
    let family = edited[5].as_array().unwrap();
    assert_eq!(
        [&family[0], &family[1], &family[3]],
        [&json!(0.), &json!(70.), &json!(14.)]
    );
    assert_eq!(&family[4..], &[json!(5), json!(1)]);
    // The real native emoji fallback uses a single joined glyph rather
    // than the three-cell fallback from Flutter's test-font environment.
    assert!(family[2].as_f64().unwrap() > 0.);

    assert_eq!(
        publish(
            &mut runtime,
            &mut controls,
            editor(Some(20.), Some(45.), TextAlign::Start, None)
        ),
        vertical(0., 0., 4),
    );
    assert_eq!(
        publish(
            &mut runtime,
            &mut controls,
            editor(Some(20.), Some(45.), TextAlign::Right, None)
        ),
        vertical(5., 0., 4),
    );
    assert_eq!(
        publish(
            &mut runtime,
            &mut controls,
            editor(Some(20.), Some(45.), TextAlign::Right, Some(14.))
        ),
        vertical(5., -14., 5),
    );
}
