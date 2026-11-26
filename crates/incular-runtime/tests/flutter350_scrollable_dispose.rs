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
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        at,
        phase,
        time,
    ));
}

fn fling_y(runtime: &mut Runtime, now: Instant, x: f32, start: f32, step: f32, samples: u64) {
    touch(runtime, now, 1, PointerPhase::Down, Offset::new(x, start));
    for i in 1..=samples {
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
        now + Duration::from_millis(16 * samples + 1),
        1,
        PointerPhase::Up,
        Offset::new(x, start + step * samples as f32),
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
        7,
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
        fling_y(&mut runtime, at(10), 250., 50., 70., 7);
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
        assert_eq!(controller.offset(), held);
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

#[test]
fn ending_a_hold_without_dragging_resumes_springback() {
    use incular_widgets::ScrollPhysics;

    for ending in [PointerPhase::Up, PointerPhase::Cancel] {
        let controller = ScrollController::new();
        let list = ListView::builder(250, |_| SizedBox::new().height(200.))
            .item_extent(200.)
            .physics(ScrollPhysics::BOUNCING)
            .controller(controller.clone());
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
        let now = Instant::now();
        let at = |ms| now + Duration::from_millis(ms);
        frame(&mut runtime, now);

        fling_y(&mut runtime, at(10), 400., 50., 70., 7);
        frame(&mut runtime, at(123));
        frame(&mut runtime, at(223));
        let previous = controller.offset();
        frame(&mut runtime, at(323));
        let held = controller.offset();
        assert!(previous < held && held < 0., "{previous} → {held}");

        let contact = Offset::new(400., 50.);
        touch(&mut runtime, at(324), 2, PointerPhase::Down, contact);
        frame(&mut runtime, at(524));
        frame(&mut runtime, at(724));
        assert_eq!(controller.offset(), held);

        touch(&mut runtime, at(725), 2, ending, contact);
        assert!(
            controller.is_scrolling(),
            "{ending:?} should resume the spring"
        );
        frame(&mut runtime, at(726));
        frame(&mut runtime, at(826));
        assert!(
            controller.offset() > held,
            "{ending:?} must approach the edge"
        );
        frame(&mut runtime, at(3000));
        assert!(controller.offset().abs() < 0.001);
        assert!(!controller.is_scrolling());
    }
}

#[test]
fn settled_touch_flings_can_restart_and_advance_the_same_position() {
    let controller = ScrollController::new();
    let list = ListView::new([SizedBox::new().height(100_000.)]).controller(controller.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert_eq!(controller.offset(), 0.);
    assert!(controller.max_offset() > 10_000.);

    let mut previous = 0.;
    for start in [10, 6010] {
        // 200 logical pixels at 500 px/s, as in scroll_interaction_test.dart.
        fling_y(
            &mut runtime,
            now + Duration::from_millis(start),
            200.,
            300.,
            -8.,
            25,
        );
        frame(&mut runtime, now + Duration::from_millis(start + 401));
        let offset_at_release = controller.offset();
        frame(&mut runtime, now + Duration::from_millis(start + 5401));
        let settled = controller.offset();
        assert!(
            offset_at_release > previous,
            "the new contact advances the position"
        );
        assert!(
            settled > offset_at_release,
            "motion continues after release"
        );
        assert!(!controller.is_scrolling(), "the position returns to idle");
        previous = settled;
    }
}

fn reference_fling(runtime: &mut Runtime, start: Instant, distance: f32, speed: f64) -> Instant {
    let center = Offset::new(400., 300.);
    touch(runtime, start, 1, PointerPhase::Down, center);
    let sample_us = 1_000_000. * f64::from(distance.abs()) / (50. * speed);
    let mut frame_us = 0.;
    for index in 0..=50 {
        let us = f64::from(index) * sample_us;
        touch(
            runtime,
            start + Duration::from_micros(us.round() as u64),
            1,
            PointerPhase::Move,
            center + Offset::new(0., distance * index as f32 / 50.),
        );
        if us + sample_us - frame_us > 16_000. {
            frame_us = us + sample_us;
            frame(runtime, start + Duration::from_micros(frame_us as u64));
        }
    }
    let end = start + Duration::from_micros((51. * sample_us).round() as u64);
    touch(
        runtime,
        end,
        1,
        PointerPhase::Up,
        center + Offset::new(0., distance),
    );
    end
}

fn tap_center(runtime: &mut Runtime, now: Instant) {
    let center = Offset::new(400., 300.);
    touch(runtime, now, 2, PointerPhase::Down, center);
    touch(
        runtime,
        now + Duration::from_millis(1),
        2,
        PointerPhase::Up,
        center,
    );
    frame(runtime, now + Duration::from_millis(51));
}

fn taps_around_a_fling(stop_early: bool) {
    use incular_core::{Color, DragStartBehavior};
    use incular_widgets::{GestureDetector, Widget};
    use std::{cell::RefCell, rc::Rc};

    let tapped = Rc::new(RefCell::new(Vec::new()));
    let controller = ScrollController::new();
    let rows = (0..250).map(|index| {
        let tapped = tapped.clone();
        let row = Widget::box_(Size::new(800., 14.), Color::WHITE);
        GestureDetector::new(row).on_tap(move || tapped.borrow_mut().push(index))
    });
    let list = ListView::new(rows)
        .item_extent(14.)
        .controller(controller.clone())
        .drag_start_behavior(DragStartBehavior::Down);
    let configured = ScrollConfiguration::new(list)
        .behavior(ScrollBehavior::new().platform(TargetPlatform::Android));
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, configured).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert!(tapped.borrow().is_empty());
    tap_center(&mut runtime, now + Duration::from_millis(1));
    assert_eq!(*tapped.borrow(), [21]);

    let release = reference_fling(&mut runtime, now + Duration::from_millis(60), -200., 1000.);
    let first_fling_frame = release + Duration::from_millis(50);
    frame(&mut runtime, first_fling_frame);
    assert_eq!(*tapped.borrow(), [21]);
    assert!(controller.is_scrolling());

    if stop_early {
        let stopping_at = controller.offset();
        tap_center(&mut runtime, first_fling_frame + Duration::from_millis(1));
        assert_eq!(
            *tapped.borrow(),
            [21],
            "the stop tap must not reach the row"
        );
        assert_eq!(controller.offset(), stopping_at);
        assert!(!controller.is_scrolling());
        tap_center(&mut runtime, first_fling_frame + Duration::from_millis(60));
        assert_eq!(*tapped.borrow(), [21, 35]);
    } else {
        let settled_frame = first_fling_frame + Duration::from_secs(50);
        frame(&mut runtime, settled_frame);
        assert!(!controller.is_scrolling());
        tap_center(&mut runtime, settled_frame + Duration::from_millis(1));
        assert_eq!(*tapped.borrow(), [21, 49]);
    }
}

#[test]
fn the_first_tap_stops_a_fling_and_only_the_next_tap_hits_the_row() {
    taps_around_a_fling(true);
}

#[test]
fn a_settled_fling_does_not_consume_the_next_tap() {
    taps_around_a_fling(false);
}

#[test]
fn stopping_a_driven_scroll_does_not_dispatch_a_new_contact_to_rows() {
    use incular_animation::Curve;
    use incular_core::Color;
    use incular_widgets::{GestureDetector, Listener, Widget};
    use std::{cell::Cell, rc::Rc};

    let presses = Rc::new(Cell::new(0));
    let taps = Rc::new(Cell::new(0));
    let controller = ScrollController::new();
    let rows = (0..100).map(|_| {
        let presses = presses.clone();
        let taps = taps.clone();
        let row = Widget::box_(Size::new(800., 40.), Color::WHITE);
        Listener::new(GestureDetector::new(row).on_tap(move || taps.set(taps.get() + 1)))
            .on_pointer_down(move |_| presses.set(presses.get() + 1))
    });
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            ListView::new(rows)
                .item_extent(40.)
                .controller(controller.clone()),
        )
        .into(),
    )
    .unwrap();
    let now = Instant::now();
    let at = |ms| now + Duration::from_millis(ms);
    frame(&mut runtime, now);
    tap_center(&mut runtime, at(1));
    assert_eq!((presses.get(), taps.get()), (1, 1));

    assert!(controller.animate_to(400., Duration::from_secs(1), Curve::Linear));
    frame(&mut runtime, at(60));
    frame(&mut runtime, at(160));
    let stopped = controller.offset();
    assert!(stopped > 0. && stopped < 400.);
    tap_center(&mut runtime, at(161));
    assert_eq!((presses.get(), taps.get()), (1, 1));
    assert_eq!(controller.offset(), stopped);
    assert!(!controller.is_scrolling());

    tap_center(&mut runtime, at(220));
    assert_eq!((presses.get(), taps.get()), (2, 2));
}
