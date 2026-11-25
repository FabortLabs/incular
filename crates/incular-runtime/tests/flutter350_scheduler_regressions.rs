//! Frame-callback regression from Flutter 3.50's SchedulerBinding.
//! See `audits/flutter-3.50.0-parity/behavior-probes.md` (B06).

use incular_animation::{
    Ticker, add_post_frame_callback, cancel_frame_callback, schedule_frame_callback,
};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

/// scheduler/animation_test.dart:21: first frame runs only the first
/// callback, cancellation prevents the second from leaking to frame two.
#[test]
fn flutter_queued_callback_cancellation_preserves_the_first_frame_epoch() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let first_frame = Instant::now() + Duration::from_millis(16);
    let second_id = Rc::new(Cell::new(None));
    let observed = Rc::new(Cell::new((false, false)));

    schedule_frame_callback({
        let second_id = second_id.clone();
        let observed = observed.clone();
        move |timestamp| {
            assert_eq!(observed.get(), (false, false));
            assert_eq!(timestamp, first_frame);
            observed.set((true, false));
            assert!(cancel_frame_callback(second_id.get().unwrap()));
        }
    });
    second_id.set(Some(schedule_frame_callback({
        let observed = observed.clone();
        move |timestamp| {
            assert_eq!(observed.get(), (true, false));
            assert_eq!(timestamp, first_frame);
            observed.set((true, true));
        }
    })));
    runtime.run_frame_at(bounds, first_frame).unwrap();
    assert_eq!(observed.get(), (true, false));

    observed.set((false, false));
    runtime
        .run_frame_at(bounds, first_frame + Duration::from_millis(16))
        .unwrap();
    assert_eq!(observed.get(), (false, false));
}

#[test]
fn transient_callback_can_cancel_a_later_callback_in_the_same_batch() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    let second_id = Rc::new(Cell::new(None));
    let callbacks = Rc::new(RefCell::new(Vec::new()));

    schedule_frame_callback({
        let second_id = second_id.clone();
        let callbacks = callbacks.clone();
        move |_| {
            callbacks.borrow_mut().push('A');
            let id = second_id.get().unwrap();
            assert!(
                cancel_frame_callback(id),
                "later work in this frame is still pending"
            );
            assert!(!cancel_frame_callback(id));
            let next = callbacks.clone();
            schedule_frame_callback(move |_| next.borrow_mut().push('D'));
        }
    });
    second_id.set(Some(schedule_frame_callback({
        let callbacks = callbacks.clone();
        move |_| callbacks.borrow_mut().push('B')
    })));
    schedule_frame_callback({
        let callbacks = callbacks.clone();
        move |_| callbacks.borrow_mut().push('C')
    });

    runtime.run_frame_at(bounds, Instant::now()).unwrap();

    assert_eq!(callbacks.borrow().as_slice(), &['A', 'C']);
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    assert_eq!(callbacks.borrow().as_slice(), &['A', 'C', 'D']);
}

#[test]
fn a_ticker_started_in_post_frame_has_zero_elapsed_at_its_first_tick() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    let elapsed = Rc::new(RefCell::new(Vec::new()));
    let ticker = Rc::new(Ticker::new({
        let elapsed = elapsed.clone();
        move |tick| elapsed.borrow_mut().push(tick)
    }));
    add_post_frame_callback({
        let ticker = ticker.clone();
        move |_| {
            let _ = ticker.start();
        }
    });
    let now = Instant::now();
    runtime.run_frame_at(bounds, now).unwrap();
    assert!(elapsed.borrow().is_empty());
    let interval = std::time::Duration::from_millis(16);
    runtime.run_frame_at(bounds, now + interval).unwrap();
    runtime.run_frame_at(bounds, now + interval * 2).unwrap();
    assert_eq!(*elapsed.borrow(), [std::time::Duration::ZERO, interval]);
    ticker.stop();
}

#[test]
fn cancelling_the_last_transient_callback_keeps_an_already_requested_frame() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    assert!(!runtime.frame_requested());
    let id = schedule_frame_callback(|_| panic!("cancelled callback ran"));
    assert!(runtime.frame_requested());
    assert!(cancel_frame_callback(id));
    assert!(runtime.frame_requested());
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
    assert!(!runtime.frame_requested());
}

/// scheduler/scheduler_test.dart:339, through both the scheduler and host
/// demand checks. A supplied frame runs passive callbacks in transient phase.
#[test]
fn a_frame_callback_can_wait_without_requesting_a_frame() {
    use incular_animation::{schedule_frame_callback_with_request, scheduler::has_scheduled_frame};
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame_at(bounds, Instant::now()).unwrap();

    for request in [false, true] {
        let invoked = Rc::new(Cell::new(None));
        let at = Instant::now();
        schedule_frame_callback_with_request(
            {
                let invoked = invoked.clone();
                move |time| invoked.set(Some(time))
            },
            request,
        );
        assert_eq!(has_scheduled_frame(), request);
        assert_eq!(runtime.frame_requested(), request);
        assert_eq!(invoked.get(), None);
        runtime.run_frame_at(bounds, at).unwrap();
        assert_eq!(invoked.get(), Some(at));
        assert!(!has_scheduled_frame());
        assert!(!runtime.frame_requested());
    }

    let passive = schedule_frame_callback_with_request(|_| panic!("passive callback ran"), false);
    assert!(cancel_frame_callback(passive));
    assert!(!has_scheduled_frame());
    runtime.run_frame_at(bounds, Instant::now()).unwrap();
}
