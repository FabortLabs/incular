//! Two pinned Ticker time-dilation cases plus the scheduler's default.
use incular_animation::{Ticker, set_time_dilation, time_dilation};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[test]
fn time_dilation_defaults_to_one_without_frames() {
    assert_eq!(time_dilation(), 1.0);
}

fn verify_ticker(dilation: f64, expected_elapsed_ms: u64) {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let start = Instant::now();
    runtime.run_frame_at(bounds, start).unwrap();

    set_time_dilation(dilation);
    let elapsed = Rc::new(RefCell::new(Vec::new()));
    let observed = elapsed.clone();
    let ticker = Ticker::new(move |delta| observed.borrow_mut().push(delta));
    let _future = ticker.start();

    for ms in [10, 20] {
        runtime
            .run_frame_at(bounds, start + Duration::from_millis(ms))
            .unwrap();
    }
    assert_eq!(
        elapsed.borrow().as_slice(),
        [Duration::ZERO, Duration::from_millis(expected_elapsed_ms)]
    );
    drop(ticker);
    set_time_dilation(1.0);
}

#[test]
fn ticker_is_twice_as_fast_at_half_dilation() {
    verify_ticker(0.5, 20);
}

#[test]
fn ticker_is_half_as_fast_at_double_dilation() {
    verify_ticker(2.0, 5);
}
