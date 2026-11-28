//! Flutter 3.50.0 EditableText groups accepted values in 500 ms windows,
//! including the first valid selection used to seed history.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, TextDirection};
use std::time::{Duration, Instant};

struct Editor {
    runtime: Runtime,
    controller: TextEditingController,
    now: Instant,
}

impl Editor {
    fn focused() -> Self {
        let controller = TextEditingController::with_text("abc");
        let field = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(180., 40.));
        let mut runtime = Runtime::new(field.into()).expect("mount editable text");
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
        let mut editor = Self {
            runtime,
            controller,
            now: Instant::now(),
        };
        editor.advance(0);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = editor.runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: Offset::new(20., 20.),
                },
                editor.now,
            );
        }
        assert!(editor.runtime.focused_element().is_some());
        editor.controller.set_selection(TextSelection::collapsed(3));
        editor.advance(16);
        editor
    }

    fn advance(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(180., 40.)), self.now)
            .expect("render editable text");
    }

    fn press(&mut self, key: KeyboardKey, code: Code, modifiers: Modifiers) {
        for event in [
            KeyboardEvent::key_down(key.clone(), code),
            KeyboardEvent::key_up(key, code),
        ] {
            let _ = self.runtime.handle_input_at(
                InputEvent::Key(KeyboardEvent { modifiers, ..event }),
                self.now,
            );
        }
        self.advance(16);
    }

    fn backspace(&mut self) {
        self.press(
            KeyboardKey::Named(NamedKey::Backspace),
            Code::Backspace,
            Modifiers::empty(),
        );
    }

    fn undo(&mut self) {
        self.press(
            KeyboardKey::Character("z".into()),
            Code::KeyZ,
            Modifiers::CONTROL,
        );
    }

    fn redo(&mut self) {
        self.press(
            KeyboardKey::Character("Z".into()),
            Code::KeyZ,
            Modifiers::CONTROL | Modifiers::SHIFT,
        );
    }
}

#[test]
fn edit_before_initial_history_sample_has_no_earlier_state_to_undo() {
    let mut editor = Editor::focused();
    editor.backspace();
    assert_eq!(editor.controller.text(), "ab");
    editor.advance(600);

    editor.undo();
    assert_eq!(
        editor.controller.text(),
        "ab",
        "the first history sample contains the already-edited value"
    );
}

#[test]
fn rapid_backspaces_after_initial_history_sample_undo_in_one_step() {
    let mut editor = Editor::focused();
    editor.advance(600);
    editor.backspace();
    assert_eq!(editor.controller.text(), "ab");
    editor.backspace();
    assert_eq!(editor.controller.text(), "a");
    editor.advance(600);

    editor.undo();
    assert_eq!(
        editor.controller.text(),
        "abc",
        "both backspaces belong to the same history sample"
    );
    editor.redo();
    assert_eq!(editor.controller.text(), "a");
}

#[test]
fn single_backspace_after_initial_history_sample_can_undo_and_redo() {
    let mut editor = Editor::focused();
    editor.advance(600);
    editor.backspace();
    assert_eq!(editor.controller.text(), "ab");
    editor.advance(600);

    editor.undo();
    assert_eq!(editor.controller.text(), "abc");
    editor.redo();
    assert_eq!(editor.controller.text(), "ab");
}

#[test]
fn keyboard_undo_cancels_pending_edit_without_creating_a_redo() {
    let mut editor = Editor::focused();
    editor.advance(600);
    editor.backspace();
    assert_eq!(editor.controller.text(), "ab");
    editor.undo();
    assert_eq!(editor.controller.text(), "abc");
    assert_eq!(editor.controller.selection(), TextSelection::collapsed(3));
    editor.advance(600);
    editor.redo();
    assert_eq!(editor.controller.text(), "abc");
}

#[test]
fn an_unfocused_editor_reports_exactly_one_group_deadline() {
    let controller = TextEditingController::with_text("abc");
    controller.set_selection(TextSelection::collapsed(3));
    let field = EditableText::new(controller).text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(field.into()).unwrap();
    let bounds = Constraints::tight(Size::new(180., 40.));
    let start = Instant::now();
    runtime.run_frame_at(bounds, start).unwrap();
    assert!(runtime.focused_element().is_none());
    assert_eq!(
        runtime.next_frame_deadline(),
        Some(start + Duration::from_millis(500))
    );
    runtime
        .run_frame_at(bounds, start + Duration::from_millis(499))
        .unwrap();
    assert_eq!(
        runtime.next_frame_deadline(),
        Some(start + Duration::from_millis(500))
    );
    runtime
        .run_frame_at(bounds, start + Duration::from_millis(500))
        .unwrap();
    assert_eq!(runtime.next_frame_deadline(), None);
}

const HISTORY_PLATFORMS: [TextShortcutPlatform; 5] = [
    TextShortcutPlatform::Android,
    TextShortcutPlatform::Ios,
    TextShortcutPlatform::Linux,
    TextShortcutPlatform::MacOs,
    TextShortcutPlatform::Windows,
];

fn unfocused_history_editor(
    text: &str,
    platform: TextShortcutPlatform,
) -> (Editor, incular_widgets::FocusNode) {
    let controller = TextEditingController::with_text(text);
    let focus = incular_widgets::FocusNode::new();
    let widget = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_text_shortcut_platform(platform);
    let mut editor = Editor {
        runtime,
        controller,
        now: Instant::now(),
    };
    editor.advance(0);
    assert!(!focus.has_focus());
    assert!(editor.controller.selection_if_set().is_none());
    (editor, focus)
}

fn history_shortcut(editor: &mut Editor, platform: TextShortcutPlatform, redo: bool) {
    let modifier = match platform {
        TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs => Modifiers::META,
        _ => Modifiers::CONTROL,
    };
    let modifiers = if redo {
        modifier | Modifiers::SHIFT
    } else {
        modifier
    };
    editor.press(
        KeyboardKey::Character(if redo { "Z" } else { "z" }.into()),
        Code::KeyZ,
        modifiers,
    );
}

#[test]
fn undo_and_redo_leave_a_never_edited_empty_field_alone_with_or_without_focus() {
    for platform in HISTORY_PLATFORMS {
        let (mut editor, focus) = unfocused_history_editor("", platform);
        let initial = editor.controller.with_value(Clone::clone);
        for redo in [false, true] {
            history_shortcut(&mut editor, platform, redo);
            assert_eq!(
                editor.controller.with_value(Clone::clone),
                initial,
                "{platform:?}"
            );
            assert!(!focus.has_focus());
        }

        editor.advance(500);
        focus.request_focus();
        editor.advance(0);
        assert!(focus.has_primary_focus());
        assert_eq!(
            editor.controller.selection_if_set(),
            Some(TextSelection::collapsed(0))
        );
        editor.advance(500);
        let focused_empty = editor.controller.with_value(Clone::clone);
        for redo in [false, true] {
            history_shortcut(&mut editor, platform, redo);
            assert_eq!(
                editor.controller.with_value(Clone::clone),
                focused_empty,
                "{platform:?}"
            );
            assert!(focus.has_primary_focus());
        }
    }
}

#[test]
fn an_undo_immediately_after_focus_does_not_discard_the_initial_value() {
    use incular_platform::{PlatformEvent, TextInputState};

    for platform in HISTORY_PLATFORMS {
        let (mut editor, focus) = unfocused_history_editor("A", platform);
        focus.request_focus();
        editor.advance(0);
        assert!(focus.has_primary_focus());
        history_shortcut(&mut editor, platform, false);
        editor.advance(500);
        let expected = if matches!(
            platform,
            TextShortcutPlatform::Linux
                | TextShortcutPlatform::MacOs
                | TextShortcutPlatform::Windows
        ) {
            TextSelection::new(0, 1)
        } else {
            TextSelection::collapsed(1)
        };
        assert_eq!(editor.controller.text(), "A", "{platform:?}");
        assert_eq!(editor.controller.selection(), expected, "{platform:?}");

        let _ = editor
            .runtime
            .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
                text: "AB".into(),
                selection_start: 2,
                selection_end: 2,
                composing: None,
            }));
        editor.advance(0);
        assert_eq!(editor.controller.text(), "AB");
        assert_eq!(editor.controller.selection(), TextSelection::collapsed(2));
        history_shortcut(&mut editor, platform, false);
        assert_eq!(editor.controller.text(), "A", "{platform:?}");
        assert_eq!(editor.controller.selection(), expected, "{platform:?}");
        assert!(focus.has_primary_focus());
    }
}

#[test]
fn immediate_undo_after_first_insertion_restores_the_focused_value_without_a_stale_caret() {
    use incular_platform::{PlatformEvent, TextInputState};

    for platform in HISTORY_PLATFORMS {
        let (mut editor, focus) = unfocused_history_editor("A", platform);
        focus.request_focus();
        editor.advance(0);
        editor.advance(500);
        let expected = if matches!(
            platform,
            TextShortcutPlatform::Linux
                | TextShortcutPlatform::MacOs
                | TextShortcutPlatform::Windows
        ) {
            TextSelection::new(0, 1)
        } else {
            TextSelection::collapsed(1)
        };
        assert_eq!(editor.controller.text(), "A");
        assert_eq!(editor.controller.selection_if_set(), Some(expected));
        let _ = editor
            .runtime
            .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
                text: "AB".into(),
                selection_start: 2,
                selection_end: 2,
                composing: None,
            }));
        editor.advance(0);
        assert_eq!(editor.controller.text(), "AB");
        assert_eq!(
            editor.controller.selection_if_set(),
            Some(TextSelection::collapsed(2))
        );

        history_shortcut(&mut editor, platform, false);
        assert_eq!(editor.controller.text(), "A");
        assert_eq!(editor.controller.selection_if_set(), Some(expected));
        assert!(focus.has_primary_focus());
    }
}

fn empty_editing_history(
    platform: TextShortcutPlatform,
    autofocus: bool,
) -> (Editor, incular_widgets::FocusNode) {
    if !autofocus {
        let (mut editor, focus) = unfocused_history_editor("", platform);
        focus.request_focus();
        assert!(editor.controller.selection_if_set().is_none());
        editor.advance(0);
        return (editor, focus);
    }
    let controller = TextEditingController::new();
    let focus = incular_widgets::FocusNode::new();
    let widget = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_text_shortcut_platform(platform);
    let mut editor = Editor {
        runtime,
        controller,
        now: Instant::now(),
    };
    editor.advance(0);
    (editor, focus)
}

fn replace_from_platform(editor: &mut Editor, text: &str) {
    use incular_platform::{PlatformEvent, TextInputState};

    let end = text.len(); // These pinned fixtures use ASCII.
    let _ = editor
        .runtime
        .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
            text: text.into(),
            selection_start: end,
            selection_end: end,
            composing: None,
        }));
    assert_history_value(editor, text);
}

fn assert_history_value(editor: &Editor, expected: &str) {
    editor.controller.with_value(|value| {
        assert_eq!(value.text, expected);
        assert_eq!(value.selection, TextSelection::collapsed(expected.len()));
        assert!(value.composing.is_none());
    });
}

fn shortcut_expect(
    editor: &mut Editor,
    platform: TextShortcutPlatform,
    operations: &[(bool, &str)],
) {
    for &(redo, expected) in operations {
        history_shortcut(editor, platform, redo);
        assert_history_value(editor, expected);
    }
}

#[test]
fn autofocused_first_frame_seeds_exactly_one_undo_step() {
    for platform in HISTORY_PLATFORMS {
        let (mut editor, focus) = empty_editing_history(platform, true);
        assert!(focus.has_primary_focus());
        assert_history_value(&editor, "");
        editor.advance(0);
        assert_history_value(&editor, "");
        editor.advance(500);
        replace_from_platform(&mut editor, "1");
        editor.advance(500);
        shortcut_expect(
            &mut editor,
            platform,
            &[(false, ""), (false, ""), (true, "1"), (true, "1")],
        );
        assert!(focus.has_primary_focus());
    }
}

#[test]
fn values_that_return_to_the_same_editing_state_do_not_duplicate_history() {
    for platform in HISTORY_PLATFORMS {
        let (mut editor, _) = empty_editing_history(platform, false);
        assert_history_value(&editor, "");
        editor.advance(500);
        replace_from_platform(&mut editor, "1");
        editor.advance(500);
        shortcut_expect(&mut editor, platform, &[(false, ""), (true, "1")]);

        replace_from_platform(&mut editor, "12");
        replace_from_platform(&mut editor, "1");
        editor.advance(500);
        assert_history_value(&editor, "1");
        shortcut_expect(
            &mut editor,
            platform,
            &[
                (true, "1"),
                (false, ""),
                (false, ""),
                (true, "1"),
                (true, "1"),
            ],
        );
    }
}

#[test]
fn single_and_multiple_separate_insertions_have_exact_undo_redo_boundaries() {
    for platform in HISTORY_PLATFORMS {
        for multiple in [false, true] {
            let (mut editor, _) = empty_editing_history(platform, false);
            assert_history_value(&editor, "");
            editor.advance(500);
            replace_from_platform(&mut editor, "A");
            editor.advance(500);
            if multiple {
                replace_from_platform(&mut editor, "AB");
                editor.advance(500);
                shortcut_expect(
                    &mut editor,
                    platform,
                    &[(false, "A"), (false, ""), (true, "A"), (true, "AB")],
                );
            } else {
                shortcut_expect(
                    &mut editor,
                    platform,
                    &[
                        (true, "A"),
                        (false, ""),
                        (false, ""),
                        (true, "A"),
                        (true, "A"),
                    ],
                );
            }
        }
    }
}

#[test]
fn a_new_insertion_from_inside_history_replaces_only_the_redo_branch() {
    for platform in HISTORY_PLATFORMS {
        let (mut editor, _) = empty_editing_history(platform, false);
        assert_history_value(&editor, "");
        editor.advance(500);
        replace_from_platform(&mut editor, "A");
        editor.advance(500);
        replace_from_platform(&mut editor, "AC");
        editor.advance(500);
        shortcut_expect(&mut editor, platform, &[(false, "A")]);
        replace_from_platform(&mut editor, "AB");
        editor.advance(500);
        shortcut_expect(
            &mut editor,
            platform,
            &[
                (true, "AB"),
                (true, "AB"),
                (false, "A"),
                (false, ""),
                (true, "A"),
                (true, "AB"),
            ],
        );
    }
}

fn replace_with_composition(editor: &mut Editor, text: &str, composing: Option<(usize, usize)>) {
    use incular_platform::{PlatformEvent, TextInputState};
    use incular_text::TextRange;

    let _ = editor
        .runtime
        .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
            text: text.into(),
            selection_start: text.len(),
            selection_end: text.len(),
            composing,
        }));
    editor.controller.with_value(|value| {
        assert_eq!(value.text, text);
        assert_eq!(value.selection, TextSelection::collapsed(text.len()));
        assert_eq!(
            value.composing,
            composing.map(|(start, end)| TextRange::new(start, end))
        );
    });
}

const EDITING_TARGETS: [incular_config::TargetPlatform; 6] = {
    use incular_config::TargetPlatform::*;
    [Android, Fuchsia, Ios, Linux, MacOs, Windows]
};

fn editor_for_composition(
    target: incular_config::TargetPlatform,
) -> (Editor, TextShortcutPlatform) {
    use incular_config::TargetPlatform;

    let (mut editor, _) = empty_editing_history(TextShortcutPlatform::Linux, false);
    // Change the target on a live editor before its first history sample.
    // The shortcut and history policies must both reach this existing field.
    editor.runtime.set_text_target_platform(target);
    let shortcuts = editor.runtime.text_shortcut_platform();
    if target == TargetPlatform::Fuchsia {
        assert_eq!(shortcuts, TextShortcutPlatform::Android);
    }
    editor.advance(500);
    (editor, shortcuts)
}

#[test]
fn android_records_composing_text_but_undo_and_redo_return_without_composing_spans() {
    use incular_config::TargetPlatform;

    let (mut editor, shortcuts) = editor_for_composition(TargetPlatform::Android);
    replace_with_composition(&mut editor, "1 ", None);
    editor.advance(500);
    replace_with_composition(&mut editor, "1 ni", Some((2, 4)));
    editor.advance(500);
    replace_with_composition(&mut editor, "1 nihao", Some((2, 7)));
    editor.advance(500);
    replace_with_composition(&mut editor, "1 你好", None);
    editor.advance(500);
    shortcut_expect(
        &mut editor,
        shortcuts,
        &[
            (false, "1 nihao"),
            (false, "1 ni"),
            (false, "1 "),
            (true, "1 ni"),
            (true, "1 nihao"),
            (true, "1 你好"),
            (true, "1 你好"),
            (false, "1 nihao"),
            (false, "1 ni"),
            (false, "1 "),
            (false, ""),
            (false, ""),
            (true, "1 "),
            (true, "1 ni"),
            (true, "1 nihao"),
            (true, "1 你好"),
        ],
    );
}

#[test]
fn a_delay_between_android_composition_undos_does_not_create_an_entry() {
    let (mut editor, shortcuts) = editor_for_composition(incular_config::TargetPlatform::Android);
    replace_with_composition(&mut editor, "1 ni", Some((2, 4)));
    editor.advance(500);
    replace_with_composition(&mut editor, "1 nihao", Some((2, 7)));
    editor.advance(500);
    replace_with_composition(&mut editor, "1 你好", None);
    editor.advance(500);
    shortcut_expect(&mut editor, shortcuts, &[(false, "1 nihao")]);
    editor.advance(500);
    shortcut_expect(&mut editor, shortcuts, &[(false, "1 ni")]);
}

#[test]
fn non_android_and_fuchsia_skip_composition_while_retaining_the_commit() {
    use incular_config::TargetPlatform;

    for target in EDITING_TARGETS
        .into_iter()
        .filter(|target| *target != TargetPlatform::Android)
    {
        let (mut editor, shortcuts) = editor_for_composition(target);
        replace_with_composition(&mut editor, "1 ", None);
        editor.advance(500);
        replace_with_composition(&mut editor, "1 ni", Some((2, 4)));
        editor.advance(500);
        replace_with_composition(&mut editor, "1 nihao", Some((2, 7)));
        editor.advance(500);
        replace_with_composition(&mut editor, "1 你好", None);
        editor.advance(500);
        shortcut_expect(
            &mut editor,
            shortcuts,
            &[
                (false, "1 "),
                (true, "1 你好"),
                (true, "1 你好"),
                (false, "1 "),
                (false, ""),
                (false, ""),
                (true, "1 "),
                (true, "1 你好"),
            ],
        );
    }
}

#[test]
fn a_pending_plain_edit_before_composition_is_not_lost_on_non_android_targets() {
    use incular_config::TargetPlatform;

    for target in EDITING_TARGETS {
        let (mut editor, shortcuts) = editor_for_composition(target);
        replace_with_composition(&mut editor, "1 ", None);
        editor.advance(500);
        replace_with_composition(&mut editor, "1 2 ", None);
        replace_with_composition(&mut editor, "1 2 ni", Some((4, 6)));
        editor.advance(500);
        replace_with_composition(&mut editor, "1 2 你", None);
        editor.advance(500);
        let intermediate = if target == TargetPlatform::Android {
            "1 2 ni"
        } else {
            "1 2 "
        };
        shortcut_expect(
            &mut editor,
            shortcuts,
            &[
                (false, intermediate),
                (false, "1 "),
                (false, ""),
                (false, ""),
                (true, "1 "),
                (true, intermediate),
                (true, "1 2 你"),
            ],
        );
    }
}
