use incular_config::{Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextEditingController, TextSelection, TextStyle};
use incular_widgets::EditableText;

const WITHOUT_GOOD: &str = "Now is the time for\nall people\nto come to the aid\nof their country.";
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    Editable,
    Obscured,
    ReadOnly,
}

#[derive(Clone, Copy, Debug)]
enum DeleteKey {
    Backspace,
    Delete,
    WordBackspace,
    WordDelete,
}

impl DeleteKey {
    fn events(self, platform: TextShortcutPlatform) -> [InputEvent; 2] {
        let backward = matches!(self, Self::Backspace | Self::WordBackspace);
        let (key, code) = if backward {
            (NamedKey::Backspace, Code::Backspace)
        } else {
            (NamedKey::Delete, Code::Delete)
        };
        let key = KeyboardKey::Named(key);
        let mut down = KeyboardEvent::key_down(key.clone(), code);
        let mut up = KeyboardEvent::key_up(key, code);
        if matches!(self, Self::WordBackspace | Self::WordDelete) {
            let mods = match platform {
                TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs => Modifiers::ALT,
                _ => Modifiers::CONTROL,
            };
            down.modifiers = mods;
            up.modifiers = mods;
        }
        [InputEvent::Key(down), InputEvent::Key(up)]
    }
}

fn check_deletion(
    key: DeleteKey,
    platforms: &[TextShortcutPlatform],
    initial_text: &str,
    selection: TextSelection,
    affinity: TextAffinity,
    mode: EditorMode,
    expected: Expected<'_>,
) {
    let bounds = Constraints::tight(Size::new(201., 200.));
    for &platform in platforms {
        let controller = TextEditingController::with_text(initial_text);
        controller.set_selection_with_affinity(selection, affinity);
        let widget = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_size(10.))
            .multiline(mode != EditorMode::Obscured)
            .obscure_text(mode == EditorMode::Obscured)
            .read_only(mode == EditorMode::ReadOnly)
            .autofocus(true);
        let mut runtime = Runtime::new(widget.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.run_frame(bounds).unwrap();
        let was_focused = runtime.focused_element();
        assert!(was_focused.is_some(), "{platform:?}");
        for event in key.events(platform) {
            let _ = runtime.handle_input(event);
        }
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
    check_deletion(
        DeleteKey::Backspace,
        NON_IOS,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        EditorMode::Editable,
        Expected {
            text: "Now is the time forall good people\nto come to the aid\nof their country.",
            caret: 19,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn read_only_backspace_does_not_change_text_or_selection_affinity() {
    check_deletion(
        DeleteKey::Backspace,
        ALL,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        EditorMode::ReadOnly,
        Expected {
            text: TEXT,
            caret: 20,
            affinity: TextAffinity::Upstream,
        },
    );
}

#[test]
fn backspace_at_document_start_is_a_no_op() {
    check_deletion(
        DeleteKey::Backspace,
        ALL,
        TEXT,
        TextSelection::collapsed(0),
        TextAffinity::Downstream,
        EditorMode::Editable,
        Expected {
            text: TEXT,
            caret: 0,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn backspace_at_document_end_deletes_just_the_last_character() {
    check_deletion(
        DeleteKey::Backspace,
        NON_IOS,
        TEXT,
        TextSelection::collapsed(72),
        TextAffinity::Upstream,
        EditorMode::Editable,
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
    check_deletion(
        DeleteKey::Backspace,
        NON_IOS,
        INPUT,
        TextSelection::collapsed(FAMILY.len()),
        TextAffinity::Upstream,
        EditorMode::Editable,
        Expected {
            text: RESULT,
            caret: 0,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn forward_delete_removes_the_first_character_of_the_current_line() {
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        EditorMode::Editable,
        Expected {
            text: "Now is the time for\nll good people\nto come to the aid\nof their country.",
            caret: 20,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn read_only_forward_delete_does_not_change_text_or_selection_affinity() {
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        TEXT,
        TextSelection::collapsed(20),
        TextAffinity::Upstream,
        EditorMode::ReadOnly,
        Expected {
            text: TEXT,
            caret: 20,
            affinity: TextAffinity::Upstream,
        },
    );
}

#[test]
fn forward_delete_at_document_start_removes_the_first_character() {
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        TEXT,
        TextSelection::collapsed(0),
        TextAffinity::Downstream,
        EditorMode::Editable,
        Expected {
            text: "ow is the time for\nall good people\nto come to the aid\nof their country.",
            caret: 0,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn forward_delete_at_document_end_preserves_affinity() {
    check_deletion(
        DeleteKey::Delete,
        ALL,
        TEXT,
        TextSelection::collapsed(72),
        TextAffinity::Upstream,
        EditorMode::Editable,
        Expected {
            text: TEXT,
            caret: 72,
            affinity: TextAffinity::Upstream,
        },
    );
}

#[test]
fn forward_delete_at_a_cluster_boundary_removes_the_entire_following_emoji() {
    const FAMILY: &str = "👨‍👩‍👦";
    const INPUT: &str = "👨‍👩‍👦👨‍👩‍👦👨‍👩‍👦";
    const RESULT: &str = "👨‍👩‍👦👨‍👩‍👦";
    assert_eq!(FAMILY.encode_utf16().count(), 8);
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        INPUT,
        TextSelection::collapsed(FAMILY.len()),
        TextAffinity::Upstream,
        EditorMode::Editable,
        Expected {
            text: RESULT,
            caret: FAMILY.len(),
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn forward_delete_removes_an_exactly_selected_emoji_cluster() {
    const FAMILY: &str = "👨‍👩‍👦";
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        "👨‍👩‍👦👨‍👩‍👦👨‍👩‍👦",
        TextSelection::new(FAMILY.len(), 2 * FAMILY.len()),
        TextAffinity::Downstream,
        EditorMode::Editable,
        Expected {
            text: "👨‍👩‍👦👨‍👩‍👦",
            caret: FAMILY.len(),
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn deletion_expands_partial_graphemes_independent_of_selection_direction() {
    // Between e and its acute mark, and between o and its diaeresis.
    for selection in [TextSelection::new(2, 5), TextSelection::new(5, 2)] {
        check_deletion(
            DeleteKey::Delete,
            NON_IOS,
            "CéöD",
            selection,
            TextAffinity::Downstream,
            EditorMode::Editable,
            Expected {
                text: "CD",
                caret: 1,
                affinity: TextAffinity::Downstream,
            },
        );
    }
}

#[test]
fn character_deletion_from_inside_a_grapheme_removes_the_whole_grapheme() {
    for key in [DeleteKey::Delete, DeleteKey::Backspace] {
        check_deletion(
            key,
            NON_IOS,
            "CéD",
            TextSelection::collapsed(2),
            TextAffinity::Upstream,
            EditorMode::Editable,
            Expected {
                text: "CD",
                caret: 1,
                affinity: TextAffinity::Downstream,
            },
        );
    }
}

#[test]
fn obscured_delete_does_not_expand_past_selected_code_points() {
    check_deletion(
        DeleteKey::Delete,
        NON_IOS,
        "CéD",
        TextSelection::new(2, 4),
        TextAffinity::Downstream,
        EditorMode::Obscured,
        Expected {
            text: "CeD",
            caret: 2,
            affinity: TextAffinity::Downstream,
        },
    );
}

#[test]
fn word_backward_deletion_matches_flutter_at_words_and_document_edges() {
    for (
        platforms,
        caret,
        original_affinity,
        mode,
        expected_text,
        expected_caret,
        expected_affinity,
    ) in [
        (
            NON_IOS,
            29,
            TextAffinity::Upstream,
            EditorMode::Editable,
            WITHOUT_GOOD,
            24,
            TextAffinity::Downstream,
        ),
        (
            ALL,
            29,
            TextAffinity::Upstream,
            EditorMode::ReadOnly,
            TEXT,
            29,
            TextAffinity::Upstream,
        ),
        (
            ALL,
            0,
            TextAffinity::Downstream,
            EditorMode::Editable,
            TEXT,
            0,
            TextAffinity::Downstream,
        ),
        (
            NON_IOS,
            72,
            TextAffinity::Upstream,
            EditorMode::Editable,
            "Now is the time for\nall good people\nto come to the aid\nof their ",
            64,
            TextAffinity::Downstream,
        ),
    ] {
        check_deletion(
            DeleteKey::WordBackspace,
            platforms,
            TEXT,
            TextSelection::collapsed(caret),
            original_affinity,
            mode,
            Expected {
                text: expected_text,
                caret: expected_caret,
                affinity: expected_affinity,
            },
        );
    }
}

#[test]
fn word_forward_deletion_matches_flutter_at_words_and_document_edges() {
    for (
        platforms,
        caret,
        original_affinity,
        mode,
        expected_text,
        expected_caret,
        expected_affinity,
    ) in [
        (
            NON_IOS,
            23,
            TextAffinity::Upstream,
            EditorMode::Editable,
            WITHOUT_GOOD,
            23,
            TextAffinity::Downstream,
        ),
        (
            ALL,
            23,
            TextAffinity::Upstream,
            EditorMode::ReadOnly,
            TEXT,
            23,
            TextAffinity::Upstream,
        ),
        (
            NON_IOS,
            0,
            TextAffinity::Downstream,
            EditorMode::Editable,
            " is the time for\nall good people\nto come to the aid\nof their country.",
            0,
            TextAffinity::Downstream,
        ),
        (
            ALL,
            72,
            TextAffinity::Upstream,
            EditorMode::Editable,
            TEXT,
            72,
            TextAffinity::Upstream,
        ),
    ] {
        check_deletion(
            DeleteKey::WordDelete,
            platforms,
            TEXT,
            TextSelection::collapsed(caret),
            original_affinity,
            mode,
            Expected {
                text: expected_text,
                caret: expected_caret,
                affinity: expected_affinity,
            },
        );
    }
}

#[test]
fn word_deletion_stops_at_the_next_emoji_cluster_boundary() {
    const FAMILY: &str = "👨‍👩‍👦";
    const INPUT: &str = "👨‍👩‍👦👨‍👩‍👦👨‍👩‍👦";
    const RESULT: &str = "👨‍👩‍👦👨‍👩‍👦";
    for (key, caret_after) in [
        (DeleteKey::WordBackspace, 0),
        (DeleteKey::WordDelete, FAMILY.len()),
    ] {
        check_deletion(
            key,
            NON_IOS,
            INPUT,
            TextSelection::collapsed(FAMILY.len()),
            TextAffinity::Upstream,
            EditorMode::Editable,
            Expected {
                text: RESULT,
                caret: caret_after,
                affinity: TextAffinity::Downstream,
            },
        );
    }
}
