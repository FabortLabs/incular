//! The actual touch sequence and same-frame geometry from the pinned
//! SliverPadding scroll-offset correction registration.

use incular_config::{Constraints, EdgeInsets};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata, Rect, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::internal::{Key, PointerDeviceKind};
use incular_widgets::{
    Center, CustomScrollView, Directionality, ScrollController, SizedBox, Sliver, SliverList,
    SliverPadding, Text, TextDirection, Widget,
};

fn list(first_height: f32, controller: Option<ScrollController>) -> Widget {
    let list = SliverList::builder(10, move |index| {
        Widget::from(
            SizedBox::new()
                .height(if index == 0 { first_height } else { 200. })
                .child(Center::new(Text::new(index.to_string()))),
        )
        .with_key(Key::from(index as u64))
    });
    let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverPadding::new(EdgeInsets::ZERO, list))];
    let mut view = CustomScrollView::new(slivers).cache_extent(0.);
    if let Some(controller) = controller {
        view = view.controller(controller);
    }
    Directionality::new(TextDirection::Ltr, view).into()
}

fn row(runtime: &Runtime, index: u64) -> Rect {
    let element = runtime
        .tree()
        .element_with_key(&Key::from(index))
        .expect("retained row");
    runtime.tree().element_bounds(element).unwrap()
}

fn drag_from_row(runtime: &mut Runtime, time: Instant, row: Rect, pointer: u64, dy: f32) {
    let start = row.origin + Offset::new(row.size.width / 2., row.size.height / 2.);
    let end = start + Offset::new(0., dy);
    for (phase, position) in [
        (PointerPhase::Down, start),
        (
            PointerPhase::Move,
            start + Offset::new(0., dy.signum() * 20.),
        ),
        (PointerPhase::Move, end),
        (PointerPhase::Up, end),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
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
                position,
            },
            time,
        );
    }
}

#[test]
fn earlier_row_growth_corrects_the_offset_within_the_first_frame_after_dragging_back() {
    let mut runtime = Runtime::new(list(200., None)).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    let now = Instant::now();
    runtime.run_frame_at(view, now).unwrap();

    let middle = row(&runtime, 2);
    drag_from_row(&mut runtime, now, middle, 1, -300.);
    runtime.run_frame_at(view, now).unwrap();
    assert_eq!(
        row(&runtime, 2),
        Rect::from_origin_size(Offset::new(0., 100.), Size::new(800., 200.)),
    );

    runtime
        .schedule_update(runtime.tree().root().unwrap(), list(400., None))
        .unwrap();
    runtime.run_frame_at(view, now).unwrap();
    let middle = row(&runtime, 2);
    assert_eq!(
        middle.origin.y, 100.,
        "building again must keep the implicit scroll position"
    );
    drag_from_row(&mut runtime, now, middle, 2, 300.);
    runtime.run_frame_at(view, now).unwrap();
    assert_eq!(
        row(&runtime, 0),
        Rect::from_origin_size(Offset::new(0., -200.), Size::new(800., 400.)),
    );
}

#[test]
fn explicit_replacement_controller_keeps_its_own_initial_position() {
    let first = ScrollController::new();
    let mut runtime = Runtime::new(list(200., Some(first.clone()))).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    let now = Instant::now();
    runtime.run_frame_at(view, now).unwrap();
    assert!(first.jump_to(300.));
    runtime.run_frame_at(view, now).unwrap();
    assert_eq!(row(&runtime, 2).origin.y, 100.);

    let replacement = ScrollController::new();
    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            list(400., Some(replacement.clone())),
        )
        .unwrap();
    runtime.run_frame_at(view, now).unwrap();
    assert_eq!(replacement.offset(), 0.);
    assert_eq!(row(&runtime, 0).origin.y, 0.);
}
