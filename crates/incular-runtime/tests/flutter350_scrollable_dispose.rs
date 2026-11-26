//! Mounted scroll teardown, including an overscrolled nested hold.
//! Upstream: widgets/scrollable_dispose_test.dart.

use std::time::Duration;

use incular_config::{Axis, Constraints, TargetPlatform};
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, ListView, RawPointerEvent, ScrollBehavior, ScrollConfiguration,
    ScrollController, SingleChildScrollView, SizedBox, TextDirection,
};

fn frame(runtime: &mut Runtime, time: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), time)
        .unwrap();
}

fn touch(runtime: &mut Runtime, time: Instant, id: u64, phase: PointerPhase, at: Offset) {
    let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
        id,
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

fn fling_y(runtime: &mut Runtime, now: Instant, x: f32, start: f32, step: f32) {
    touch(runtime, now, 1, PointerPhase::Down, Offset::new(x, start));
    for i in 1..=7 {
        touch(
            runtime,
            now + Duration::from_millis(i * 16),
            1,
            PointerPhase::Move,
            Offset::new(x, start + step * i as f32),
        );
    }
    touch(
        runtime,
        now + Duration::from_millis(113),
        1,
        PointerPhase::Up,
        Offset::new(x, start + step * 7.),
    );
}

fn remove_scrollables(runtime: &mut Runtime, time: Instant) {
    let root = runtime.tree().root().unwrap();
    let empty = Directionality::new(TextDirection::Ltr, SizedBox::new());
    runtime.schedule_update(root, empty.into()).unwrap();
    frame(runtime, time);
}

#[test]
fn replacing_a_list_on_the_frame_its_fling_would_end_releases_the_position() {
    let controller = ScrollController::new();
    let list = ListView::builder(250, |_| SizedBox::new().height(20.))
        .item_extent(20.)
        .controller(controller.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert!(controller.has_clients());
    fling_y(
        &mut runtime,
        now + Duration::from_millis(10),
        400.,
        300.,
        -28.,
    );
    frame(&mut runtime, now + Duration::from_millis(123));
    let dragged = controller.offset();
    assert!(dragged > 0.);
    frame(&mut runtime, now + Duration::from_millis(139));
    assert!(
        controller.offset() > dragged,
        "a ballistic frame actually ran"
    );

    remove_scrollables(&mut runtime, now + Duration::from_secs(5 * 60 * 60));
    assert!(!controller.has_clients());
    assert!(!runtime.tree().has_pending_scroll_animations());
}

#[test]
fn replacing_nested_scrollables_while_holding_overscroll_detaches_both_positions() {
    for platform in [TargetPlatform::Ios, TargetPlatform::MacOs] {
        let controller = ScrollController::new();
        let outer_controller = ScrollController::new();
        let inner = ListView::builder(250, |_| SizedBox::new().height(200.))
            .item_extent(200.)
            .controller(controller.clone());
        let outer = SingleChildScrollView::new(SizedBox::new().width(500.).child(inner))
            .scroll_direction(Axis::Horizontal)
            .controller(outer_controller.clone());
        let scene = Center::new(SizedBox::new().width(400.).child(outer));
        let scene =
            ScrollConfiguration::new(scene).behavior(ScrollBehavior::new().platform(platform));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
        let now = Instant::now();
        let at = |ms| now + Duration::from_millis(ms);
        frame(&mut runtime, now);
        assert!(controller.has_clients() && outer_controller.has_clients());
        fling_y(&mut runtime, at(10), 250., 50., 70.);
        frame(&mut runtime, at(123));
        frame(&mut runtime, at(223));
        let previous = controller.offset();
        assert!(previous < 0., "{platform:?}: {previous}");
        frame(&mut runtime, at(323));
        let held = controller.offset();
        assert!(
            held > previous && held < 0.,
            "{platform:?}: {previous} → {held}"
        );

        let contact = Offset::new(250., 50.);
        touch(&mut runtime, at(324), 2, PointerPhase::Down, contact);
        frame(&mut runtime, at(524));
        frame(&mut runtime, at(724));
        assert_eq!(controller.offset(), held, "{platform:?}");
        remove_scrollables(&mut runtime, at(725));
        assert!(!controller.has_clients());
        assert!(!outer_controller.has_clients());
        touch(&mut runtime, at(726), 2, PointerPhase::Up, contact);
        frame(&mut runtime, at(1000));
        assert!(!runtime.tree().has_pending_scroll_animations());
    }
}
