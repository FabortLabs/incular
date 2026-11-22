//! Flutter's touch selection handles on text fields. `EditableTextState`
//! shows them after a touch or stylus long press selects, after a touch
//! double tap selects a word, and after a touch tap that places the caret in
//! a focused field on Android (`_shouldShowSelectionHandles`). A mouse press,
//! a typed edit or losing focus closes them.
//!
//! Touch drags follow `TextSelectionGestureDetectorBuilder`: iOS never selects
//! on a touch drag, and Android selects only from a field that already had
//! focus.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, SizedBox, TextDirection, TextSelectionHandles, Widget,
    internal::{TextEditingController, TextSelection},
};
use std::time::{Duration, Instant};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 120.))
}

fn mount(text: &str, platform: TextShortcutPlatform) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.))
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn pointer(
    runtime: &mut Runtime,
    phase: PointerPhase,
    position: Offset,
    kind: PointerDeviceKind,
    at: Instant,
) {
    let (buttons, button) = match phase {
        PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
        PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
        _ => (PRIMARY_POINTER_BUTTON, None),
    };
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind,
            buttons,
            button,
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        },
        at,
    );
}

/// A quick press and release at `position`, starting at `at`.
fn tap(runtime: &mut Runtime, position: Offset, kind: PointerDeviceKind, at: Instant) -> Instant {
    pointer(runtime, PointerPhase::Down, position, kind, at);
    let up = at + Duration::from_millis(50);
    pointer(runtime, PointerPhase::Up, position, kind, up);
    runtime.run_frame_at(bounds(), up).unwrap();
    up
}

/// A press held past the long press deadline, then released.
fn long_press(runtime: &mut Runtime, position: Offset, kind: PointerDeviceKind) -> Instant {
    let start = Instant::now();
    pointer(runtime, PointerPhase::Down, position, kind, start);
    let held = start + Duration::from_millis(600);
    runtime.run_frame_at(bounds(), held).unwrap();
    pointer(runtime, PointerPhase::Up, position, kind, held);
    runtime.run_frame_at(bounds(), held).unwrap();
    held
}

/// Down, move and up with a touch drag from `from` to `to`.
fn drag(runtime: &mut Runtime, from: Offset, to: Offset, kind: PointerDeviceKind, at: Instant) {
    pointer(runtime, PointerPhase::Down, from, kind, at);
    pointer(
        runtime,
        PointerPhase::Move,
        to,
        kind,
        at + Duration::from_millis(100),
    );
    pointer(
        runtime,
        PointerPhase::Up,
        to,
        kind,
        at + Duration::from_millis(120),
    );
    runtime.run_frame(bounds()).unwrap();
}

/// The handles showing on the focused field, if any.
fn shown_handles(runtime: &Runtime) -> Option<TextSelectionHandles> {
    let field = runtime.focused_element()?;
    runtime
        .tree()
        .text_field_selection_handles(field)?
        .handles()
}

#[test]
fn touch_long_press_shows_start_and_end_handles() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    let handles = shown_handles(&runtime).expect("range handles");
    assert!(handles.end.is_some(), "a range shows an end handle");
}

#[test]
fn touch_tap_on_focused_field_shows_a_collapsed_handle_on_android() {
    let (mut runtime, controller) = mount("hello", TextShortcutPlatform::Android);
    let start = Instant::now();
    let first = tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Touch,
        start,
    );
    assert!(
        shown_handles(&runtime).is_none(),
        "the focusing tap shows none"
    );
    // A second tap well after the double tap window places the caret.
    tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Touch,
        first + Duration::from_secs(1),
    );
    let selection = controller.selection();
    assert_eq!(
        selection.base, selection.extent,
        "the tap collapses the selection"
    );
    let handles = shown_handles(&runtime).expect("collapsed handle");
    assert!(
        handles.end.is_none(),
        "a collapsed selection shows one handle"
    );
}

#[test]
fn mouse_click_shows_no_handles() {
    let (mut runtime, _controller) = mount("hello", TextShortcutPlatform::Windows);
    let start = Instant::now();
    let first = tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Mouse,
        start,
    );
    tap(
        &mut runtime,
        Offset::new(20., 10.),
        PointerDeviceKind::Mouse,
        first + Duration::from_secs(1),
    );
    assert!(shown_handles(&runtime).is_none());
}

#[test]
fn typing_hides_the_handles() {
    let (mut runtime, _controller) = mount("hello world", TextShortcutPlatform::Android);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    assert!(shown_handles(&runtime).is_some());
    let _ = runtime.handle_input(InputEvent::Text("x".into()));
    runtime.run_frame(bounds()).unwrap();
    assert!(shown_handles(&runtime).is_none());
}

#[test]
fn ios_touch_drag_does_not_change_the_selection() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Ios);
    drag(
        &mut runtime,
        Offset::new(12., 10.),
        Offset::new(150., 10.),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    let selection = controller.selection();
    assert_eq!(
        selection.base, selection.extent,
        "iOS touch drag selects nothing"
    );
}

#[test]
fn android_touch_drag_selects_only_from_a_focused_field() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Android);
    let start = Instant::now();
    drag(
        &mut runtime,
        Offset::new(12., 10.),
        Offset::new(150., 10.),
        PointerDeviceKind::Touch,
        start,
    );
    let selection = controller.selection();
    assert_eq!(
        selection.base, selection.extent,
        "an unfocused field does not drag-select"
    );

    // The field has focus now, so the same drag selects.
    drag(
        &mut runtime,
        Offset::new(12., 10.),
        Offset::new(150., 10.),
        PointerDeviceKind::Touch,
        start + Duration::from_secs(2),
    );
    let selection = controller.selection();
    assert_ne!(
        selection.base, selection.extent,
        "a focused field drag-selects"
    );
}
