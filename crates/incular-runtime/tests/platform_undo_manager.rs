//! Flutter's platform undo manager (`services/undo_manager.dart` and
//! `UndoHistoryState` in `widgets/undo_history.dart`, support-003): a system
//! undo or redo command acts on the focused text field's history, and on
//! iOS the focused field reports its undo availability to the platform.

use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, UndoDirection, UndoManagerCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{Column, EditableText, Widget, internal::TextEditingController};

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
) -> Runtime {
    let mut runtime = Runtime::new(Widget::from(Column::new([
        Widget::from(field(first)),
        Widget::from(field(second)),
    ])))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(constraints()).unwrap();
    tap(&mut runtime, 5.);
    runtime
}

fn tap(runtime: &mut Runtime, y: f32) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(5., y),
        });
    }
    runtime.run_frame(constraints()).unwrap();
}

fn state(can_undo: bool, can_redo: bool) -> UndoManagerCommand {
    UndoManagerCommand::SetUndoState { can_undo, can_redo }
}

#[test]
fn platform_undo_and_redo_act_on_the_focused_field() {
    let first = TextEditingController::with_text("one");
    let second = TextEditingController::with_text("other");
    let mut runtime = mount(&first, &second, TextShortcutPlatform::Windows);
    first.set_text("two");
    runtime.run_frame(constraints()).unwrap();

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
    let mut runtime = mount(&first, &second, TextShortcutPlatform::Ios);
    // Gaining focus reports the state (`_handleFocus` then `_updateState`).
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, false)]);

    first.set_text("two");
    runtime.run_frame(constraints()).unwrap();
    assert_eq!(runtime.take_undo_manager_commands(), [state(true, false)]);
    runtime.run_frame(constraints()).unwrap();
    assert!(
        runtime.take_undo_manager_commands().is_empty(),
        "an unchanged state is not sent again"
    );

    assert!(runtime.handle_platform_undo(UndoDirection::Undo));
    runtime.run_frame(constraints()).unwrap();
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, true)]);

    // The other field becomes the client and reports its own history.
    tap(&mut runtime, 40.);
    assert_eq!(runtime.take_undo_manager_commands(), [state(false, false)]);
}
