use std::{
    cell::{Cell, RefCell},
    future::{Future, poll_fn},
    pin::pin,
    task::{Context, Poll, Waker},
};

use incular_core::{
    debug_instrument_action, debug_instrumentation_enabled, set_debug_instrumentation_enabled,
};

fn expect_timing(message: &str, about: &str) {
    let duration = message
        .strip_prefix(&format!("Action \"{about}\" took "))
        .unwrap();
    let pieces: Vec<_> = duration.split([':', '.']).collect();
    assert_eq!(pieces.len(), 4, "{duration:?}");
    assert!(
        pieces
            .iter()
            .all(|piece| !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()))
    );
    assert_eq!(pieces[3].len(), 6);
}

#[test]
#[cfg(debug_assertions)]
fn successful_action_logs_timing_after_its_output() {
    assert!(!set_debug_instrumentation_enabled(true));
    assert!(debug_instrumentation_enabled());
    let lines = RefCell::new(Vec::new());
    let result = pin!(debug_instrument_action(
        "no-op",
        || async {
            lines.borrow_mut().push("action()".to_string());
            1
        },
        |line| lines.borrow_mut().push(line.to_owned()),
    ))
    .poll(&mut Context::from_waker(Waker::noop()));
    assert_eq!(result, Poll::Ready(1));
    assert_eq!(lines.borrow().len(), 2);
    assert_eq!(lines.borrow()[0], "action()");
    expect_timing(&lines.borrow()[1], "no-op");
    assert!(set_debug_instrumentation_enabled(false));
}

#[test]
#[cfg(debug_assertions)]
fn asynchronous_failure_is_returned_unchanged_and_still_timed() {
    #[derive(Debug, PartialEq)]
    struct Failed(u8);

    set_debug_instrumentation_enabled(true);
    let first = Cell::new(true);
    let lines = RefCell::new(Vec::new());
    let future = debug_instrument_action(
        "throws",
        || {
            poll_fn(|cx| {
                if first.replace(false) {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Err::<(), _>(Failed(23)))
                }
            })
        },
        |line| lines.borrow_mut().push(line.to_owned()),
    );
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
    assert!(lines.borrow().is_empty());
    assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(Err(Failed(23))));
    assert_eq!(lines.borrow().len(), 1);
    expect_timing(&lines.borrow()[0], "throws");
    set_debug_instrumentation_enabled(false);
}

#[test]
fn disabled_instrumentation_runs_without_logging() {
    set_debug_instrumentation_enabled(false);
    assert!(!debug_instrumentation_enabled());
    let result = pin!(debug_instrument_action(
        "silent",
        || async { 31 },
        |_| panic!("disabled instrumentation must not touch the sink"),
    ))
    .poll(&mut Context::from_waker(Waker::noop()));
    assert_eq!(result, Poll::Ready(31));
}

#[test]
#[cfg(debug_assertions)]
fn dropping_a_pending_action_does_not_emit_a_completion() {
    set_debug_instrumentation_enabled(true);
    let logged = Cell::new(false);
    {
        let mut pending = pin!(debug_instrument_action(
            "cancelled",
            std::future::pending::<()>,
            |_| logged.set(true),
        ));
        assert_eq!(
            pending
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );
    }
    assert!(!logged.get());
    set_debug_instrumentation_enabled(false);
}

#[test]
#[cfg(all(debug_assertions, panic = "unwind"))]
fn unwinding_reports_timing_without_changing_the_original_payload() {
    use std::panic::{AssertUnwindSafe, catch_unwind, panic_any};

    #[derive(Debug, PartialEq)]
    struct Failed;
    let lines = RefCell::new(Vec::new());
    set_debug_instrumentation_enabled(true);
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = pin!(debug_instrument_action(
            "panic",
            || async { panic_any(Failed) },
            |line| lines.borrow_mut().push(line.to_owned()),
        ))
        .poll(&mut Context::from_waker(Waker::noop()));
    }))
    .unwrap_err();
    assert_eq!(panic.downcast_ref(), Some(&Failed));
    assert_eq!(lines.borrow().len(), 1);
    expect_timing(&lines.borrow()[0], "panic");
    set_debug_instrumentation_enabled(false);
}
