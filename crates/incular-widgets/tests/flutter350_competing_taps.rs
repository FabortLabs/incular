use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size, time::Instant};
use incular_gestures::PointerEvent;
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

type TapLog = Rc<RefCell<Vec<(&'static str, &'static str)>>>;

fn record(owner: &'static str, log: &TapLog, child: impl Into<Widget>) -> GestureDetector {
    let emit = |phase| {
        let log = Rc::clone(log);
        move || log.borrow_mut().push((owner, phase))
    };
    let down = emit("down");
    let up = emit("up");
    GestureDetector::new(child)
        .on_tap_down(move |_| down())
        .on_tap_up(move |_| up())
        .on_tap(emit("tap"))
        .on_tap_cancel(emit("cancel"))
}

fn mounted(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree
}

fn dispatch(
    tree: &mut WidgetTree,
    pointer: u64,
    phase: PointerPhase,
    position: Offset,
    time: Instant,
) {
    let _ = tree.dispatch_pointer_gesture_in_window(
        0,
        1,
        PointerDeviceKind::Touch,
        PointerEvent {
            pointer,
            position,
            phase,
            time,
        },
    );
}

#[test]
fn two_competing_tap_recognizers_deliver_each_stream_only_once() {
    let log = TapLog::default();
    let leaf = record("A", &log, Widget::box_(Size::new(100., 100.), Color::WHITE));
    let mut tree = mounted(record("B", &log, leaf));
    let start = Instant::now();

    for (pointer, down, up) in [
        (1, Offset::new(10., 10.), Offset::new(11., 9.)),
        (2, Offset::new(30., 30.), Offset::new(31., 29.)),
    ] {
        let now = start + Duration::from_millis(pointer * 20);
        dispatch(&mut tree, pointer, PointerPhase::Down, down, now);
        assert!(log.borrow().is_empty());
        dispatch(
            &mut tree,
            pointer,
            PointerPhase::Up,
            up,
            now + Duration::from_millis(1),
        );
        assert_eq!(log.take(), [("A", "down"), ("A", "up"), ("A", "tap")]);
        let _ = tree.pump_gesture_timers(now + Duration::from_millis(10));
        assert!(log.borrow().is_empty());
    }
}

#[test]
fn a_late_pointer_cancel_cancels_once_after_the_tap_down_deadline() {
    let log = TapLog::default();
    let tap = record(
        "tap",
        &log,
        Widget::box_(Size::new(100., 100.), Color::WHITE),
    );
    let drag = GestureDetector::new(tap).on_horizontal_drag_start(|_| {});
    let mut tree = mounted(drag);
    let now = Instant::now();
    let position = Offset::new(10., 10.);
    dispatch(&mut tree, 5, PointerPhase::Down, position, now);
    assert!(log.borrow().is_empty());
    let late = now + Duration::from_secs(1);
    let _ = tree.pump_gesture_timers(late);
    assert_eq!(log.take(), [("tap", "down")]);
    dispatch(&mut tree, 5, PointerPhase::Cancel, position, late);
    assert_eq!(log.take(), [("tap", "cancel")]);
    let _ = tree.pump_gesture_timers(late + Duration::from_secs(1));
    assert!(log.borrow().is_empty());
}

fn pressed_button(
    tree: &mut WidgetTree,
    pointer: u64,
    buttons: u32,
    phase: PointerPhase,
    position: Offset,
    time: Instant,
) {
    let hit = tree.pointer_hit_test(position);
    let _ = tree.dispatch_pointer_gesture_with_hit_buttons(
        0,
        1,
        PointerDeviceKind::Touch,
        buttons,
        PointerEvent {
            pointer,
            position,
            phase,
            time,
        },
        &hit,
    );
}

#[test]
fn the_primary_pan_competes_only_with_a_tap_for_the_same_button() {
    let now = Instant::now();
    let position = Offset::new(10., 10.);
    let point = Widget::box_(Size::new(100., 100.), Color::WHITE);
    for (button, phase_of_tap) in [(1, PointerPhase::Up), (2, PointerPhase::Down)] {
        let log = TapLog::default();
        let pan = GestureDetector::new(point.clone()).on_pan_start({
            let log = Rc::clone(&log);
            move |_| log.borrow_mut().push(("pan", "start"))
        });
        let tap = GestureDetector::new(pan);
        let tap = if button == 1 {
            tap.on_tap_down({
                let log = Rc::clone(&log);
                move |_| log.borrow_mut().push(("primary", "down"))
            })
        } else {
            tap.on_secondary_tap_down({
                let log = Rc::clone(&log);
                move |_| log.borrow_mut().push(("secondary", "down"))
            })
        };
        let mut tree = mounted(tap);
        pressed_button(&mut tree, 1, button, PointerPhase::Down, position, now);
        if phase_of_tap == PointerPhase::Down {
            assert_eq!(log.take(), [("secondary", "down")]);
        } else {
            assert!(log.borrow().is_empty());
        }
        pressed_button(
            &mut tree,
            1,
            0,
            PointerPhase::Up,
            position,
            now + Duration::from_millis(1),
        );
        if phase_of_tap == PointerPhase::Up {
            assert_eq!(log.take(), [("primary", "down")]);
        } else {
            assert!(log.borrow().is_empty());
        }
    }
}

#[test]
fn a_secondary_drag_never_invokes_primary_recognizers() {
    let log = TapLog::default();
    let record = |phase, log: &TapLog| {
        let log = Rc::clone(log);
        move || log.borrow_mut().push(("primary", phase))
    };
    let child = Widget::box_(Size::new(100., 100.), Color::WHITE);
    let press = record("down", &log);
    let start = record("start", &log);
    let update = record("update", &log);
    let end = record("end", &log);
    let pan = GestureDetector::new(child)
        .on_pan_down(move |_| press())
        .on_pan_start(move |_| start())
        .on_pan_update(move |_| update())
        .on_pan_end(move |_| end())
        .on_pan_cancel(record("cancel", &log));
    let target = GestureDetector::new(pan).on_tap(record("tap", &log));
    let now = Instant::now();
    for _ in 0..2 {
        let mut tree = mounted(target.clone());
        for (offset, phase, buttons, position) in [
            (0, PointerPhase::Down, 2, Offset::new(10., 10.)),
            (1, PointerPhase::Move, 2, Offset::new(20., 30.)),
            (2, PointerPhase::Move, 2, Offset::new(20., 25.)),
            (3, PointerPhase::Up, 0, Offset::new(20., 25.)),
        ] {
            pressed_button(
                &mut tree,
                5,
                buttons,
                phase,
                position,
                now + Duration::from_millis(offset),
            );
        }
        assert!(log.borrow().is_empty());
    }
}

fn horizontal_drag_with_tap(log: &TapLog) -> WidgetTree {
    let emit = |owner, phase| {
        let log = Rc::clone(&log);
        move || log.borrow_mut().push((owner, phase))
    };
    let h_down = emit("H", "down");
    let h_start = emit("H", "start");
    let h_update = emit("H", "update");
    let h_end = emit("H", "end");
    let t_down = emit("T", "down");
    let t_up = emit("T", "up");
    let child = Widget::box_(Size::new(100., 100.), Color::WHITE);
    let drag = GestureDetector::new(child)
        .on_horizontal_drag_down(move |_| h_down())
        .on_horizontal_drag_start(move |_| h_start())
        .on_horizontal_drag_update(move |_| h_update())
        .on_horizontal_drag_end(move |_| h_end())
        .on_horizontal_drag_cancel(emit("H", "cancel"));
    let tap = GestureDetector::new(drag)
        .on_tap_down(move |_| t_down())
        .on_tap_up(move |_| t_up())
        .on_tap_cancel(|| {});
    mounted(tap)
}

#[test]
fn a_horizontal_drag_keeps_one_lifecycle_across_two_contacts() {
    for (first_release, accepts) in [(4, None), (5, None), (5, Some(4)), (4, Some(5))] {
        let log = TapLog::default();
        let mut tree = horizontal_drag_with_tap(&log);
        let now = Instant::now();
        let position = |pointer| Offset::new(if pointer == 4 { 10. } else { 11. }, 10.);

        dispatch(&mut tree, 4, PointerPhase::Down, position(4), now);
        assert_eq!(log.take(), [("H", "down")]);
        dispatch(
            &mut tree,
            5,
            PointerPhase::Down,
            position(5),
            now + Duration::from_millis(1),
        );
        assert!(
            log.borrow().is_empty(),
            "the second contact must not restart the drag: {:?}",
            *log.borrow()
        );
        let mut tick = 2;
        if accepts == Some(4) {
            dispatch(
                &mut tree,
                4,
                PointerPhase::Move,
                Offset::new(100., 100.),
                now + Duration::from_millis(tick),
            );
            tick += 1;
            assert_eq!(log.take(), [("H", "start")]);
        }

        dispatch(
            &mut tree,
            first_release,
            PointerPhase::Up,
            position(first_release),
            now + Duration::from_millis(tick),
        );
        tick += 1;
        if first_release == 4 {
            assert_eq!(log.take(), [("T", "down"), ("T", "up")]);
        } else {
            assert!(log.borrow().is_empty());
        }
        let last = 9 - first_release;
        if accepts == Some(5) {
            dispatch(
                &mut tree,
                5,
                PointerPhase::Move,
                Offset::new(100., 100.),
                now + Duration::from_millis(tick),
            );
            tick += 1;
            assert_eq!(log.take(), [("H", "start")]);
        }
        dispatch(
            &mut tree,
            last,
            PointerPhase::Up,
            if accepts == Some(last) {
                Offset::new(100., 100.)
            } else {
                position(last)
            },
            now + Duration::from_millis(tick),
        );
        if accepts.is_some() {
            assert_eq!(log.take(), [("H", "end")]);
        } else if first_release == 4 {
            assert_eq!(log.take(), [("H", "cancel")]);
        } else {
            assert_eq!(log.take(), [("H", "cancel"), ("T", "down"), ("T", "up")]);
        }
    }
}

#[test]
fn a_canceled_button_chord_does_not_leave_the_other_contact_waiting() {
    let log = TapLog::default();
    let mut tree = horizontal_drag_with_tap(&log);
    let now = Instant::now();
    let time = |ms| now + Duration::from_millis(ms);
    pressed_button(
        &mut tree,
        4,
        1,
        PointerPhase::Down,
        Offset::new(10., 10.),
        time(0),
    );
    pressed_button(
        &mut tree,
        5,
        1,
        PointerPhase::Down,
        Offset::new(11., 10.),
        time(1),
    );
    assert_eq!(log.take(), [("H", "down")]);

    pressed_button(
        &mut tree,
        4,
        2,
        PointerPhase::Move,
        Offset::new(9.9, 9.9),
        time(2),
    );
    assert_eq!(log.take(), [("H", "start")]);
    pressed_button(
        &mut tree,
        5,
        1,
        PointerPhase::Move,
        Offset::new(100., 100.),
        time(3),
    );
    assert_eq!(log.take(), [("H", "update")]);
    pressed_button(
        &mut tree,
        5,
        0,
        PointerPhase::Up,
        Offset::new(100., 100.),
        time(4),
    );
    assert_eq!(log.take(), [("H", "end")]);
}

#[test]
fn an_accepted_vertical_drag_ends_only_after_the_last_tracked_contact_cancels() {
    use incular_widgets::{Positioned, Stack};

    let log = TapLog::default();
    let emit = |owner, phase| {
        let log = Rc::clone(&log);
        move || log.borrow_mut().push((owner, phase))
    };
    let tap_down = emit("T", "down");
    let tap_up = emit("T", "up");
    let shortcut = GestureDetector::new(Widget::box_(Size::new(12.5, 12.5), Color::BLACK))
        .on_tap_down(move |_| tap_down())
        .on_tap_up(move |_| tap_up())
        .on_tap_cancel(|| {});
    // The first three contacts compete with a tap. The fourth is outside
    // the nested tap target, but inside the same mounted vertical drag.
    let surface = Stack::new([
        Widget::box_(Size::new(100., 100.), Color::WHITE),
        Positioned::new(shortcut)
            .left(0.)
            .top(0.)
            .width(12.5)
            .height(12.5)
            .into(),
    ]);
    let down = emit("D", "down");
    let start = emit("D", "start");
    let update = emit("D", "update");
    let end = emit("D", "end");
    let drag = GestureDetector::new(surface)
        .on_vertical_drag_down(move |_| down())
        .on_vertical_drag_start(move |_| start())
        .on_vertical_drag_update(move |_| update())
        .on_vertical_drag_end(move |_| end())
        .on_vertical_drag_cancel(emit("D", "cancel"));
    let mut tree = mounted(drag);
    let now = Instant::now();
    let events: &[(u64, PointerPhase, &[(&str, &str)])] = &[
        (1, PointerPhase::Down, &[("D", "down")]),
        (2, PointerPhase::Down, &[]),
        (3, PointerPhase::Down, &[]),
        (4, PointerPhase::Down, &[("D", "start")]),
        (2, PointerPhase::Up, &[]),
        (4, PointerPhase::Cancel, &[]),
        (3, PointerPhase::Cancel, &[]),
        (1, PointerPhase::Cancel, &[("D", "end")]),
    ];
    for (time, &(pointer, phase, expected)) in events.iter().enumerate() {
        let coordinate = 9. + pointer as f32;
        dispatch(
            &mut tree,
            pointer,
            phase,
            Offset::new(coordinate, coordinate),
            now + Duration::from_millis(time as u64),
        );
        assert_eq!(&log.take(), expected, "pointer {pointer} {phase:?}");
    }
}
