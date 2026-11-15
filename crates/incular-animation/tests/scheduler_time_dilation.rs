//! Flutter `timeDilation` (scheduler/binding.dart `_adjustForEpoch`,
//! `resetEpoch`): frame timestamps, tickers and frame callbacks slow
//! together, and a change does not make time jump.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::{
    Ticker, schedule_frame_callback, scheduler, set_time_dilation, time_dilation,
};
use incular_core::time::Instant;

/// Values from harness anim_core__scheduler_callbacks_time_dilation: with
/// `timeDilation = 2` set between frames, the next frame lands on the last
/// frame's timestamp, so a ticker started there reports 8 ms at the frame
/// 16 ms of real time later.
#[test]
fn dilation_slows_ticker_time_from_the_next_frame() {
    let base = Instant::now();
    let frame = |ms: u64| base + Duration::from_millis(ms);
    scheduler::begin_frame(frame(0));
    scheduler::end_frame(frame(0));

    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let ticker = Ticker::new({
        let elapsed = elapsed.clone();
        move |time| elapsed.set(time)
    });
    let stamps = Rc::new(Cell::new(None));
    schedule_frame_callback({
        let stamps = stamps.clone();
        move |time| stamps.set(Some(time))
    });
    set_time_dilation(2.0);
    assert_eq!(time_dilation(), 2.0);
    drop(ticker.start());

    scheduler::begin_frame(frame(16));
    assert_eq!(
        stamps.get(),
        Some(frame(0)),
        "the epoch starts at the last frame"
    );
    assert_eq!(elapsed.get(), Duration::ZERO);
    scheduler::end_frame(frame(16));
    scheduler::begin_frame(frame(32));
    assert_eq!(elapsed.get(), Duration::from_millis(8));
    scheduler::end_frame(frame(32));

    // Back to real time: the new epoch continues from the dilated stamp.
    set_time_dilation(1.0);
    scheduler::begin_frame(frame(48));
    scheduler::end_frame(frame(48));
    scheduler::begin_frame(frame(64));
    assert_eq!(elapsed.get(), Duration::from_millis(24));
    scheduler::end_frame(frame(64));
    assert_eq!(
        scheduler::frame_timestamp(frame(64)),
        frame(24),
        "dilated time trails real time by the slowed and lost intervals"
    );
}
