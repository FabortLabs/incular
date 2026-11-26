use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    SECONDARY_POINTER_BUTTON, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, RawPointerEvent, Widget};

#[test]
fn a_retained_tap_cancels_when_the_pressed_mouse_buttons_change() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let callback = |name| {
        let log = log.clone();
        move || log.borrow_mut().push(name)
    };
    let down = callback("down");
    let up = callback("up");
    let moved = callback("move");
    let widget = GestureDetector::new(Widget::box_(Size::new(60., 60.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap_down(move |_| down())
        .on_tap_up(move |_| up())
        .on_tap_move(move |_| moved())
        .on_tap_cancel(callback("cancel"))
        .on_tap(callback("tap"));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let clock = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(60., 60.)), clock)
        .unwrap();
    let send = |runtime: &mut Runtime, phase, buttons, millis| {
        let _ = runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                1,
                0,
                PointerDeviceKind::Mouse,
                buttons,
                Offset::new(20., 20.),
                phase,
                clock + Duration::from_millis(millis),
            )
            .with_button(
                matches!(phase, PointerPhase::Down | PointerPhase::Up)
                    .then_some(PRIMARY_POINTER_BUTTON),
            ),
        );
    };
    send(&mut runtime, PointerPhase::Down, PRIMARY_POINTER_BUTTON, 1);
    runtime
        .run_frame_at(
            Constraints::tight(Size::new(60., 60.)),
            clock + Duration::from_millis(120),
        )
        .unwrap();
    assert_eq!(*log.borrow(), ["down"], "the press deadline delivers down");
    send(
        &mut runtime,
        PointerPhase::Move,
        PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON,
        121,
    );
    send(
        &mut runtime,
        PointerPhase::Move,
        PRIMARY_POINTER_BUTTON,
        122,
    );
    send(&mut runtime, PointerPhase::Up, 0, 123);
    assert_eq!(
        *log.borrow(),
        ["down", "cancel"],
        "no move, up or click after rejection"
    );

    send(
        &mut runtime,
        PointerPhase::Down,
        PRIMARY_POINTER_BUTTON,
        124,
    );
    send(&mut runtime, PointerPhase::Up, 0, 125);
    assert_eq!(*log.borrow(), ["down", "cancel", "down", "up", "tap"]);
}

#[test]
fn slight_mouse_drift_does_not_cancel_a_retained_primary_tap() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |label| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(label)
    };
    let widget = GestureDetector::new(Widget::box_(Size::new(60., 60.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap(record("tap"))
        .on_tap_cancel(record("cancel"));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let clock = Instant::now();
    let bounds = Constraints::tight(Size::new(60., 60.));
    runtime.run_frame_at(bounds, clock).unwrap();

    for (index, (distance, expected)) in [(2., "tap"), (18., "tap"), (18.25, "cancel")]
        .into_iter()
        .enumerate()
    {
        let start = index as u64 * 200 + 1;
        trace.borrow_mut().clear();
        let route = |runtime: &mut Runtime, phase, x, ms| {
            let buttons = if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            };
            let button = matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON);
            let _ = runtime.handle_raw_pointer_event(
                RawPointerEvent::new(
                    1,
                    0,
                    PointerDeviceKind::Mouse,
                    buttons,
                    Offset::new(x, 20.),
                    phase,
                    clock + Duration::from_millis(ms),
                )
                .with_button(button),
            );
        };
        route(&mut runtime, PointerPhase::Down, 20., start);
        runtime
            .run_frame_at(bounds, clock + Duration::from_millis(start + 120))
            .unwrap();
        route(
            &mut runtime,
            PointerPhase::Move,
            20. + distance,
            start + 121,
        );
        route(&mut runtime, PointerPhase::Up, 20. + distance, start + 122);
        assert_eq!(*trace.borrow(), [expected], "moved {distance} logical px");
    }
}
