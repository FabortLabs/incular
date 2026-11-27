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
