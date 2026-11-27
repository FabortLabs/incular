//! Flutter 3.50.0 selectable-region mouse cursor and trailing newline behavior.
use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_text::TextStyle;
use incular_widgets::{
    Border, BoxDecoration, Center, Container, Directionality, MouseCursor, MouseRegion, Padding,
    SelectableRegion, SelectedContentRange, SelectionAreaController, SelectionStatus, SizedBox,
    Text, Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn mounted(scene: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    runtime
}

#[test]
fn an_empty_region_defers_to_ancestor_and_child_cursors() {
    let inner = Widget::from(
        MouseRegion::new(SizedBox::new().width(200.).height(50.)).cursor(MouseCursor::NotAllowed),
    )
    .with_key(1_u64);
    let bordered = Container::with_child(SelectableRegion::new(Padding::all(40., inner)))
        .decoration(BoxDecoration::new().border(Border::new(1., Color::BLACK)));
    let mut runtime = mounted(MouseRegion::new(Center::new(bordered)).cursor(MouseCursor::Grab));
    let element = runtime.tree().element_with_key(&Key::from(1_u64)).unwrap();
    let rect = runtime.tree().element_bounds(element).unwrap();
    let outer = Offset::new(10., 10.);
    let empty = rect.origin - Offset::new(3., 3.);
    let inner = rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.);
    for (index, (position, expected)) in [
        (outer, MouseCursor::Grab),
        (empty, MouseCursor::Grab),
        (inner, MouseCursor::NotAllowed),
        (empty, MouseCursor::Grab),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Mouse,
            position,
            phase: if index == 0 {
                PointerPhase::Down
            } else {
                PointerPhase::Move
            },
            buttons: 1,
            button: (index == 0).then_some(1),
            sample: PointerSampleMetadata::default(),
        });
        assert_eq!(runtime.mouse_cursor_for_device(1, 1), Some(expected));
    }
}

#[test]
fn a_drag_to_the_bottom_right_includes_the_final_newline() {
    const TEXT: &str = "Hello world\n";
    let controller = SelectionAreaController::new();
    let label = Widget::from(Text::new(TEXT).style(TextStyle::new().line_height_multiplier(1.4)))
        .with_key(1_u64);
    let mut runtime = mounted(Directionality::new(
        TextDirection::Ltr,
        Widget::selection_area(controller.clone(), Center::new(label).into()),
    ));
    let element = runtime.tree().element_with_key(&Key::from(1_u64)).unwrap();
    let rect = runtime.tree().element_bounds(element).unwrap();
    let bottom_right = rect.origin + Offset::new(rect.size.width, rect.size.height);
    for (phase, position) in [
        (PointerPhase::Down, rect.origin),
        (PointerPhase::Move, bottom_right),
        (PointerPhase::Up, bottom_right),
    ] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    assert_eq!(controller.selection().status, SelectionStatus::Uncollapsed);
    assert_eq!(
        controller.selection().range,
        Some(SelectedContentRange::new(0, 12))
    );
    assert_eq!(controller.selected_text(), TEXT);
}
