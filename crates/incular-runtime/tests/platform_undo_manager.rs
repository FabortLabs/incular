//! Flutter's platform undo manager (`services/undo_manager.dart` and
//! `UndoHistoryState` in `widgets/undo_history.dart`, support-003): a system
//! undo or redo command acts on the focused text field's history, and on
//! iOS the focused field reports its undo availability to the platform.

use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, UndoDirection, UndoManagerCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingValue, TextSelection};
use incular_widgets::{Column, EditableText, FocusNode, Widget, internal::TextEditingController};
use std::time::{Duration, Instant};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(180., 100.))
}

fn field(controller: &TextEditingController) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .size(Size::new(180., 32.))
}

/// Two fields; the first is focused by a tap.
fn mount(
    first: &TextEditingController,
    second: &TextEditingController,
    platform: TextShortcutPlatform,
) -> (Runtime, Instant) {
    let mut runtime = Runtime::new(Widget::from(Column::new([
        Widget::from(field(first)),
        Widget::from(field(second)),
    ])))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    let start = Instant::now();
    runtime.run_frame_at(constraints(), start).unwrap();
    tap(&mut runtime, 5., start);
    let seeded = start + Duration::from_millis(600);
    runtime.run_frame_at(constraints(), seeded).unwrap();
    (runtime, seeded)
}

fn tap(runtime: &mut Runtime, y: f32, now: Instant) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: Offset::new(5., y),
            },
            now,
        );
    }
    runtime.run_frame_at(constraints(), now).unwrap();
}

fn commit_edit(
    runtime: &mut Runtime,
    controller: &TextEditingController,
    text: &str,
    now: Instant,
) -> Instant {
    controller.set_value(TextEditingValue {
        text: text.into(),
        selection: TextSelection::collapsed(text.len()),
        composing: None,
    });
    let next = now + Duration::from_millis(600);
    runtime.run_frame_at(constraints(), next).unwrap();
    next
}

fn state(can_undo: bool, can_redo: bool) -> UndoManagerCommand {
    UndoManagerCommand::SetUndoState { can_undo, can_redo }
}

#[test]
fn platform_undo_and_redo_act_on_the_focused_field() {
    let first = TextEditingController::with_text("one");
    let second = TextEditingController::with_text("other");
    let (mut runtime, now) = mount(&first, &second, TextShortcutPlatform::Windows);
    commit_edit(&mut runtime, &first, "two", now);

    let _ = runtime.handle_platform_event(PlatformEvent::Undo(UndoDirection::Undo));
    assert_eq!(first.text(), "one");
    assert_eq!(second.text(), "other");
    let _ = runtime.handle_platform_event(PlatformEvent::Undo(UndoDirection::Redo));
    assert_eq!(first.text(), "two");
    assert!(
        !runtime.handle_platform_undo(UndoDirection::Redo),
        "nothing to redo"
    );

    // Flutter sends the undo state only on iOS.
    assert!(runtime.take_undo_manager_commands().is_empty());
}

#[test]
fn without_a_focused_field_platform_undo_does_nothing() {
    let controller = TextEditingController::with_text("a");
    let mut runtime = Runtime::new(Widget::from(field(&controller))).unwrap();
    runtime.run_frame(constraints()).unwrap();
    controller.set_text("b");
    assert!(!runtime.handle_platform_undo(UndoDirection::Undo));
    assert_eq!(controller.text(), "b");
}

#[test]
fn on_ios_the_focused_field_reports_its_undo_state() {
    let first = TextEditingController::with_text("one");
    let second = TextEditingController::with_text("x");
    let (mut runtime, now) = mount(&first, &second, TextShortcutPlatform::Ios);
    // Gaining focus reports the state (`_handleFocus` then `_updateState`).
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, false)]);

    let now = commit_edit(&mut runtime, &first, "two", now);
    assert_eq!(runtime.take_undo_manager_commands(), [state(true, false)]);
    runtime.run_frame_at(constraints(), now).unwrap();
    assert!(
        runtime.take_undo_manager_commands().is_empty(),
        "an unchanged state is not sent again"
    );

    assert!(runtime.handle_platform_undo(UndoDirection::Undo));
    runtime.run_frame_at(constraints(), now).unwrap();
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, true)]);

    // The other field becomes the client and reports its own history.
    tap(&mut runtime, 40., now);
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, false)]);
}

#[test]
fn native_undo_targets_the_latest_focus_request_before_the_next_frame() {
    let first = TextEditingController::with_text("one");
    let second = TextEditingController::with_text("alpha");
    let first_focus = FocusNode::new();
    let second_focus = FocusNode::new();
    let mut runtime = Runtime::new(Widget::from(Column::new([
        field(&first).focus_node(first_focus.clone()),
        field(&second).focus_node(second_focus.clone()),
    ])))
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let now = Instant::now();
    runtime.run_frame_at(constraints(), now).unwrap();
    first_focus.request_focus();
    runtime.run_frame_at(constraints(), now).unwrap();
    let now = now + Duration::from_millis(600);
    runtime.run_frame_at(constraints(), now).unwrap();
    let now = commit_edit(&mut runtime, &first, "two", now);
    assert_eq!(
        runtime.take_undo_manager_commands().last(),
        Some(&state(true, false))
    );

    second_focus.request_focus();
    runtime.run_frame_at(constraints(), now).unwrap();
    let now = now + Duration::from_millis(600);
    runtime.run_frame_at(constraints(), now).unwrap();
    let now = commit_edit(&mut runtime, &second, "beta", now);
    assert!(second_focus.has_primary_focus());
    assert_eq!(
        runtime.take_undo_manager_commands().last(),
        Some(&state(true, false))
    );

    // The request arrives directly from a focus node and native undo can
    // arrive immediately, without a key event or intervening frame.
    first_focus.request_focus();
    let _ = runtime.handle_platform_event(PlatformEvent::Undo(UndoDirection::Undo));
    assert_eq!(
        second.text(),
        "beta",
        "native undo edited the outgoing field"
    );
    assert_eq!(first.text(), "one");
    assert!(first_focus.has_primary_focus());
    runtime.run_frame_at(constraints(), now).unwrap();
    assert_eq!(
        runtime.take_undo_manager_commands().last(),
        Some(&state(false, true))
    );

    // Moving the focus back must not apply the outgoing field's redo.
    second_focus.request_focus();
    let _ = runtime.handle_platform_event(PlatformEvent::Undo(UndoDirection::Redo));
    assert!(second_focus.has_primary_focus());
    assert_eq!(second.text(), "beta");
    assert_eq!(first.text(), "one");
}
