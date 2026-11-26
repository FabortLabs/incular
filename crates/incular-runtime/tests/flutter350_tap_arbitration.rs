use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, GestureDetector, HitTestBehavior, Widget};

#[test]
fn retained_tap_up_waits_for_a_double_tap_competitor_to_leave_the_arena() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let record = |event| {
        let log = log.clone();
        move || log.borrow_mut().push(event)
    };
    let down = record("down");
    let up = record("up");
    let scene = Center::new(
        GestureDetector::new(Widget::box_(Size::new(120., 120.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(move |_| down())
            .on_tap_up(move |_| up())
            .on_tap(record("single"))
            .on_tap_cancel(record("cancel"))
            .on_double_tap(record("double")),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(200., 200.));
    let base = Instant::now();
    let at = |ms| base + Duration::from_millis(ms);
    runtime.run_frame_at(bounds, base).unwrap();
    let route = |runtime: &mut Runtime, pointer, phase, time| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                position: Offset::new(100., 100.),
                sample: PointerSampleMetadata::EMPTY,
            },
            at(time),
        );
    };
    let press = |runtime: &mut Runtime, pointer, from| {
        route(runtime, pointer, PointerPhase::Down, from);
        route(runtime, pointer, PointerPhase::Up, from + 10);
    };
    press(&mut runtime, 1, 0);
    assert!(
        log.borrow().is_empty(),
        "held tap reported {:?}",
        *log.borrow()
    );
    runtime.run_frame_at(bounds, at(110)).unwrap();
    assert!(log.borrow().is_empty());
    runtime.run_frame_at(bounds, at(310)).unwrap();
    assert_eq!(*log.borrow(), ["down", "up", "single"]);

    log.borrow_mut().clear();
    press(&mut runtime, 2, 320);
    press(&mut runtime, 3, 370);
    assert_eq!(*log.borrow(), ["double"]);
    runtime.run_frame_at(bounds, at(730)).unwrap();
    assert_eq!(*log.borrow(), ["double"]);

    log.borrow_mut().clear();
    route(&mut runtime, 4, PointerPhase::Down, 740);
    runtime.run_frame_at(bounds, at(850)).unwrap();
    assert_eq!(*log.borrow(), ["down"]);
    route(&mut runtime, 4, PointerPhase::Up, 850);
    assert_eq!(*log.borrow(), ["down"]);
    runtime.run_frame_at(bounds, at(1150)).unwrap();
    assert_eq!(*log.borrow(), ["down", "up", "single"]);

    log.borrow_mut().clear();
    route(&mut runtime, 5, PointerPhase::Down, 1160);
    runtime.run_frame_at(bounds, at(1270)).unwrap();
    route(&mut runtime, 5, PointerPhase::Up, 1270);
    assert_eq!(*log.borrow(), ["down"]);
    press(&mut runtime, 6, 1310);
    assert_eq!(*log.borrow(), ["down", "double"]);
}
