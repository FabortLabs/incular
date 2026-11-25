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
