use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{
    SelectionChangedCause, TextEditingController, TextSelection, TextStyle,
    register_application_font_as,
};
use incular_widgets::{EditableText, RawPointerEvent, SizedBox};

fn ahem_style() -> Option<TextStyle> {
    const AHEM: &str = "Pinned pointer editing Ahem";
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.tmp/flutter/packages/flutter_tools/static/Ahem.ttf")
        });
    if !path.is_file() {
        eprintln!("Pinned Ahem unavailable; pointer geometry not run");
        return None;
    }
    register_application_font_as(AHEM, std::fs::read(path).unwrap());
    Some(TextStyle::new().font_family(AHEM).font_size(14.))
}

#[test]
fn physical_taps_drags_and_long_press_report_their_selection_causes() {
    use PointerDeviceKind::{Mouse, Touch};
    use PointerPhase::{Down, Move, Up};
    use SelectionChangedCause::{DoubleTap, Drag, LongPress, Tap};

    let Some(style) = ahem_style() else {
        return;
    };
    let controller = TextEditingController::with_text("alpha beta gamma");
    controller.set_selection(TextSelection::collapsed(0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .style(style)
        .context_menu_builder(|_, _| SizedBox::square(10.).into())
        .on_selection_changed({
            let changes = changes.clone();
            move |selection, cause| changes.borrow_mut().push((selection, cause))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    let constraints = Constraints::loose(Size::new(800., 100.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    assert!(runtime.focused_element().is_some());
    assert!(changes.borrow().is_empty());

    let send = |runtime: &mut Runtime, time, pointer, kind, phase, x| {
        let event = RawPointerEvent::new(
            pointer,
            if kind == Mouse { 7 } else { 8 },
            kind,
            u32::from(phase != Up),
            Offset::new(x, 7.),
            phase,
            base + Duration::from_millis(time),
        )
        .with_button(matches!(phase, Down | Up).then_some(PRIMARY_POINTER_BUTTON));
        let _ = runtime.handle_raw_pointer_event(event);
    };
    let check = |selection, cause| {
        assert_eq!(controller.selection(), selection);
        assert_eq!(changes.borrow().last(), Some(&(selection, cause)));
    };

    send(&mut runtime, 0, 1, Touch, Down, 30.);
    send(&mut runtime, 50, 1, Touch, Up, 30.);
    check(TextSelection::collapsed(2), Tap);
    send(&mut runtime, 60, 2, Touch, Down, 30.);
    send(&mut runtime, 110, 2, Touch, Up, 30.);
    check(TextSelection::new(0, 5), DoubleTap);
    let field = runtime.focused_element().unwrap();
    let menu = runtime.tree().text_field_context_menu(field).unwrap();
    assert!(
        menu.is_showing(),
        "a touch double tap opens the selection toolbar"
    );
    menu.hide();

    send(&mut runtime, 710, 3, Mouse, Down, 16.);
    send(&mut runtime, 725, 3, Mouse, Move, 48.);
    send(&mut runtime, 740, 3, Mouse, Move, 114.);
    send(&mut runtime, 750, 3, Mouse, Up, 114.);
    check(TextSelection::new(1, 8), Drag);

    send(&mut runtime, 1350, 4, Touch, Down, 100.);
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(1900))
        .unwrap();
    send(&mut runtime, 1910, 4, Touch, Up, 100.);
    check(TextSelection::new(6, 10), LongPress);
}

#[test]
fn the_renderer_can_leave_pointer_selection_and_focus_to_its_caller() {
    use incular_core::InputEvent;
    use incular_widgets::FocusNode;

    let Some(style) = ahem_style() else {
        return;
    };
    let controller = TextEditingController::with_text("alpha beta gamma");
    controller.set_selection(TextSelection::collapsed(1));
    let focus = FocusNode::new();
    let causes = Rc::new(RefCell::new(Vec::new()));
    let build = |ignore| {
        EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .renderer_ignores_pointer(ignore)
            .style(style.clone())
            .on_selection_changed({
                let causes = causes.clone();
                move |_, cause| causes.borrow_mut().push(cause)
            })
    };
    let mut runtime = Runtime::new(build(true).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    let constraints = Constraints::loose(Size::new(800., 100.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    let field = runtime.focused_element().unwrap();
    let render = runtime.tree().render_id(field).unwrap();
    assert!(focus.has_primary_focus());

    let tap = |runtime: &mut Runtime, position: Offset, milliseconds: u64| {
        for (phase, duration) in [(PointerPhase::Down, 0), (PointerPhase::Up, 20)] {
            let _ = runtime.handle_input_at(
                InputEvent::Pointer { phase, position },
                base + Duration::from_millis(milliseconds + duration),
            );
        }
    };
    let point = Offset::new(58., 7.);
    assert_eq!(
        runtime
            .tree()
            .text_field_position_at(field, point)
            .unwrap()
            .0,
        4
    );
    tap(&mut runtime, point, 100);
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    assert!(causes.borrow().is_empty());
    assert!(focus.has_primary_focus());

    focus.unfocus();
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(150))
        .unwrap();
    tap(&mut runtime, point, 200);
    assert!(runtime.focused_element().is_none());
    assert_eq!(controller.selection(), TextSelection::collapsed(1));

    focus.request_focus();
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(250))
        .unwrap();
    assert!(runtime.select_editable_position_at_with_cause(
        field,
        point,
        None,
        SelectionChangedCause::Tap
    ));
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    assert_eq!(causes.borrow().as_slice(), &[SelectionChangedCause::Tap]);

    runtime.schedule_update(field, build(false).into()).unwrap();
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(300))
        .unwrap();
    assert_eq!(runtime.tree().render_id(field), Some(render));
    assert!(focus.has_primary_focus());
    tap(&mut runtime, Offset::new(30., 7.), 350);
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
    assert_eq!(
        causes.borrow().as_slice(),
        &[SelectionChangedCause::Tap, SelectionChangedCause::Tap]
    );
}

#[test]
fn an_android_finger_does_not_move_a_focused_caret_until_the_tap_is_released() {
    let Some(style) = ahem_style() else {
        return;
    };
    let controller = TextEditingController::with_text("alpha beta gamma");
    controller.set_selection(TextSelection::collapsed(1));
    let causes = Rc::new(RefCell::new(Vec::new()));
    let focus = incular_widgets::FocusNode::new();
    let editor = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .style(style)
        .on_selection_changed({
            let causes = causes.clone();
            move |selection, cause| causes.borrow_mut().push((selection, cause))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let constraints = Constraints::loose(Size::new(800., 100.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    let field = runtime.focused_element().unwrap();
    let point = Offset::new(58., 7.);
    let send = |runtime: &mut Runtime, id, at, phase| {
        let changed_button = matches!(phase, PointerPhase::Down | PointerPhase::Up)
            .then_some(PRIMARY_POINTER_BUTTON);
        let _ = runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                id,
                3,
                PointerDeviceKind::Touch,
                u32::from(phase == PointerPhase::Down),
                point,
                phase,
                base + Duration::from_millis(at),
            )
            .with_button(changed_button),
        );
    };
    send(&mut runtime, 1, 0, PointerPhase::Down);
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(100))
        .unwrap();
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    assert!(causes.borrow().is_empty());
    send(&mut runtime, 1, 110, PointerPhase::Cancel);
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    assert!(causes.borrow().is_empty());
    assert_eq!(runtime.focused_element(), Some(field));

    send(&mut runtime, 2, 700, PointerPhase::Down);
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(800))
        .unwrap();
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    assert!(causes.borrow().is_empty());
    send(&mut runtime, 2, 810, PointerPhase::Up);
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    assert_eq!(
        causes.borrow().as_slice(),
        &[(TextSelection::collapsed(4), SelectionChangedCause::Tap)]
    );
    assert_eq!(runtime.focused_element(), Some(field));

    controller.set_selection(TextSelection::collapsed(1));
    causes.borrow_mut().clear();
    send(&mut runtime, 3, 1400, PointerPhase::Down);
    focus.unfocus();
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(1450))
        .unwrap();
    send(&mut runtime, 3, 1490, PointerPhase::Up);
    assert!(runtime.focused_element().is_none());
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    assert!(causes.borrow().is_empty());
}

#[test]
fn ios_touch_taps_keep_a_selected_range_or_place_the_caret_at_a_word_edge() {
    let Some(style) = ahem_style() else {
        return;
    };
    let controller = TextEditingController::with_text("alpha beta gamma");
    let selected = TextSelection::new(1, 4);
    controller.set_selection(selected);
    let causes = Rc::new(RefCell::new(Vec::new()));
    let editor = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .autofocus(true)
        .style(style)
        .context_menu_builder(|_, _| {
            incular_widgets::IgnorePointer::new(SizedBox::square(10.)).into()
        })
        .on_selection_changed({
            let causes = causes.clone();
            move |_, cause| causes.borrow_mut().push(cause)
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let constraints = Constraints::loose(Size::new(800., 100.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    let field = runtime.focused_element().unwrap();
    let menu = runtime.tree().text_field_context_menu(field).unwrap();
    let tap = |runtime: &mut Runtime, id: u64, milliseconds: u64, x: f32| {
        for (phase, offset) in [(PointerPhase::Down, 0), (PointerPhase::Up, 50)] {
            let _ = runtime.handle_raw_pointer_event(
                RawPointerEvent::new(
                    id,
                    2,
                    PointerDeviceKind::Touch,
                    u32::from(phase == PointerPhase::Down),
                    Offset::new(x, 7.),
                    phase,
                    base + Duration::from_millis(milliseconds + offset),
                )
                .with_button(Some(PRIMARY_POINTER_BUTTON)),
            );
        }
        runtime
            .run_frame_at(constraints, base + Duration::from_millis(milliseconds + 60))
            .unwrap();
    };
    tap(&mut runtime, 1, 0, 30.);
    assert_eq!(controller.selection(), selected);
    assert!(menu.is_showing());
    assert!(causes.borrow().is_empty());

    tap(&mut runtime, 2, 700, 30.);
    assert_eq!(controller.selection(), selected);
    assert!(!menu.is_showing());
    assert!(causes.borrow().is_empty());

    controller.set_selection(TextSelection::collapsed(0));
    runtime
        .run_frame_at(constraints, base + Duration::from_millis(800))
        .unwrap();
    tap(&mut runtime, 3, 1400, 100.);
    assert_eq!(controller.selection(), TextSelection::collapsed(10));
    assert_eq!(
        controller.caret_affinity(),
        incular_text::TextAffinity::Upstream
    );
    assert_eq!(causes.borrow().as_slice(), &[SelectionChangedCause::Tap]);
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn non_ios_platforms_tap_to_a_precise_caret_instead_of_toggling_the_toolbar() {
    let Some(style) = ahem_style() else {
        return;
    };
    // Android and Fuchsia use the same caret policy in the runtime.
    for platform in [
        TextShortcutPlatform::Android,
        TextShortcutPlatform::Linux,
        TextShortcutPlatform::MacOs,
        TextShortcutPlatform::Windows,
    ] {
        let controller = TextEditingController::with_text("alpha beta gamma");
        let changes = Rc::new(RefCell::new(Vec::new()));
        let editor = EditableText::new(controller.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .style(style.clone())
            .context_menu_builder(|_, _| SizedBox::square(10.).into())
            .on_selection_changed({
                let changes = changes.clone();
                move |selection, cause| changes.borrow_mut().push((selection, cause))
            });
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        let constraints = Constraints::loose(Size::new(800., 100.));
        let base = Instant::now();
        runtime.run_frame_at(constraints, base).unwrap();
        let field = runtime.focused_element().unwrap();
        let toolbar = runtime.tree().text_field_context_menu(field).unwrap();
        for (index, initial, x, expected) in [
            (0, TextSelection::collapsed(0), 100., 7),
            (1, TextSelection::new(1, 4), 30., 2),
        ] {
            let at = base + Duration::from_millis(600 * index);
            controller.set_selection(initial);
            runtime.run_frame_at(constraints, at).unwrap();
            changes.borrow_mut().clear();

            for (phase, advance) in [(PointerPhase::Down, 0), (PointerPhase::Up, 50)] {
                runtime.handle_raw_pointer_event(
                    RawPointerEvent::new(
                        index + 1,
                        2,
                        PointerDeviceKind::Touch,
                        u32::from(phase == PointerPhase::Down),
                        Offset::new(x, 7.),
                        phase,
                        at + Duration::from_millis(advance),
                    )
                    .with_button(Some(PRIMARY_POINTER_BUTTON)),
                );
            }
            runtime
                .run_frame_at(constraints, at + Duration::from_millis(60))
                .unwrap();

            let expected = TextSelection::collapsed(expected);
            assert_eq!(controller.selection(), expected, "{platform:?}");
            assert_eq!(
                changes.borrow().as_slice(),
                &[(expected, SelectionChangedCause::Tap)],
                "{platform:?}",
            );
            assert!(!toolbar.is_showing(), "{platform:?}");
            assert_eq!(runtime.focused_element(), Some(field), "{platform:?}");
        }
    }
}

#[test]
fn a_mobile_shift_tap_uses_the_modifiers_captured_at_press() {
    use incular_core::{Code, InputEvent, KeyState, KeyboardEvent, KeyboardKey, NamedKey};

    let Some(style) = ahem_style() else {
        return;
    };
    for (platform, anchor) in [
        (TextShortcutPlatform::Android, 2),
        (TextShortcutPlatform::Ios, 5),
    ] {
        let initial = TextSelection::new(2, 5);
        let controller = TextEditingController::with_text("alpha beta gamma");
        controller.set_selection(initial);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let editor = EditableText::new(controller.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .style(style.clone())
            .on_selection_changed({
                let changes = changes.clone();
                move |selection, cause| changes.borrow_mut().push((selection, cause))
            });
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        let now = Instant::now();
        runtime.run_frame_at(Constraints::loose(Size::new(800., 100.)), now).unwrap();
        let mut shift = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Shift), Code::ShiftLeft);
        let _ = runtime.handle_input_at(InputEvent::Key(shift.clone()), now);

        let _ = runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                1, 2, PointerDeviceKind::Touch, PRIMARY_POINTER_BUTTON,
                Offset::new(16., 7.), PointerPhase::Down, now,
            ).with_button(Some(PRIMARY_POINTER_BUTTON)),
        );
        assert_eq!(controller.selection(), initial, "{platform:?} pressed");
        assert!(changes.borrow().is_empty(), "{platform:?} pressed");

        shift.state = KeyState::Up;
        let _ = runtime.handle_input_at(InputEvent::Key(shift), now + Duration::from_millis(60));
        let _ = runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                1, 2, PointerDeviceKind::Touch, 0, Offset::new(16., 7.),
                PointerPhase::Up, now + Duration::from_millis(80),
            ).with_button(Some(PRIMARY_POINTER_BUTTON)),
        );
        let expected = TextSelection::new(anchor, 1);
        assert_eq!(controller.selection(), expected, "{platform:?} released");
        assert_eq!(
            changes.borrow().as_slice(),
            &[(expected, SelectionChangedCause::Tap)],
            "{platform:?} released",
        );
    }
}

#[test]
fn ios_mouse_places_a_precise_caret_only_after_a_confirmed_tap() {
    let Some(style) = ahem_style() else {
        return;
    };
    let initial = TextSelection::new(2, 5);
    let controller = TextEditingController::with_text("alpha beta gamma");
    controller.set_selection(initial);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .style(style)
        .on_selection_changed({
            let changes = changes.clone();
            move |selection, cause| changes.borrow_mut().push((selection, cause))
        });
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let now = Instant::now();
    runtime.run_frame_at(Constraints::loose(Size::new(800., 100.)), now).unwrap();
    let mut button = |id, phase, ms| {
        let _ = runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                id, 3, PointerDeviceKind::Mouse, u32::from(phase == PointerPhase::Down),
                Offset::new(58., 7.), phase, now + Duration::from_millis(ms),
            ).with_button(Some(PRIMARY_POINTER_BUTTON)),
        );
    };
    button(1, PointerPhase::Down, 0);
    assert_eq!(controller.selection(), initial);
    assert!(changes.borrow().is_empty());
    button(1, PointerPhase::Cancel, 100);
    assert_eq!(controller.selection(), initial);
    assert!(changes.borrow().is_empty());

    button(2, PointerPhase::Down, 700);
    assert_eq!(controller.selection(), initial);
    button(2, PointerPhase::Up, 780);
    let caret = TextSelection::collapsed(4);
    assert_eq!(controller.selection(), caret);
    assert_eq!(changes.borrow().as_slice(), &[(caret, SelectionChangedCause::Tap)]);
}
