use std::{
    cell::{Cell, RefCell},
    panic::panic_any,
    rc::Rc,
    time::Duration,
};

use incular_core::{Offset, PointerPhase, time::Instant};
use incular_gestures::{
    GestureCallbacks, PointerEvent, PointerGestureRecognizer, TapCallbackError, TapCallbackKind,
    TapCancelReason, set_tap_callback_error_handler,
};

fn event(at: Instant, phase: PointerPhase, milliseconds: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position: Offset::ZERO,
        phase,
        time: at + Duration::from_millis(milliseconds),
    }
}

#[test]
fn tap_callback_failure_reports_the_typed_panic_at_release() {
    #[derive(Debug, PartialEq)]
    struct TapFailure(u32);
    let failures = Rc::new(RefCell::new(Vec::<TapCallbackError>::new()));
    let previous = set_tap_callback_error_handler(Some(Rc::new({
        let failures = failures.clone();
        move |failure| failures.borrow_mut().push(failure)
    })));
    let attempts = Rc::new(Cell::new(0));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap: Some(Rc::new({
            let attempts = attempts.clone();
            move || {
                let count = attempts.get() + 1;
                attempts.set(count);
                if count == 1 {
                    panic_any(TapFailure(17));
                }
            }
        })),
        ..GestureCallbacks::default()
    });

    let start = Instant::now();
    tap.handle(event(start, PointerPhase::Down, 0));
    assert!(failures.borrow().is_empty());
    tap.handle(event(start, PointerPhase::Up, 10));
    assert_eq!(failures.borrow()[0].callback, TapCallbackKind::Tap);
    assert_eq!(
        failures.borrow()[0].panic.downcast_ref(),
        Some(&TapFailure(17))
    );

    tap.handle(event(start, PointerPhase::Down, 400));
    tap.handle(event(start, PointerPhase::Up, 401));
    assert_eq!(
        attempts.get(),
        2,
        "the failed user callback did not poison the next press"
    );
    assert_eq!(failures.borrow().len(), 1);
    set_tap_callback_error_handler(previous);
}

#[test]
fn failed_tap_cancel_names_the_spontaneous_cancellation() {
    let failures = Rc::new(RefCell::new(Vec::<TapCallbackError>::new()));
    let previous = set_tap_callback_error_handler(Some(Rc::new({
        let failures = failures.clone();
        move |failure| failures.borrow_mut().push(failure)
    })));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_cancel: Some(Rc::new(|| panic!("cancel failed"))),
        ..GestureCallbacks::default()
    });
    let start = Instant::now();
    tap.handle(event(start, PointerPhase::Down, 0));
    tap.accept_tap_on_arena_close(1);
    let _ = tap.poll(start + Duration::from_millis(500));
    tap.handle(event(start, PointerPhase::Cancel, 501));
    assert_eq!(failures.borrow().len(), 1);
    let failure = &failures.borrow()[0];
    assert_eq!(
        failure.callback,
        TapCallbackKind::Cancel(TapCancelReason::Spontaneous)
    );
    assert_eq!(failure.message(), Some("cancel failed"));
    assert!(failure.to_string().contains("\"spontaneous onTapCancel\""));
    set_tap_callback_error_handler(previous);
}

#[test]
fn a_failed_tap_up_does_not_skip_the_terminal_tap() {
    let reports = Rc::new(Cell::new(0));
    let previous = set_tap_callback_error_handler(Some(Rc::new({
        let reports = reports.clone();
        move |_| reports.set(reports.get() + 1)
    })));
    let taps = Rc::new(Cell::new(0));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_up: Some(Rc::new(|_| panic!("tap up failed"))),
        on_tap: Some(Rc::new({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        })),
        ..GestureCallbacks::default()
    });
    let start = Instant::now();
    tap.handle(event(start, PointerPhase::Down, 0));
    tap.handle(event(start, PointerPhase::Up, 10));
    assert_eq!(reports.get(), 1);
    assert_eq!(taps.get(), 1);
    set_tap_callback_error_handler(previous);
}
