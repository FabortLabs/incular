use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextAlign, TextEditingController, TextSelection, TextStyle};
use incular_widgets::{Align, EditableText, FocusNode, SizedBox};

const DOCUMENT: &str =
    "Now is the time for\nall good people\nto come to the aid\nof their country.";
const PLATFORMS: [TextShortcutPlatform; 2] =
    [TextShortcutPlatform::Ios, TextShortcutPlatform::MacOs];

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn editor(platform: TextShortcutPlatform, input: &str) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(input);
    controller.set_selection_with_affinity(TextSelection::collapsed(0), TextAffinity::Upstream);
    let focus = FocusNode::new();
    let mut style = TextStyle::new().font_size(16.);
    if let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") {
        const FONT: &str = "FlutterTest Apple document keys";
        incular_text::register_application_font_as(FONT, std::fs::read(font).unwrap());
        style = style.font_family(FONT);
    }
    let field = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right)
        .max_lines(Some(10))
        .style(style);
    let mut runtime = Runtime::new(
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(400.).child(field),
        )
        .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
    (runtime, controller)
}

fn control(runtime: &mut Runtime, key: &str, code: Code) {
    let ctrl = incular_core::KeyboardKey::Named(incular_core::NamedKey::Control);
    let key = KeyboardKey::Character(key.to_owned());
    let _ =
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(ctrl.clone(), Code::ControlLeft));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
        modifiers: Modifiers::CONTROL,
        ..KeyboardEvent::key_down(key.clone(), code)
    });
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
        modifiers: Modifiers::CONTROL,
        ..KeyboardEvent::key_up(key, code)
    });
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(ctrl, Code::ControlLeft));
    frame(runtime);
}

#[test]
fn control_line_character_and_vertical_movement_keep_exact_offsets_on_apple() {
    for platform in PLATFORMS {
        let (mut runtime, text) = editor(platform, DOCUMENT);
        control(&mut runtime, "e", Code::KeyE);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(19),
            "{platform:?}"
        );
        assert_eq!(
            text.caret_affinity(),
            TextAffinity::Upstream,
            "{platform:?}"
        );
        control(&mut runtime, "a", Code::KeyA);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(0),
            "{platform:?}"
        );
        assert_eq!(
            text.caret_affinity(),
            TextAffinity::Downstream,
            "{platform:?}"
        );

        text.set_selection_with_affinity(TextSelection::collapsed(0), TextAffinity::Upstream);
        control(&mut runtime, "f", Code::KeyF);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(1),
            "{platform:?}"
        );
        control(&mut runtime, "b", Code::KeyB);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(0),
            "{platform:?}"
        );

        text.set_selection_with_affinity(TextSelection::collapsed(0), TextAffinity::Upstream);
        control(&mut runtime, "n", Code::KeyN);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(20),
            "{platform:?}"
        );
        control(&mut runtime, "p", Code::KeyP);
        assert_eq!(
            text.selection(),
            TextSelection::collapsed(0),
            "{platform:?}"
        );
        assert_eq!(text.text(), DOCUMENT);
    }
}

#[test]
fn control_transpose_keeps_ranges_and_swaps_the_same_characters_on_apple() {
    for platform in PLATFORMS {
        let (mut runtime, text) = editor(platform, DOCUMENT);
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.selection(), TextSelection::collapsed(0));
        assert_eq!(text.text(), DOCUMENT);
        text.set_selection(TextSelection::new(1, 4));
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.selection(), TextSelection::new(1, 4));
        assert_eq!(text.text(), DOCUMENT);
        text.set_selection(TextSelection::collapsed(5));
        for (at, first_line) in [
            (6, "Now si the time for"),
            (7, "Now s ithe time for"),
            (8, "Now s tihe time for"),
        ] {
            control(&mut runtime, "t", Code::KeyT);
            assert_eq!(text.selection(), TextSelection::collapsed(at));
            assert_eq!(&text.text()[..19], first_line);
        }
        text.set_selection(TextSelection::collapsed(text.text().len()));
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.selection(), TextSelection::collapsed(72));
        assert_eq!(&text.text()[55..], "of their countr.y");
    }
}

#[test]
fn control_transpose_moves_an_entire_emoji_grapheme_on_apple() {
    const FAMILY: &str = "👨‍👩‍👦";
    const ORIGINAL: &str = "👨‍👩‍👦😆";
    for platform in PLATFORMS {
        let (mut runtime, text) = editor(platform, ORIGINAL);
        let utf16_offset = |byte| text.text()[..byte].encode_utf16().count();
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.text(), ORIGINAL);
        assert_eq!(text.selection(), TextSelection::collapsed(0));

        text.set_selection(TextSelection::new(FAMILY.len(), ORIGINAL.len()));
        let range = text.selection();
        assert_eq!(
            (utf16_offset(range.base), utf16_offset(range.extent)),
            (8, 10)
        );
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.text(), ORIGINAL);
        assert_eq!(text.selection(), range);

        text.set_selection(TextSelection::collapsed(FAMILY.len()));
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.text(), "😆👨‍👩‍👦");
        assert_eq!(utf16_offset(text.selection().extent), 10);
        control(&mut runtime, "t", Code::KeyT);
        assert_eq!(text.text(), ORIGINAL);
        assert_eq!(utf16_offset(text.selection().extent), 10);
    }
}

#[test]
fn host_macos_selectors_apply_to_the_current_paragraph_in_order() {
    use serde_json::json;

    let (mut runtime, text) = editor(TextShortcutPlatform::MacOs, "test\nline2");
    text.set_selection(TextSelection::collapsed(10));
    frame(&mut runtime);
    let client = runtime.flutter_text_input_configuration().unwrap().0;
    for (selectors, offset) in [
        (vec!["moveLeft:"], 9),
        (vec!["moveToBeginningOfParagraph:"], 5),
        (vec!["moveBackward:", "moveToBeginningOfParagraph:"], 0),
    ] {
        let packet = serde_json::to_vec(&json!({
            "method": "TextInputClient.performSelectors",
            "args": [client.get(), selectors],
        }))
        .unwrap();
        assert_eq!(
            runtime
                .handle_flutter_text_input_selectors_call(&packet)
                .unwrap(),
            Some(true)
        );
        frame(&mut runtime);
        assert_eq!(text.selection(), TextSelection::collapsed(offset));
    }
    assert_eq!(text.text(), "test\nline2");
}
