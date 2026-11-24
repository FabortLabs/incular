//! Runtime behavior from the pinned SingleChildScrollView originals.
use incular_config::{Axis, Constraints};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    Center, Directionality, LayoutBuilder, Placeholder, SingleChildScrollView, SizedBox,
    TextDirection, Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
fn scroll(axis: Axis, extent: f32, controller: Option<ScrollController>, child_key: Key) -> Widget {
    let size = match axis {
        Axis::Horizontal => Size::new(extent, VIEW.height),
        Axis::Vertical => Size::new(VIEW.width, extent),
    };
    let child = Widget::box_(size, Color::rgba(0, 255, 0, 255)).with_key(child_key);
    let mut view = SingleChildScrollView::new(child).scroll_direction(axis);
    if let Some(controller) = controller {
        view = view.controller(controller);
    }
    Directionality::new(TextDirection::Ltr, view).into()
}

fn child_origin(runtime: &Runtime, key: &Key) -> Offset {
    let child = runtime.tree().element_with_key(key).unwrap();
    runtime.tree().element_bounds(child).unwrap().origin
}

#[test]
fn default_scroller_clips_only_when_children_overflow() {
    let key = Key::Value(1);
    let mut actual = Vec::new();
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let fit = axis.main_extent(VIEW);
        let mut runtime = Runtime::new(scroll(axis, fit, None, key.clone())).unwrap();
        for size in [fit, fit + 0.1] {
            let root = runtime.tree().root().unwrap();
            runtime
                .schedule_update(root, scroll(axis, size, None, key.clone()))
                .unwrap();
            let (list, _) = runtime.run_frame(Constraints::tight(VIEW)).unwrap();
            actual.push(
                list.commands()
                    .iter()
                    .any(|command| matches!(command, PaintCommand::PushClip { .. })),
            );
        }
    }
    assert_eq!(actual, [false, true, false, true]);
}

#[test]
fn diagonal_touch_drag_moves_the_vertical_child_200_pixels_only() {
    let key = Key::Value(2);
    let mut runtime = Runtime::new(scroll(Axis::Vertical, 2000., None, key.clone())).unwrap();
    let constraints = Constraints::tight(VIEW);
    let now = Instant::now();
    runtime.run_frame_at(constraints, now).unwrap();
    assert_eq!(child_origin(&runtime, &key), Offset::ZERO);
    for (phase, x, y) in [
        (PointerPhase::Down, 400., 300.),
        (PointerPhase::Move, 380., 280.), // flutter_test.dragFrom's 20-pixel slop waypoint
        (PointerPhase::Move, 200., 100.),
        (PointerPhase::Up, 200., 100.),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(x, y),
            },
            now,
        );
    }
    runtime.run_frame_at(constraints, now).unwrap();
    assert_eq!(child_origin(&runtime, &key), Offset::new(0., -200.));
}

#[test]
fn replacement_controller_owns_the_retained_viewport() {
    let key = Key::Value(3);
    let first = ScrollController::new();
    let second = ScrollController::new().with_initial_scroll_offset(120.);
    let mut runtime = Runtime::new(scroll(
        Axis::Vertical,
        2000.,
        Some(first.clone()),
        key.clone(),
    ))
    .unwrap();
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    assert_eq!(first.max_offset(), 1400.);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            scroll(Axis::Vertical, 2000., Some(second.clone()), key.clone()),
        )
        .unwrap();
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    assert_eq!(second.max_offset(), 1400.);
    first.jump_to(20.);
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    assert_eq!(child_origin(&runtime, &key), Offset::new(0., -120.));
}

#[test]
fn controller_can_be_attached_during_a_dirty_layout_builder() {
    let key = Key::Value(4);
    let controller = ScrollController::new();
    let scene = |width, selected: Option<ScrollController>| {
        let key = key.clone();
        Center::new(
            SizedBox::new()
                .width(width)
                .child(LayoutBuilder::new(move |_, _| {
                    scroll(Axis::Vertical, 2000., selected.clone(), key.clone())
                })),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(750., None)).unwrap();
    let constraints = Constraints::tight(VIEW);
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(700., Some(controller.clone())))
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    assert_eq!(controller.max_offset(), 1400.);
}

#[test]
fn centered_scroll_view_can_lay_out_at_zero_area() {
    let key = Key::Value(5);
    let scene = Center::new(
        Widget::from(SingleChildScrollView::new(Placeholder::new())).with_key(key.clone()),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    let target = runtime.tree().element_with_key(&key).unwrap();
    assert_eq!(
        runtime.tree().element_bounds(target).unwrap().size,
        Size::ZERO
    );
}
