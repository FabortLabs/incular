use incular_config::{Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextEditingController, TextSelection, TextStyle};
use incular_widgets::EditableText;

const TEXT: &str = "Now is the time for\nall good people\nto come to the aid\nof their country.";

const NON_IOS: &[TextShortcutPlatform] = &[
    TextShortcutPlatform::Android, // Fuchsia uses the same common shortcut map.
    TextShortcutPlatform::Linux,
    TextShortcutPlatform::MacOs,
    TextShortcutPlatform::Windows,
];

const ALL: &[TextShortcutPlatform] = &[
    TextShortcutPlatform::Android,
    TextShortcutPlatform::Ios,
    TextShortcutPlatform::Linux,
    TextShortcutPlatform::MacOs,
    TextShortcutPlatform::Windows,
];

struct Expected<'a> {
    text: &'a str,
    caret: usize,
    affinity: TextAffinity,
}

fn backspace(
    platforms: &[TextShortcutPlatform],
    initial_text: &str,
    selection: TextSelection,
    affinity: TextAffinity,
    read_only: bool,
    expected: Expected<'_>,
) {
    let bounds = Constraints::tight(Size::new(201., 200.));
    for &platform in platforms {
        let controller = TextEditingController::with_text(initial_text);
        controller.set_selection_with_affinity(selection, affinity);
        let widget = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_size(10.))
            .multiline(true)
            .read_only(read_only)
            .autofocus(true);
        let mut runtime = Runtime::new(widget.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.run_frame(bounds).unwrap();
        let was_focused = runtime.focused_element();
        assert!(was_focused.is_some(), "{platform:?}");
        let key = KeyboardKey::Named(NamedKey::Backspace);
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
            key.clone(),
            Code::Backspace,
        )));
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_up(key, Code::Backspace)));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(runtime.focused_element(), was_focused);
        assert_eq!(controller.text(), expected.text, "{platform:?}");
        assert_eq!(
            controller.selection(),
            TextSelection::collapsed(expected.caret),
            "{platform:?}"
        );
        assert_eq!(
            controller.caret_affinity(),
            expected.affinity,
            "{platform:?}"
        );
    }
}

#[test]
fn backspace_joins_with_the_previous_line() {
    backspace(
        NON_IOS,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        false,
        Expected {
            text: "Now is the time forall good people\nto come to the aid\nof their country.",
            caret: 19,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn read_only_backspace_does_not_change_text_or_selection_affinity() {
    backspace(
        ALL,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        true,
        Expected {
            text: TEXT,
            caret: 20,
            affinity: TextAffinity::Upstream,
        },
    );
}

#[test]
fn backspace_at_document_start_is_a_no_op() {
    backspace(
        ALL,
        TEXT,
        TextSelection::collapsed(0),
        TextAffinity::Downstream,
        false,
        Expected {
            text: TEXT,
            caret: 0,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn backspace_at_document_end_deletes_just_the_last_character() {
    backspace(
        NON_IOS,
        TEXT,
        TextSelection::collapsed(72),
        TextAffinity::Upstream,
        false,
        Expected {
            text: "Now is the time for\nall good people\nto come to the aid\nof their country",
            caret: 71,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn backspace_at_a_cluster_boundary_removes_the_entire_preceding_emoji() {
    const FAMILY: &str = "👨‍👩‍👦";
    const INPUT: &str = "👨‍👩‍👦👨‍👩‍👦👨‍👩‍👦";
    const RESULT: &str = "👨‍👩‍👦👨‍👩‍👦";
    // Flutter records a UTF-16 selection at code-unit offset 8.
    // Incular's equivalent is the UTF-8 byte boundary after the first emoji.
    assert_eq!(FAMILY.encode_utf16().count(), 8);
    backspace(
        NON_IOS,
        INPUT,
        TextSelection::collapsed(FAMILY.len()),
        TextAffinity::Upstream,
        false,
        Expected {
            text: RESULT,
            caret: 0,
            affinity: TextAffinity::Downstream,
        },
    );
}
