#![cfg(debug_assertions)]

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::{
    Ticker,
    scheduler::{begin_frame, end_frame},
};
use incular_core::time::Instant;

#[inline(never)]
fn create_test_ticker() -> Ticker {
    Ticker::new(|_| {})
}

#[test]
fn ticker_debug_is_one_line_and_optional_stack_identifies_constructor() {
    let ticker = create_test_ticker();
    assert_eq!(ticker.debug_description(false), "Ticker()");
    assert_eq!(format!("{ticker:?}").lines().count(), 1);
    let trace = ticker.debug_description(true);
    assert!(trace.starts_with("Ticker()\n"));
    assert!(trace.contains("create_test_ticker"), "{trace}");
}

#[test]
fn active_ticker_rejects_duplicate_start_without_disturbing_muting_or_run() {
    let ticks = Rc::new(Cell::new(0));
    let ticker = Ticker::new({
        let ticks = ticks.clone();
        move |_| ticks.set(ticks.get() + 1)
    });
    let state = |active, ticking, count| {
        assert_eq!(
            (ticker.is_active(), ticker.is_ticking(), ticks.get()),
            (active, ticking, count)
        );
    };
    state(false, false, 0);
    let _future = ticker.start();
    state(true, true, 0);
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ticker.start()))
        .err()
        .expect("duplicate start must be rejected");
    let error = failed.downcast_ref::<String>().expect("debug diagnostic");
    assert!(error.starts_with("A ticker was started twice."));
    assert!(error.contains("first stopping it."));
    assert!(error.contains("The affected ticker was:\nTicker()"));
    state(true, true, 0);

    let origin = Instant::now();
    let frame = |step: u64| {
        let at = origin + Duration::from_millis(step * 10);
        begin_frame(at);
        end_frame(at);
    };
    frame(1);
    state(true, true, 1);
    ticker.set_muted(true);
    frame(2);
    state(true, false, 1);
    ticker.set_muted(false);
    frame(3);
    state(true, true, 2);
    ticker.set_muted(true);
    frame(4);
    state(true, false, 2);
    ticker.stop();
    state(false, false, 2);
    ticker.set_muted(false);
    state(false, false, 2);
    frame(5);
    state(false, false, 2);
}

#[test]
fn ticker_observers_receive_matching_create_and_dispose_id() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};
    use std::cell::RefCell;
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let seen = seen.clone();
        move |entry| seen.borrow_mut().push(entry)
    });
    let ticker = Ticker::new(|_| {});
    assert_eq!(seen.borrow().len(), 1);
    drop(ticker);
    let values = seen.borrow();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].kind, ObjectEventKind::Created);
    assert_eq!(values[1].kind, ObjectEventKind::Disposed);
    assert_eq!(values[0].id, values[1].id);
    assert_eq!(values[0].library, "scheduler");
    assert_eq!(values[0].class_name, "Ticker");
}
