//! Mounted scale and drag recognizers competing for real pointer routes.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, RawPointerEvent, SizedBox};

#[test]
fn an_accepted_scale_claims_new_contacts_before_a_horizontal_drag() {
    use PointerPhase::{Down, Move, Up};

    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |event| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(event)
    };
    let start = record("scale start");
    let update = record("scale update");
    let end = record("scale end");
    // Give the scale first position in the hit route, just as the pinned
    // test adds the scale recognizer before the horizontal drag recognizer.
    let inner = GestureDetector::new(SizedBox::expand())
        .drag_start_behavior(DragStartBehavior::Down)
        .behavior(HitTestBehavior::Opaque)
        .on_scale_start(move |_| start())
        .on_scale_update(move |_| update())
        .on_scale_end(move |_| end());
    let drag_start = record("drag start");
    let drag_end = record("drag end");
    let outer = GestureDetector::new(inner)
        .on_horizontal_drag_start(move |_| drag_start())
        .on_horizontal_drag_end(move |_| drag_end());

    let mut runtime = Runtime::new(outer.into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();
    let mut ms = 0;
    let mut route = |pointer, phase, x, y| {
        ms += 1;
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            if phase == Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
        std::mem::take(&mut *trace.borrow_mut())
    };
    assert!(route(1, Down, 10., 10.).is_empty());
    assert_eq!(route(1, Move, 10., 50.), ["scale start", "scale update"]);
    assert_eq!(route(2, Down, 10., 20.), ["scale end"]);
    assert_eq!(route(2, Move, 30., 20.), ["scale start", "scale update"]);
    assert_eq!(route(1, Up, 10., 50.), ["scale end"]);
    assert!(route(2, Up, 30., 20.).is_empty());

    assert!(route(3, Down, 30., 30.).is_empty());
    assert_eq!(route(3, Move, 100., 30.), ["scale start", "scale update"]);
    assert_eq!(route(3, Up, 100., 30.), ["scale end"]);
}

#[test]
fn mounted_scale_keeps_event_timestamps_and_excludes_the_tap_competitor() {
    use PointerPhase::{Down, Move, Up};

    let now = Instant::now();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |phase| {
        let trace = trace.clone();
        move |time: Option<Instant>, count| {
            trace.borrow_mut().push((
                phase,
                time.map(|time| time.duration_since(now).as_millis()),
                count,
            ));
        }
    };
    let start = record("start");
    let update = record("update");
    let end = record("end");
    let tap = record("tap");
    let scale = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .drag_start_behavior(DragStartBehavior::Down)
        .on_scale_start(move |d| start(d.source_time, d.pointer_count))
        .on_scale_update(move |d| update(d.source_time, d.pointer_count))
        .on_scale_end(move |d| end(None, d.pointer_count));
    let parent = GestureDetector::new(scale).on_tap(move || tap(None, 0));
    let mut runtime = Runtime::new(parent.into()).unwrap();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();

    let mut route = |pointer, phase, x, y, ms| {
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            if phase == Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
        std::mem::take(&mut *trace.borrow_mut())
    };
    assert!(route(1, Down, 0., 0., 10).is_empty());
    assert_eq!(
        route(1, Move, 20., 30., 20),
        [("start", Some(10), 1), ("update", Some(20), 1)]
    );
    assert_eq!(route(2, Down, 10., 20., 30), [("end", None, 2)]);
    assert_eq!(
        route(2, Move, 0., 10., 40),
        [("start", Some(40), 2), ("update", Some(40), 2)]
    );
    assert_eq!(route(2, Move, 15., 25., 50), [("update", Some(50), 2)]);
    assert_eq!(route(2, Move, 0., 20., 60), [("update", Some(60), 2)]);
    assert_eq!(route(2, Move, 10., 10., 70), [("update", Some(70), 2)]);
    assert_eq!(route(2, Move, 15., 25., 75), [("update", Some(75), 2)]);

    assert_eq!(route(3, Down, 25., 35., 80), [("end", None, 3)]);
    assert_eq!(
        route(3, Move, 55., 65., 90),
        [("start", Some(90), 3), ("update", Some(90), 3)]
    );
    assert_eq!(route(1, Move, 25., 35., 100), [("update", Some(100), 3)]);
    assert_eq!(route(2, Move, 20., 30., 110), [("update", Some(110), 3)]);
    assert_eq!(route(3, Move, 15., 25., 120), [("update", Some(120), 3)]);
    assert_eq!(route(1, Up, 25., 35., 125), [("end", None, 2)]);
    assert_eq!(
        route(3, Move, 10., 20., 130),
        [("start", Some(130), 2), ("update", Some(130), 2)]
    );
    assert_eq!(route(3, Move, 30., 40., 140), [("update", Some(140), 2)]);
    assert_eq!(route(3, Move, 10., 20., 140), [("update", Some(140), 2)]);
    assert_eq!(route(2, Up, 20., 30., 145), [("end", None, 1)]);
    assert_eq!(
        route(3, Move, 0., 0., 150),
        [("start", Some(150), 1), ("update", Some(150), 1)]
    );
    assert_eq!(route(3, Up, 0., 0., 155), [("end", None, 0)]);
}

#[test]
fn scale_keeps_same_numbered_pointers_on_separate_devices_distinct() {
    use PointerPhase::{Down, Move, Up};

    let now = Instant::now();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |phase| {
        let trace = trace.clone();
        move |count, focal, scale| trace.borrow_mut().push((phase, count, focal, scale))
    };
    let start = record("start");
    let update = record("update");
    let end = record("end");
    let root = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .drag_start_behavior(DragStartBehavior::Down)
        .on_scale_start(move |d| start(d.pointer_count, d.focal_point.x, 1.))
        .on_scale_update(move |d| update(d.pointer_count, d.focal_point.x, d.scale))
        .on_scale_end(move |d| end(d.pointer_count, 0., 0.));
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame_at(Constraints::tight(Size::new(100., 80.)), now)
        .unwrap();
    let mut ms = 0;
    let mut route = |device, phase, x| {
        ms += 1;
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            1,
            device,
            PointerDeviceKind::Touch,
            if phase == Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, 10.),
            phase,
            now + Duration::from_millis(ms),
        ));
        std::mem::take(&mut *trace.borrow_mut())
    };
    assert_eq!(route(4, Down, 10.), [("start", 1, 10., 1.)]);
    assert_eq!(route(9, Down, 30.), [("end", 2, 0., 0.)]);
    assert_eq!(
        route(9, Move, 50.),
        [("start", 2, 30., 1.), ("update", 2, 30., 2.)]
    );
    assert_eq!(route(4, Up, 10.), [("end", 1, 0., 0.)]);
    assert_eq!(
        route(9, Move, 60.),
        [("start", 1, 60., 1.), ("update", 1, 60., 1.)]
    );
    assert_eq!(route(9, Up, 60.), [("end", 0, 0., 0.)]);
}

#[test]
fn an_earlier_drag_can_win_a_new_pointer_before_scale_restarts() {
    use PointerPhase::{Down, Move, Up};

    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |name| {
        let trace = trace.clone();
        move |count| trace.borrow_mut().push((name, count))
    };
    let drag = record("drag");
    let child = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .on_horizontal_drag_start(move |_| drag(0));
    let start = record("start");
    let update = record("update");
    let end = record("end");
    let parent = GestureDetector::new(child)
        .behavior(HitTestBehavior::Opaque)
        .on_scale_start(move |d| start(d.pointer_count))
        .on_scale_update(move |d| update(d.pointer_count))
        .on_scale_end(move |d| end(d.pointer_count));
    let mut runtime = Runtime::new(parent.into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();
    let mut ms = 0;
    let mut route = |pointer, phase, x, y| {
        ms += 1;
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            if phase == Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
        std::mem::take(&mut *trace.borrow_mut())
    };
    assert!(route(1, Down, 10., 10.).is_empty());
    assert_eq!(route(1, Move, 10., 50.), [("start", 1), ("update", 1)]);
    assert_eq!(route(2, Down, 10., 20.), [("end", 2)]);
    assert_eq!(route(2, Move, 30., 20.), [("drag", 0)]);
    assert_eq!(route(1, Move, 10., 90.), [("start", 1), ("update", 1)]);
    assert_eq!(route(1, Up, 10., 90.), [("end", 0)]);
    assert!(route(2, Up, 30., 20.).is_empty());
}

#[test]
fn mounted_rotation_keeps_the_sign_and_rebases_when_fingers_change() {
    use PointerPhase::{Down, Move, Up};

    let starts = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let taps = Rc::new(std::cell::Cell::new(0));
    let scale = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .drag_start_behavior(DragStartBehavior::Down)
        .on_scale_start({
            let starts = starts.clone();
            move |d| starts.borrow_mut().push((d.focal_point, d.pointer_count))
        })
        .on_scale_update({
            let updates = updates.clone();
            move |d| {
                updates.borrow_mut().push((
                    d.focal_point,
                    d.pointer_count,
                    d.focal_point_delta,
                    d.rotation,
                ))
            }
        })
        .on_scale_end({
            let ends = ends.clone();
            move |d| ends.borrow_mut().push(d.pointer_count)
        });
    let scene = GestureDetector::new(scale).on_tap({
        let taps = taps.clone();
        move || taps.set(taps.get() + 1)
    });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();
    let mut ms = 0;
    let mut route = |pointer, phase, x, y| {
        ms += 1;
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            if phase == Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
        std::mem::take(&mut *updates.borrow_mut())
    };
    let near = |actual: f32, expected: f32| (actual - expected).abs() < 0.0001;
    let check = |details: Vec<(Offset, usize, Offset, f32)>, (x, y), (dx, dy), rotation, count| {
        let [details] = details.as_slice() else {
            panic!("one rotation update: {details:?}");
        };
        let (focal, contacts, delta, angle) = *details;
        assert_eq!(focal, Offset::new(x, y));
        assert_eq!(contacts, count);
        assert!(near(delta.x, dx) && near(delta.y, dy));
        assert!(near(angle, rotation), "{details:?}");
    };

    assert!(route(1, Down, 0., 0.).is_empty());
    assert!(starts.borrow().is_empty());
    check(route(1, Move, 20., 30.), (20., 30.), (20., 30.), 0., 1);
    assert!(route(2, Down, 30., 40.).is_empty());
    check(route(2, Move, 40., 50.), (30., 40.), (5., 5.), 0., 2);
    check(
        route(2, Move, 0., 10.),
        (10., 20.),
        (-20., -20.),
        std::f32::consts::PI,
        2,
    );
    assert!(route(3, Down, 25., 35.).is_empty());
    check(route(3, Move, 55., 65.), (25., 35.), (10., 10.), 0., 3);
    let _ = route(1, Move, 25., 35.);
    let _ = route(2, Move, 20., 30.);
    check(
        route(3, Move, 15., 25.),
        (20., 30.),
        (-40. / 3., -40. / 3.),
        0.,
        3,
    );
    assert!(route(1, Up, 25., 35.).is_empty());
    check(route(3, Move, 10., 20.), (15., 25.), (-2.5, -2.5), 0., 2);
    check(
        route(3, Move, 30., 40.),
        (25., 35.),
        (10., 10.),
        -std::f32::consts::PI,
        2,
    );
    check(route(3, Move, 10., 20.), (15., 25.), (-10., -10.), 0., 2);
    assert!(route(2, Up, 20., 30.).is_empty());
    assert!(route(3, Up, 10., 20.).is_empty());
    assert_eq!(
        *starts.borrow(),
        [
            (Offset::new(20., 30.), 1),
            (Offset::new(30., 40.), 2),
            (Offset::new(25., 35.), 3),
            (Offset::new(15., 25.), 2),
        ]
    );
    assert_eq!(*ends.borrow(), [2, 3, 2, 1]);
    assert_eq!(taps.get(), 0);
}
