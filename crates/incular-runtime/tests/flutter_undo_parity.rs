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
