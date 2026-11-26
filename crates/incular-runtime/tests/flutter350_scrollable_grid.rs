//! Visible grid children from the pinned widgets/scrollable_grid_test.dart.

use std::time::Duration;

use incular_config::{Constraints, EdgeInsets};
use incular_core::{
    Color, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Rect, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, GridView, RawPointerEvent, ScrollController, SizedBox, TextDirection,
    Widget,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn frame(runtime: &mut Runtime, time: Instant) {
    runtime
        .run_frame_at(Constraints::tight(VIEW), time)
        .unwrap();
}

fn tile(index: u64) -> Widget {
    Widget::box_(Size::new(100., 200.), Color::WHITE).with_key(index)
}

fn bounds(runtime: &Runtime, index: u64) -> Option<Rect> {
    let tree = runtime.tree();
    tree.element_bounds(tree.element_with_key(&index.into())?)
}

fn onscreen(runtime: &Runtime, count: u64) -> Vec<u64> {
    let viewport = Rect::from_origin_size(Offset::ZERO, VIEW);
    (0..count)
        .filter(|&index| bounds(runtime, index).is_some_and(|rect| rect.intersects(viewport)))
        .collect()
}

fn center(rect: Rect) -> Offset {
    rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.)
}

fn touch(runtime: &mut Runtime, pointer: u64, phase: PointerPhase, at: Offset, time: Instant) {
    let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
        pointer,
        0,
        PointerDeviceKind::Touch,
        if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        at,
        phase,
        time,
    ));
}

fn drag_grid(runtime: &mut Runtime, pointer: u64, start: Offset, dy: f32, time: Instant) {
    for (phase, delta, elapsed) in [
        (PointerPhase::Down, 0., 0),
        (PointerPhase::Move, dy.signum() * 19., 15),
        (PointerPhase::Move, dy, 30),
        (PointerPhase::Up, dy, 150),
    ] {
        touch(
            runtime,
            pointer,
            phase,
            start + Offset::new(0., delta),
            time + Duration::from_millis(elapsed),
        );
    }
    frame(runtime, time + Duration::from_millis(200));
}

#[test]
fn an_empty_centered_one_lane_grid_mounts_and_has_no_scroll_extent() {
    let controller = ScrollController::new();
    let empty = GridView::count(1, Vec::<Widget>::new()).controller(controller.clone());
    let root = Directionality::new(TextDirection::Ltr, Center::new(empty));
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime, Instant::now());
    assert!(controller.has_clients());
    assert_eq!(controller.viewport_extent(), 600.);
    assert_eq!(controller.max_offset(), 0.);
}

#[test]
fn padded_grid_shows_the_expected_tiles_after_both_touch_drags() {
    let controller = ScrollController::new();
    let grid = GridView::count(1, (0..10).map(tile))
        .controller(controller.clone())
        .padding(EdgeInsets::from_ltrb(0., 100., 0., 0.));
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().width(300.).height(800.).child(grid)),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    let first = bounds(&runtime, 0).unwrap();
    let second = bounds(&runtime, 1).unwrap();
    assert_eq!(
        first,
        Rect::from_origin_size(Offset::new(250., 100.), Size::new(300., 300.))
    );
    assert_eq!(second.origin.y, 400.);
    assert_eq!(onscreen(&runtime, 10), [0, 1]);

    drag_grid(
        &mut runtime,
        1,
        center(second),
        -500.,
        now + Duration::from_millis(10),
    );
    assert!(controller.offset() > 400.);
    assert_eq!(onscreen(&runtime, 10), [1, 2, 3]);

    let second = bounds(&runtime, 1).unwrap();
    drag_grid(
        &mut runtime,
        2,
        center(second),
        150.,
        now + Duration::from_millis(250),
    );
    assert!(controller.offset() < 400.);
    assert_eq!(onscreen(&runtime, 10), [0, 1, 2]);
}

#[test]
fn fixed_count_grid_recomputes_its_lane_width_and_shows_an_appended_tile() {
    let controller = ScrollController::new();
    let root = |n| -> Widget {
        Directionality::new(
            TextDirection::Ltr,
            GridView::count(n, (0..n as u64).map(tile)).controller(controller.clone()),
        )
        .into()
    };
    let mut runtime = Runtime::new(root(3)).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert_eq!(onscreen(&runtime, 4), [0, 1, 2]);
    assert!((bounds(&runtime, 0).unwrap().size.width - 800. / 3.).abs() < 0.001);

    let replacement = runtime.tree().root().unwrap();
    runtime.schedule_update(replacement, root(4)).unwrap();
    frame(&mut runtime, now + Duration::from_millis(1));
    drag_grid(
        &mut runtime,
        3,
        Offset::new(0., 300.),
        -200.,
        now + Duration::from_millis(3),
    );
    assert_eq!(onscreen(&runtime, 4), [0, 1, 2, 3]);
    assert_eq!(
        bounds(&runtime, 3),
        Some(Rect::from_origin_size(
            Offset::new(600., 0.),
            Size::new(200., 200.)
        ))
    );
    assert_eq!(controller.offset(), 0.);
}
