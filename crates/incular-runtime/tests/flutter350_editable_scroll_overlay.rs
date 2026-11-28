use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{
    EditableText, SizedBox, Widget,
    internal::{ElementId, Key},
};

fn pointer(runtime: &mut Runtime, phase: PointerPhase, position: Offset, at: Instant) {
    let terminal = matches!(phase, PointerPhase::Up | PointerPhase::Cancel);
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 2,
            kind: PointerDeviceKind::Touch,
            buttons: if terminal { 0 } else { PRIMARY_POINTER_BUTTON },
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            phase,
            position,
            sample: PointerSampleMetadata::default(),
        },
        at,
    );
}

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(400., 200.)), at)
        .unwrap();
}

fn toolbar_visible(runtime: &Runtime, field: ElementId) -> bool {
    runtime
        .tree()
        .text_field_toolbar_state(field)
        .unwrap()
        .is_showing()
}

#[test]
fn a_short_touch_scroll_hides_then_restores_the_toolbar_without_semantics() {
    const TOOLBAR: &str = "editor scroll test toolbar";
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let controller =
            TextEditingController::with_text("Lorem ipsum dolor sit amet ".repeat(200));
        let editor = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None)
            .context_menu_builder(|_, _| Widget::from(SizedBox::shrink()).with_key(TOOLBAR))
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.set_platform_semantics_enabled(false);
        assert!(!runtime.semantics_enabled());

        let clock = Instant::now();
        let time = |milliseconds| clock + Duration::from_millis(milliseconds);
        let middle = Offset::new(200., 100.);
        frame(&mut runtime, time(0));
        pointer(&mut runtime, PointerPhase::Down, middle, time(10));
        frame(&mut runtime, time(510));
        pointer(&mut runtime, PointerPhase::Up, middle, time(530));
        frame(&mut runtime, time(530));
        let field = runtime.focused_element().unwrap();
        let selection = controller.selection();
        assert!(!selection.is_collapsed());
        assert!(toolbar_visible(&runtime, field));
        let previous_offset = runtime.tree().text_field_scroll_offset(field).unwrap();

        pointer(&mut runtime, PointerPhase::Down, middle, time(1000));
        let moved = middle - Offset::new(0., 20.);
        pointer(&mut runtime, PointerPhase::Move, moved, time(1020));
        frame(&mut runtime, time(1020));
        assert!(
            runtime.tree().text_field_scroll_offset(field).unwrap().y > previous_offset.y,
            "the editor's viewport must consume the short drag ({platform:?})"
        );
        assert_eq!(controller.selection(), selection);
        assert!(!toolbar_visible(&runtime, field));

        pointer(&mut runtime, PointerPhase::Up, moved, time(1040));
        frame(&mut runtime, time(1040));
        assert!(
            runtime.frame_requested(),
            "restoration must schedule its own paint"
        );
        frame(&mut runtime, time(1060));
        assert!(
            runtime
                .tree()
                .element_with_key(&Key::from(TOOLBAR))
                .is_some()
        );
        assert!(toolbar_visible(&runtime, field));
        assert_eq!(controller.selection(), selection);
    }
}

fn started_with_toolbar() -> (Runtime, TextEditingController, ElementId, Instant) {
    let controller = TextEditingController::with_text("text ".repeat(10_000));
    let editor = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .max_lines(None)
        .context_menu_builder(|_, _| SizedBox::shrink().into())
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let clock = Instant::now();
    frame(&mut runtime, clock);
    let first_word = Offset::new(2., 8.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        first_word,
        clock + Duration::from_millis(10),
    );
    frame(&mut runtime, clock + Duration::from_millis(510));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        first_word,
        clock + Duration::from_millis(530),
    );
    frame(&mut runtime, clock + Duration::from_millis(530));
    let field = runtime.focused_element().unwrap();
    assert_eq!(
        controller.selection(),
        incular_text::TextSelection::new(0, 4)
    );
    assert!(toolbar_visible(&runtime, field));
    (runtime, controller, field, clock)
}

fn drag_away_from_selection(runtime: &mut Runtime, field: ElementId, clock: Instant) {
    let center = Offset::new(200., 100.);
    let moved = center - Offset::new(0., 200.);
    pointer(
        runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1000),
    );
    pointer(
        runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1020),
    );
    frame(runtime, clock + Duration::from_millis(1020));
    assert!(runtime.tree().text_field_scroll_offset(field).unwrap().y > 100.);
    assert!(!toolbar_visible(runtime, field));
    pointer(
        runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1040),
    );
}

#[test]
fn a_value_replacement_before_the_scroll_end_frame_never_reopens_the_old_toolbar() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    controller.set_text("a different value");
    for elapsed in [1040, 1060, 1200] {
        frame(&mut runtime, clock + Duration::from_millis(elapsed));
        assert!(!toolbar_visible(&runtime, field));
    }
    assert_eq!(controller.text(), "a different value");
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn replacing_the_document_between_two_scroll_contacts_does_not_reuse_old_geometry() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    controller.set_text("a different value");
    let center = Offset::new(200., 100.);
    let moved = center - Offset::new(0., 100.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1050),
    );
    pointer(
        &mut runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1070),
    );
    frame(&mut runtime, clock + Duration::from_millis(1070));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1090),
    );
    frame(&mut runtime, clock + Duration::from_millis(1090));
    assert!(!toolbar_visible(&runtime, field));
    assert_eq!(controller.text(), "a different value");
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn an_unchanged_toolbar_waits_until_its_selection_is_back_inside_the_editor() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    frame(&mut runtime, clock + Duration::from_millis(1040));
    assert!(!toolbar_visible(&runtime, field));

    let center = Offset::new(200., 100.);
    let moved = center + Offset::new(0., 200.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1100),
    );
    pointer(
        &mut runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1120),
    );
    frame(&mut runtime, clock + Duration::from_millis(1120));
    assert!(!toolbar_visible(&runtime, field));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1140),
    );
    frame(&mut runtime, clock + Duration::from_millis(1140));
    frame(&mut runtime, clock + Duration::from_millis(1160));
    assert!(toolbar_visible(&runtime, field));
    assert_eq!(
        controller.selection(),
        incular_text::TextSelection::new(0, 4)
    );
}
