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
