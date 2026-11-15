//! Flutter's long-press selection. `TextSelectionGestureDetector` on an
//! `EditableText` selects the word under a held press on Android, Fuchsia,
//! Linux and Windows (`onSingleLongTapStart`), places the caret on iOS and
//! macOS, and extends the selection by word while the press drags
//! (`onSingleLongTapMoveUpdate`). `SelectableRegion` does the same for touch
//! and stylus presses only (`_handleTouchLongPressStart`), and reports the
//! content through `SelectionArea.onSelectionChanged`.
//!
//! The 500 ms deadline is `kLongPressTimeout`, measured on the frame clock.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextStyle;
use incular_widgets::{
    EditableText, TextDirection, Widget,
    internal::{SelectionAreaController, TextEditingController, TextSelection},
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 120.))
}

fn mount_field(text: &str, platform: TextShortcutPlatform) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.));
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn touch(
    runtime: &mut Runtime,
    phase: PointerPhase,
    position: Offset,
    kind: PointerDeviceKind,
    at: Instant,
) {
    let (buttons, button) = match phase {
        PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
        PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
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

/// Holds a press at `position` for `held`, running frames on the way.
fn hold(
    runtime: &mut Runtime,
    position: Offset,
    kind: PointerDeviceKind,
    held: Duration,
) -> Instant {
    let start = Instant::now();
    touch(runtime, PointerPhase::Down, position, kind, start);
    runtime.run_frame_at(bounds(), start + held).unwrap();
    start + held
}

#[test]
fn text_field_long_press_selects_the_word_on_windows() {
    let (mut runtime, controller) = mount_field("hello world", TextShortcutPlatform::Windows);
    let at = hold(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
        Duration::from_millis(600),
    );
    touch(
        &mut runtime,
        PointerPhase::Up,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
        at,
    );
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
}

#[test]
fn text_field_long_press_waits_for_the_long_press_deadline() {
    let (mut runtime, controller) = mount_field("hello world", TextShortcutPlatform::Windows);
    hold(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
        Duration::from_millis(400),
    );
    assert!(controller.selection().is_collapsed());
}

#[test]
fn text_field_long_press_on_macos_places_the_caret_when_focused() {
    let (mut runtime, controller) = mount_field("hello world", TextShortcutPlatform::MacOs);
    // A first press focuses the field; the long press then places a caret.
    let start = Instant::now();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        touch(
            &mut runtime,
            phase,
            Offset::new(12., 10.),
            PointerDeviceKind::Mouse,
            start,
        );
    }
    runtime.run_frame(bounds()).unwrap();
    let at = hold(
        &mut runtime,
        Offset::new(60., 10.),
        PointerDeviceKind::Touch,
        Duration::from_millis(600),
    );
    touch(
        &mut runtime,
        PointerPhase::Up,
        Offset::new(60., 10.),
        PointerDeviceKind::Touch,
        at,
    );
    runtime.run_frame(bounds()).unwrap();
    assert!(
        controller.selection().is_collapsed(),
        "iOS and macOS place the caret on a focused field's long press"
    );
}

#[test]
fn text_field_long_press_cancels_when_the_pointer_leaves_the_slop() {
    let (mut runtime, controller) = mount_field("hello world", TextShortcutPlatform::Windows);
    let start = Instant::now();
    touch(
        &mut runtime,
        PointerPhase::Down,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
        start,
    );
    // Moving 40 px, beyond the 18 px touch slop, turns the press into a drag.
    touch(
        &mut runtime,
        PointerPhase::Move,
        Offset::new(52., 10.),
        PointerDeviceKind::Touch,
        start + Duration::from_millis(100),
    );
    runtime
        .run_frame_at(bounds(), start + Duration::from_millis(600))
        .unwrap();
    assert!(
        controller.selection().start() != 0 || controller.selection().end() != 5,
        "a dragged press must not select the word under the press"
    );
}

#[test]
fn text_field_long_press_drag_extends_by_word() {
    let (mut runtime, controller) = mount_field("hello big world", TextShortcutPlatform::Windows);
    let start = Instant::now();
    touch(
        &mut runtime,
        PointerPhase::Down,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
        start,
    );
    let held = start + Duration::from_millis(600);
    runtime.run_frame_at(bounds(), held).unwrap();
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    touch(
        &mut runtime,
        PointerPhase::Move,
        Offset::new(100., 10.),
        PointerDeviceKind::Touch,
        held,
    );
    runtime.run_frame_at(bounds(), held).unwrap();
    let selection = controller.selection();
    assert_eq!(selection.start(), 0);
    assert_eq!(
        selection.end(),
        15,
        "the drag extends to the word under the pointer"
    );
}

fn mount_area(
    text: &str,
    platform: TextShortcutPlatform,
) -> (
    Runtime,
    SelectionAreaController,
    Rc<RefCell<Vec<Option<String>>>>,
) {
    let controller = SelectionAreaController::new();
    let changes: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
    controller.set_selection_changed_listener({
        let changes = changes.clone();
        move |content| {
            changes
                .borrow_mut()
                .push(content.map(|content| content.plain_text));
        }
    });
    let mut runtime = Runtime::new(Widget::selection_area(
        controller.clone(),
        Widget::selectable_text_styled(text, TextStyle::default(), incular_text::TextAlign::Start),
    ))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller, changes)
}

#[test]
fn selection_area_touch_long_press_selects_the_word_and_reports_it() {
    let (mut runtime, controller, changes) =
        mount_area("alpha beta", TextShortcutPlatform::Android);
    let at = hold(
        &mut runtime,
        Offset::new(4., 8.),
        PointerDeviceKind::Touch,
        Duration::from_millis(600),
    );
    touch(
        &mut runtime,
        PointerPhase::Up,
        Offset::new(4., 8.),
        PointerDeviceKind::Touch,
        at,
    );
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(controller.selected_text(), "alpha");
    assert_eq!(changes.borrow().as_slice(), [Some("alpha".to_owned())]);
}

#[test]
fn selection_area_mouse_press_does_not_long_press() {
    let (mut runtime, controller, _changes) =
        mount_area("alpha beta", TextShortcutPlatform::Windows);
    hold(
        &mut runtime,
        Offset::new(4., 8.),
        PointerDeviceKind::Mouse,
        Duration::from_millis(600),
    );
    assert!(
        controller.selected_text().is_empty(),
        "SelectableRegion long press accepts touch and stylus only"
    );
}
