use incular_config::{Constraints, MainAxisSize, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_text::{TextAlign, TextStyle};
use incular_widgets::{
    Center, Column, Directionality, SelectionAreaController, SizedBox, Text, Widget, internal::Key,
};

#[test]
fn captured_mouse_drag_keeps_selecting_beyond_labels_and_across_gaps() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned FlutterTest for static text drag";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let text = |value, key| {
        Widget::selectable_text_styled(
            value,
            TextStyle::new().font_size(10.).font_family(FONT),
            TextAlign::Start,
        )
        .with_key(key)
    };
    let plain_text = |value, key| {
        Widget::from(Text::new(value).style(TextStyle::new().font_size(10.).font_family(FONT)))
            .with_key(key)
    };
    for split in [false, true] {
        for forward in [true, false] {
            let controller = SelectionAreaController::new();
            let child = if split {
                Widget::from(
                    Column::new([
                        plain_text("ABCD", "first"),
                        SizedBox::new().height(24.).into(),
                        plain_text("EFGH", "last"),
                    ])
                    .main_axis_size(MainAxisSize::Min),
                )
            } else {
                text("ABCD\nEFGH", "first")
            };
            let mut runtime = Runtime::new(
                Directionality::new(
                    TextDirection::Ltr,
                    Widget::selection_area(controller.clone(), Center::new(child).into()),
                )
                .into(),
            )
            .unwrap();
            runtime
                .run_frame(Constraints::tight(Size::new(800., 600.)))
                .unwrap();
            let bounds = |key| {
                let node = runtime.tree().element_with_key(&Key::from(key)).unwrap();
                runtime.tree().element_bounds(node).unwrap()
            };
            let leading = bounds("first");
            let trailing = if split { bounds("last") } else { leading };
            let (right, bottom) = (
                trailing.origin.x + trailing.size.width,
                trailing.origin.y + trailing.size.height,
            );
            let start = if forward {
                leading.origin + Offset::new(1., 5.)
            } else {
                Offset::new(right - 1., bottom - 5.)
            };
            let goal = if split {
                Offset::new(
                    start.x,
                    if forward {
                        bottom + 30.
                    } else {
                        leading.origin.y - 30.
                    },
                )
            } else {
                Offset::new(
                    if forward {
                        right + 30.
                    } else {
                        leading.origin.x - 30.
                    },
                    start.y,
                )
            };
            assert!(runtime.tree().selectable_text_at(goal).is_none());
            let mut send = |phase, position| {
                let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
            };
            send(PointerPhase::Down, start);
            if split {
                let gap_y = (leading.origin.y + leading.size.height + trailing.origin.y) / 2.;
                send(PointerPhase::Move, Offset::new(start.x, gap_y));
                assert_eq!(
                    controller.selected_text(),
                    if forward { "ABCD" } else { "EFGH" }
                );
            }
            send(PointerPhase::Move, goal);
            send(PointerPhase::Up, goal);
            assert_eq!(
                controller.selected_text(),
                if split { "ABCDEFGH" } else { "ABCD\nEFGH" },
                "split={split} forward={forward}"
            );
        }
    }
}

#[test]
fn captured_drag_interprets_outside_points_in_the_rotated_paragraph_coordinates() {
    use incular_widgets::Transform;
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned FlutterTest for transformed static drag";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let label = Key::from("rotated text");
    let text = Widget::from(
        Text::new("ABCD\nEFGH").style(TextStyle::new().font_size(10.).font_family(FONT)),
    )
    .with_key(label.clone());
    let controller = SelectionAreaController::new();
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            Widget::selection_area(
                controller.clone(),
                Center::new(Transform::rotation(0.6, text)).into(),
            ),
        )
        .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let label = runtime.tree().element_with_key(&label).unwrap();
    let (world, size) = runtime.tree().element_geometry(label).unwrap();
    let start = world.transform_point(Offset::new(1., 5.));
    let goal = world.transform_point(Offset::new(size.width + 30., 5.));
    assert!(runtime.tree().selectable_text_at(goal).is_none());
    for (phase, position) in [
        (PointerPhase::Down, start),
        (PointerPhase::Move, goal),
        (PointerPhase::Up, goal),
    ] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    assert_eq!(controller.selected_text(), "ABCD\nEFGH");
}

#[test]
fn captured_drag_uses_rtl_logical_edges_instead_of_ltr_corners() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const SOURCE: &str = "אבגד\nהוזח";
    const FONT: &str = "Pinned FlutterTest for right-to-left drag";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let label = Key::from("RTL text");
    for forward in [true, false] {
        let controller = SelectionAreaController::new();
        let text = Widget::from(
            Text::new(SOURCE).style(TextStyle::new().font_size(10.).font_family(FONT)),
        )
        .with_key(label.clone());
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Rtl,
                Widget::selection_area(controller.clone(), Center::new(text).into()),
            )
            .into(),
        )
        .unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let label = runtime.tree().element_with_key(&label).unwrap();
        let rect = runtime.tree().element_bounds(label).unwrap();
        let right = rect.origin.x + rect.size.width;
        let start = if forward {
            Offset::new(right - 1., rect.origin.y + 5.)
        } else {
            Offset::new(rect.origin.x + 1., rect.origin.y + rect.size.height - 5.)
        };
        let goal = Offset::new(
            if forward {
                right + 30.
            } else {
                rect.origin.x - 30.
            },
            start.y,
        );
        assert!(runtime.tree().selectable_text_at(goal).is_none());
        for (phase, position) in [
            (PointerPhase::Down, start),
            (PointerPhase::Move, goal),
            (PointerPhase::Up, goal),
        ] {
            let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
        }
        assert_eq!(controller.selected_text(), SOURCE, "forward={forward}");
    }
}

#[test]
fn long_press_drag_extends_whole_words_off_the_label_in_both_directions() {
    use incular_core::{PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerSampleMetadata};
    use incular_runtime::TextShortcutPlatform;
    use std::time::{Duration, Instant};

    let Some(path) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const TEXT: &str = "alpha beta\nomega zeta";
    const FONT: &str = "Pinned FlutterTest for off-label long press";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let frame = Constraints::tight(Size::new(800., 600.));
    let key = Key::from("touch text");
    for forward in [true, false] {
        let controller = SelectionAreaController::new();
        let child =
            Widget::from(Text::new(TEXT).style(TextStyle::new().font_family(FONT).font_size(10.)))
                .with_key(key.clone());
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                Widget::selection_area(controller.clone(), Center::new(child).into()),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
        let down = Instant::now();
        runtime.run_frame_at(frame, down).unwrap();
        let node = runtime.tree().element_with_key(&key).unwrap();
        let bounds = runtime.tree().element_bounds(node).unwrap();
        let right = bounds.origin.x + bounds.size.width;
        let start = if forward {
            bounds.origin + Offset::new(15., 5.)
        } else {
            Offset::new(right - 15., bounds.origin.y + bounds.size.height - 5.)
        };
        let goal = Offset::new(
            if forward {
                right + 30.
            } else {
                bounds.origin.x - 30.
            },
            start.y,
        );
        let send = |runtime: &mut Runtime, phase, position, at| {
            let active = phase != PointerPhase::Up;
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 3,
                    kind: PointerDeviceKind::Touch,
                    buttons: if active { PRIMARY_POINTER_BUTTON } else { 0 },
                    button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position,
                },
                at,
            );
        };
        send(&mut runtime, PointerPhase::Down, start, down);
        let held = down + Duration::from_millis(510);
        runtime.run_frame_at(frame, held).unwrap();
        assert_eq!(
            controller.selected_text(),
            if forward { "alpha" } else { "zeta" }
        );
        send(&mut runtime, PointerPhase::Move, goal, held);
        assert_eq!(controller.selected_text(), TEXT, "forward={forward}");
        send(
            &mut runtime,
            PointerPhase::Up,
            goal,
            held + Duration::from_millis(1),
        );
        assert_eq!(controller.selected_text(), TEXT);
    }
}
