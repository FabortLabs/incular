//! Ticker scheduling regression against pinned Flutter 3.50.0 (C04).

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::{Ticker, schedule_frame_callback};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

#[test]
fn starting_ticker_during_transient_callbacks_defers_its_first_tick() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();

    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Rc::new(Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    }));
    schedule_frame_callback({
        let ticker = ticker.clone();
        move |_| {
            ticker.start();
        }
    });

    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    let first_batch = ticks.borrow().clone();
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(16))
        .unwrap();
    assert!(
        first_batch.is_empty(),
        "new ticker ran in the current batch: {first_batch:?}"
    );
    assert_eq!(ticks.borrow().as_slice(), &[Duration::from_millis(16)]);
}

#[test]
fn restarting_a_ticker_during_the_batch_cancels_its_old_tick() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();

    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Rc::new(Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    }));
    // Register the ordinary callback first so it precedes the old tick.
    schedule_frame_callback({
        let ticker = ticker.clone();
        move |_| {
            ticker.stop();
            ticker.start();
        }
    });
    ticker.start();
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(10))
        .unwrap();
    assert!(ticks.borrow().is_empty());
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(26))
        .unwrap();
    assert_eq!(ticks.borrow().as_slice(), &[Duration::from_millis(16)]);
}

#[test]
fn ticker_callbacks_share_registration_order_with_other_transients() {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    schedule_frame_callback({
        let log = log.clone();
        move |_| log.borrow_mut().push("first")
    });
    let ticker = Ticker::new({
        let log = log.clone();
        move |_| log.borrow_mut().push("ticker")
    });
    ticker.start();
    schedule_frame_callback({
        let log = log.clone();
        move |_| log.borrow_mut().push("last")
    });
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(16))
        .unwrap();
    assert_eq!(&*log.borrow(), &["first", "ticker", "last"]);

    log.borrow_mut().clear();
    schedule_frame_callback({
        let log = log.clone();
        move |_| log.borrow_mut().push("later registration")
    });
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(32))
        .unwrap();
    assert_eq!(&*log.borrow(), &["ticker", "later registration"]);
}

#[test]
fn unmuting_in_a_transient_waits_for_the_next_frame_and_starts_at_zero() {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Rc::new(Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    }));
    ticker.set_muted(true);
    ticker.start();
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(10))
        .unwrap();
    assert!(ticks.borrow().is_empty());
    schedule_frame_callback({
        let ticker = ticker.clone();
        move |_| ticker.set_muted(false)
    });
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(20))
        .unwrap();
    assert!(ticks.borrow().is_empty());
    assert!(runtime.frame_requested());
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(36))
        .unwrap();
    assert_eq!(&*ticks.borrow(), &[Duration::ZERO]);
}

// scheduler/ticker_test.dart:21, excluding Dart's duplicate-start diagnostic.
#[test]
fn muting_preserves_an_active_run_and_does_not_restart_a_stopped_one() {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    });
    assert!(!ticker.is_ticking() && !ticker.is_active());
    ticker.start();
    assert!(ticker.is_ticking() && ticker.is_active());
    assert!(ticks.borrow().is_empty());

    runtime
        .run_frame_at(bounds, base + Duration::from_millis(10))
        .unwrap();
    assert_eq!(&*ticks.borrow(), &[Duration::ZERO]);
    ticker.set_muted(true);
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(20))
        .unwrap();
    assert!(!ticker.is_ticking() && ticker.is_active());
    assert_eq!(ticks.borrow().len(), 1);
    ticker.set_muted(false);
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(30))
        .unwrap();
    assert!(ticker.is_ticking() && ticker.is_active());
    assert_eq!(
        &*ticks.borrow(),
        &[Duration::ZERO, Duration::from_millis(20)]
    );

    ticker.set_muted(true);
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(40))
        .unwrap();
    assert!(!ticker.is_ticking() && ticker.is_active());
    assert_eq!(ticks.borrow().len(), 2);
    ticker.stop();
    assert!(!ticker.is_ticking() && !ticker.is_active());
    ticker.set_muted(false);
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(50))
        .unwrap();
    assert!(!ticker.is_ticking() && !ticker.is_active());
    assert_eq!(ticks.borrow().len(), 2);
}
