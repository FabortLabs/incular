//! The frame scheduler the runtime drives: Flutter's `Ticker`,
//! `scheduleFrameCallback`, `addPostFrameCallback`, and controllers with
//! `vsync:` that tick themselves.

use incular_animation::{
    AnimationController, AnimationStatus, Ticker, add_post_frame_callback, cancel_frame_callback,
    schedule_frame, schedule_frame_callback,
};
use incular_config::Constraints;
use incular_core::time::Instant;
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::Widget;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(100., 100.))
}

fn runtime() -> Runtime {
    let mut runtime =
        Runtime::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).expect("runtime");
    runtime
        .run_frame_at(bounds(), Instant::now())
        .expect("first frame");
    runtime
}

#[test]
fn a_ticker_ticks_once_per_frame_from_its_first_frame() {
    let mut runtime = runtime();
    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    });
    assert!(!runtime.frame_requested());
    ticker.start();
    assert!(ticker.is_ticking());
    assert!(
        runtime.frame_requested(),
        "a running ticker asks for frames"
    );
    let base = Instant::now();
    for frame in 0..3 {
        runtime
            .run_frame_at(bounds(), base + Duration::from_millis(16 * frame))
            .unwrap();
    }
    assert_eq!(
        ticks.borrow().as_slice(),
        &[
            Duration::ZERO,
            Duration::from_millis(16),
            Duration::from_millis(32)
        ]
    );
    ticker.set_muted(true);
    assert!(!runtime.frame_requested(), "a muted ticker asks for none");
    runtime
        .run_frame_at(bounds(), base + Duration::from_millis(48))
        .unwrap();
    assert_eq!(ticks.borrow().len(), 3);
    ticker.set_muted(false);
    ticker.stop();
    assert!(!runtime.frame_requested());
}

#[test]
fn frame_callbacks_run_before_the_build_and_can_be_cancelled() {
    let mut runtime = runtime();
    let seen = Rc::new(Cell::new(None));
    let cancelled = Rc::new(Cell::new(false));
    schedule_frame_callback({
        let seen = seen.clone();
        move |now| seen.set(Some(now))
    });
    let id = schedule_frame_callback({
        let cancelled = cancelled.clone();
        move |_| cancelled.set(true)
    });
    assert!(cancel_frame_callback(id));
    assert!(runtime.frame_requested());
    let now = Instant::now();
    runtime.run_frame_at(bounds(), now).unwrap();
    assert_eq!(seen.get(), Some(now));
    assert!(!cancelled.get());
    assert!(!runtime.frame_requested());
}

#[test]
fn post_frame_callbacks_run_once_after_the_frame_and_ask_for_none() {
    let mut runtime = runtime();
    let runs = Rc::new(Cell::new(0));
    add_post_frame_callback({
        let runs = runs.clone();
        move |_| runs.set(runs.get() + 1)
    });
    assert!(!runtime.frame_requested(), "Flutter: no frame is scheduled");
    schedule_frame();
    assert!(runtime.frame_requested());
    runtime.run_frame_at(bounds(), Instant::now()).unwrap();
    assert_eq!(runs.get(), 1);
    runtime.run_frame_at(bounds(), Instant::now()).unwrap();
    assert_eq!(runs.get(), 1);
}

#[test]
fn a_vsync_controller_ticks_itself_until_it_completes() {
    let mut runtime = runtime();
    let controller = AnimationController::new(Duration::from_millis(100)).with_vsync();
    let statuses = Rc::new(RefCell::new(Vec::new()));
    controller.add_status_listener({
        let statuses = statuses.clone();
        move |status| statuses.borrow_mut().push(status)
    });
    controller.forward_on_next_tick();
    assert!(runtime.frame_requested());
    let base = Instant::now();
    runtime.run_frame_at(bounds(), base).unwrap();
    assert_eq!(controller.value(), 0.);
    runtime
        .run_frame_at(bounds(), base + Duration::from_millis(50))
        .unwrap();
    assert!((controller.value() - 0.5).abs() < 1e-4);
    runtime
        .run_frame_at(bounds(), base + Duration::from_millis(100))
        .unwrap();
    assert_eq!(controller.value(), 1.);
    // Flutter completes on the first frame past the duration.
    assert_eq!(controller.status(), AnimationStatus::Forward);
    runtime
        .run_frame_at(bounds(), base + Duration::from_millis(116))
        .unwrap();
    assert_eq!(controller.status(), AnimationStatus::Completed);
    assert!(!runtime.frame_requested(), "a finished controller idles");
    assert_eq!(
        statuses.borrow().as_slice(),
        &[AnimationStatus::Forward, AnimationStatus::Completed]
    );
}
