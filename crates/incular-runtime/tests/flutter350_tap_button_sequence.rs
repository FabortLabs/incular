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
