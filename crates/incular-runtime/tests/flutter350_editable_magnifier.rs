use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextAlign, TextEditingController, TextStyle};
use incular_widgets::{
    Align, EditableText, FocusNode, MagnifierInfo, SizedBox, TextMagnifierConfiguration,
    TextSelectionHandleLayout, Transform, Widget, internal::Key,
};

fn pointer(runtime: &mut Runtime, phase: PointerPhase, position: Offset, at: Instant) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 2,
            kind: PointerDeviceKind::Touch,
            buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            phase,
            position,
            sample: PointerSampleMetadata::default(),
        },
        at,
    );
}

#[test]
fn editable_text_has_no_default_magnifier() {
    let field = EditableText::new(TextEditingController::new()).text_direction(TextDirection::Ltr);
    assert!(
        field
            .configured_magnifier()
            .build(MagnifierInfo::EMPTY)
            .is_none()
    );
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert!(
        !runtime
            .tree()
            .text_field_has_magnifier(runtime.tree().root().unwrap())
    );
}

#[test]
fn half_scale_handle_drag_changes_the_magnifier_line_only_at_one_local_line() {
    const MAGNIFIER: &str = "neutral magnifier";
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "FlutterTest editable magnifier";
    incular_text::register_application_font_as(FONT, std::fs::read(font).unwrap());
    let controller = TextEditingController::with_text("hello \n world \n this \n is \n text");
    let focus = FocusNode::new();
    let infos = Rc::new(RefCell::new(Vec::<MagnifierInfo>::new()));
    let configuration = TextMagnifierConfiguration::new({
        let infos = infos.clone();
        move |info| {
            infos.borrow_mut().push(info);
            let lens = Widget::from(SizedBox::new().width(20.).height(10.)).with_key(MAGNIFIER);
            Some(
                Align::new(
                    Alignment::TOP_LEFT,
                    Transform::translation(
                        Offset::new(info.global_gesture_position.x, info.caret_rect.origin.y),
                        lens,
                    ),
                )
                .into(),
            )
        }
    })
    .should_display_handles_in_magnifier(false);
    let editor = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right)
        .max_lines(None)
        .style(TextStyle::new().font_family(FONT).font_size(14.))
        .selection_handles_builder(|_, _| SizedBox::shrink().into())
        .collapsed_selection_handle_layout(TextSelectionHandleLayout::new(
            Size::new(4., 4.),
            Offset::new(2., 0.),
        ))
        .magnifier_configuration(configuration);
    let mut runtime =
        Runtime::new(Align::new(Alignment::CENTER, Transform::scale(0.5, editor)).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let frame = |runtime: &mut Runtime, at| {
        runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), at)
            .unwrap();
    };
    let clock = Instant::now();
    frame(&mut runtime, clock);
    let field = runtime.tree().text_field_elements().next().unwrap();
    assert!(runtime.request_text_input_focus_for(field));
    frame(&mut runtime, clock);
    let caret = runtime
        .tree()
        .text_field_offset_rect(field, 3, TextAffinity::Downstream)
        .unwrap();
    let tap = caret.origin + Offset::new(0., caret.size.height * 0.5);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        tap,
        clock + Duration::from_millis(10),
    );
    pointer(
        &mut runtime,
        PointerPhase::Up,
        tap,
        clock + Duration::from_millis(30),
    );
    frame(&mut runtime, clock + Duration::from_millis(40));
    let handles = runtime
        .tree()
        .text_field_selection_handles(field)
        .unwrap()
        .handles()
        .unwrap();
    assert!(handles.end.is_none());
    let contact = handles.start.anchor - Offset::new(0., 8.);
    let bounds = |runtime: &Runtime| {
        runtime
            .tree()
            .element_with_key(&Key::from(MAGNIFIER))
            .and_then(|id| runtime.tree().element_bounds(id))
    };

    assert!(bounds(&runtime).is_none());
    pointer(
        &mut runtime,
        PointerPhase::Down,
        contact,
        clock + Duration::from_millis(400),
    );
    frame(&mut runtime, clock + Duration::from_millis(600));
    let initial = bounds(&runtime).unwrap().origin.y;
    let info = *infos.borrow().last().unwrap();
    assert_eq!(info.field_bounds.size.width, 400.);
    assert_eq!(info.caret_rect.size.height, 7.);
    assert_eq!(info.current_line_boundaries.size.height, 7.);

    for (index, (dy, expected_delta)) in [(3.5, 0.), (7., 7.), (3.5, 7.), (0., 0.)]
        .into_iter()
        .enumerate()
    {
        let at = clock + Duration::from_millis(620 + index as u64 * 20);
        pointer(
            &mut runtime,
            PointerPhase::Move,
            contact + Offset::new(0., dy),
            at,
        );
        frame(&mut runtime, at);
        assert_eq!(bounds(&runtime).unwrap().origin.y, initial + expected_delta);
    }
    pointer(
        &mut runtime,
        PointerPhase::Up,
        contact,
        clock + Duration::from_millis(750),
    );
    frame(&mut runtime, clock + Duration::from_millis(770));
    assert!(bounds(&runtime).is_none());
    // Cancel and focus loss both remove a showing lens. Neither should
    // leave a selection portal alive after the input owner was retired.
    for (index, blur) in [false, true].into_iter().enumerate() {
        let at = clock + Duration::from_millis(900 + index as u64 * 600);
        pointer(&mut runtime, PointerPhase::Down, contact, at);
        frame(&mut runtime, at + Duration::from_millis(100));
        assert!(bounds(&runtime).is_some());
        if blur {
            focus.unfocus();
        } else {
            pointer(
                &mut runtime,
                PointerPhase::Cancel,
                contact,
                at + Duration::from_millis(110),
            );
        }
        frame(&mut runtime, at + Duration::from_millis(120));
        assert!(bounds(&runtime).is_none());
    }
}

#[test]
fn toggling_a_builder_preserves_the_mounted_editor_when_handles_are_configured() {
    let controller = TextEditingController::with_text("example");
    let build = |enabled| {
        let plain = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let field = if enabled {
            plain.magnifier_configuration(TextMagnifierConfiguration::new(|_| {
                Some(SizedBox::shrink().into())
            }))
        } else {
            plain
        };
        Widget::from(Align::new(Alignment::CENTER, field))
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(viewport).unwrap();
    let field = runtime.tree().text_field_elements().next().unwrap();
    assert!(runtime.request_text_input_focus_for(field));
    for enabled in [true, false, true] {
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, build(enabled)).unwrap();
        runtime.run_frame(viewport).unwrap();
        assert_eq!(runtime.tree().text_field_elements().next(), Some(field));
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(runtime.tree().text_field_has_magnifier(field), enabled);
    }
}

#[test]
fn a_custom_lens_can_show_without_a_handle_overlay_and_updates_its_key() {
    let controller = TextEditingController::with_text("abc");
    let current_key = Rc::new(Cell::new("lens one"));
    let geometry = Rc::new(Cell::new(MagnifierInfo::EMPTY));
    let configuration = TextMagnifierConfiguration::new({
        let current_key = current_key.clone();
        let geometry = geometry.clone();
        move |info| {
            geometry.set(info);
            Some(Widget::from(SizedBox::shrink()).with_key(current_key.get()))
        }
    })
    .should_display_handles_in_magnifier(false);
    let root = || {
        Widget::from(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .magnifier_configuration(configuration.clone()),
        )
    };
    let viewport = Constraints::tight(Size::new(200., 50.));
    let mut runtime = Runtime::new(root()).unwrap();
    runtime.run_frame(viewport).unwrap();
    let field = runtime.tree().text_field_elements().next().unwrap();
    assert!(runtime.tree().text_field_selection_handles(field).is_none());
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("lens one"))
            .is_none()
    );

    let position = Offset::new(10., 10.);
    assert!(runtime.show_editable_magnifier(field, position));
    runtime.run_frame(viewport).unwrap();
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("lens one"))
            .is_some()
    );
    assert_eq!(geometry.get().global_gesture_position, position);
    assert_eq!(geometry.get().field_bounds.origin, Offset::ZERO);
    assert_eq!(geometry.get().field_bounds.size, Size::new(200., 50.));
    assert!(
        runtime.focused_element().is_none(),
        "showing a lens does not request focus"
    );

    current_key.set("lens two");
    let element = runtime.tree().root().unwrap();
    runtime.schedule_update(element, root()).unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(runtime.tree().text_field_elements().next(), Some(field));
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("lens one"))
            .is_none()
    );
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("lens two"))
            .is_some()
    );
    assert!(runtime.hide_editable_magnifier(field));
    runtime.run_frame(viewport).unwrap();
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("lens two"))
            .is_none()
    );
}

#[test]
fn touch_long_press_updates_a_configured_lens_until_release_on_mobile_only() {
    for (platform, expected) in [
        (TextShortcutPlatform::Android, true),
        (TextShortcutPlatform::Ios, true),
        (TextShortcutPlatform::Linux, false),
    ] {
        let controller = TextEditingController::with_text("alpha beta gamma");
        let geometry = Rc::new(Cell::new(MagnifierInfo::EMPTY));
        let editor = EditableText::new(controller)
            .text_direction(TextDirection::Ltr)
            .magnifier_configuration(TextMagnifierConfiguration::new({
                let geometry = geometry.clone();
                move |info| {
                    geometry.set(info);
                    Some(Widget::from(SizedBox::shrink()).with_key("long press lens"))
                }
            }));
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        let viewport = Constraints::tight(Size::new(200., 50.));
        let clock = Instant::now();
        runtime.run_frame_at(viewport, clock).unwrap();
        let field = runtime.tree().text_field_elements().next().unwrap();
        assert!(runtime.request_text_input_focus_for(field));
        runtime.run_frame_at(viewport, clock).unwrap();
        let at = |milliseconds| clock + Duration::from_millis(milliseconds);
        let visible = |runtime: &Runtime| {
            runtime
                .tree()
                .element_with_key(&Key::from("long press lens"))
                .is_some()
        };

        let down = Offset::new(18., 22.);
        pointer(&mut runtime, PointerPhase::Down, down, at(20));
        runtime.run_frame_at(viewport, at(519)).unwrap();
        assert!(!visible(&runtime));
        runtime.run_frame_at(viewport, at(520)).unwrap();
        assert_eq!(visible(&runtime), expected);
        if expected {
            assert_eq!(geometry.get().global_gesture_position, down);
        }
        let moved = Offset::new(70., 22.);
        pointer(&mut runtime, PointerPhase::Move, moved, at(560));
        runtime.run_frame_at(viewport, at(560)).unwrap();
        assert_eq!(visible(&runtime), expected);
        if expected {
            assert_eq!(geometry.get().global_gesture_position, moved);
        }
        pointer(&mut runtime, PointerPhase::Up, moved, at(580));
        runtime.run_frame_at(viewport, at(580)).unwrap();
        assert!(!visible(&runtime));
    }
}
