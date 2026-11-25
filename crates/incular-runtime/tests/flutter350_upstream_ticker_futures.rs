//! Animation-controller futures from Flutter 3.50.0 animation/futures_test.dart.

use std::{
    cell::RefCell,
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

use incular_animation::{AnimationController, TickerCanceled};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

fn frame_driver() -> impl FnMut(u64) {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    move |millis| {
        runtime
            .run_frame_at(bounds, base + Duration::from_millis(millis))
            .unwrap();
    }
}

fn controller() -> AnimationController {
    AnimationController::new(Duration::from_millis(100)).with_vsync()
}

#[test]
fn creating_or_cancel_after_completion_returns_a_ready_future() {
    // Upstream line 153: both the original await and the later orCancel
    // await must finish. The Rust executor protocol exposes readiness by poll.
    let mut frame = frame_driver();
    let controller = controller();
    let future = controller.forward_on_next_tick();
    frame(0);
    frame(200);

    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(pin!(future.clone()).poll(&mut context), Poll::Ready(()));
    assert_eq!(
        pin!(future.or_cancel()).poll(&mut context),
        Poll::Ready(Ok(())),
    );
}

#[test]
fn creating_or_cancel_after_stop_returns_a_ready_cancellation() {
    // Upstream line 167, which has the same Dart test name as line 153.
    let mut frame = frame_driver();
    let controller = controller();
    let future = controller.forward_on_next_tick();
    frame(0);
    controller.stop();

    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(
        pin!(future.or_cancel()).poll(&mut context),
        Poll::Ready(Err(TickerCanceled)),
    );
}

#[test]
fn releasing_the_last_controller_handle_cancels_an_awaiting_run() {
    // Upstream line 122. Flutter explicitly disposes the controller; Incular
    // has no controller dispose method, so release the only owning handle.
    // One bounded poll models polling the waiting continuation at idle.
    let mut frame = frame_driver();
    let controller = controller();
    let log = RefCell::new(vec!["start"]);
    let future = controller.forward_on_next_tick();
    let mut continuation = pin!(async {
        match future.or_cancel().await {
            Ok(()) => log.borrow_mut().push("fail"),
            Err(TickerCanceled) => log.borrow_mut().push("caught"),
        }
        log.borrow_mut().push("end");
    });
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(continuation.as_mut().poll(&mut context), Poll::Pending);
    frame(0);
    assert_eq!(continuation.as_mut().poll(&mut context), Poll::Pending);
    assert_eq!(log.borrow().as_slice(), ["start"]);
    frame(50);
    assert_eq!(continuation.as_mut().poll(&mut context), Poll::Pending);
    assert_eq!(log.borrow().as_slice(), ["start"]);

    drop(controller);
    assert_eq!(log.borrow().as_slice(), ["start"]);
    assert_eq!(
        (continuation.poll(&mut context), log.borrow().as_slice()),
        (Poll::Ready(()), &["start", "caught", "end"][..]),
    );
}

#[test]
fn only_the_last_handle_cancels_even_without_a_scheduler_ticker() {
    let controller = AnimationController::new(Duration::from_millis(100));
    let other_owner = controller.clone();
    let future = controller.forward_on_next_tick();
    let mut cancel = pin!(future.or_cancel());
    let mut context = Context::from_waker(Waker::noop());
    drop(controller);
    assert_eq!(cancel.as_mut().poll(&mut context), Poll::Pending);
    drop(other_owner);
    assert_eq!(cancel.poll(&mut context), Poll::Ready(Err(TickerCanceled)));
}

#[derive(Default)]
struct WakeFlag(std::sync::atomic::AtomicBool);

impl std::task::Wake for WakeFlag {
    fn wake(self: std::sync::Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &std::sync::Arc<Self>) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

fn await_three_controllers_or_cancel(use_or_cancel: bool) {
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, atomic::Ordering},
    };

    let mut frame = frame_driver();
    let controllers = [100, 600, 300]
        .map(|millis| AnimationController::new(Duration::from_millis(millis)).with_vsync());
    let log = Rc::new(RefCell::new(vec!["start"]));
    let continuation = Rc::new(RefCell::new(Box::pin({
        let log = log.clone();
        async move {
            for (label, controller) in ["a", "b", "c"].into_iter().zip(controllers.iter()) {
                log.borrow_mut().push(label);
                let run = controller.forward_on_next_tick();
                if use_or_cancel {
                    run.or_cancel().await.unwrap();
                } else {
                    run.await;
                }
            }
            log.borrow_mut().extend(["d", "end"]);
        }
    })));
    let wake = Arc::new(WakeFlag::default());
    let waker = Waker::from(wake.clone());
    let ready = Rc::new(Cell::new(false));
    assert_eq!(
        continuation
            .borrow_mut()
            .as_mut()
            .poll(&mut Context::from_waker(&waker)),
        Poll::Pending,
    );
    assert_eq!(log.borrow().as_slice(), ["start", "a"]);

    let steps: &[(u64, &[&str])] = &[
        (0, &["start", "a"]),
        (0, &["start", "a"]),
        (50, &["start", "a"]),
        (150, &["start", "a", "b"]),
        (200, &["start", "a", "b"]),
        (600, &["start", "a", "b"]),
        (799, &["start", "a", "b", "c"]),
        (850, &["start", "a", "b", "c"]),
        (1200, &["start", "a", "b", "c", "d", "end"]),
        (1600, &["start", "a", "b", "c", "d", "end"]),
    ];
    for &(millis, expected) in steps {
        // Poll the UI-local await only when woken by a completion, in this
        // frame while its timestamp is still active. No wall-clock sleeps.
        incular_animation::scheduler::add_post_frame_callback({
            let continuation = continuation.clone();
            let wake = wake.clone();
            let waker = waker.clone();
            let ready = ready.clone();
            move |_| {
                if !ready.get() && wake.0.swap(false, Ordering::SeqCst) {
                    let poll = continuation
                        .borrow_mut()
                        .as_mut()
                        .poll(&mut Context::from_waker(&waker));
                    ready.set(poll.is_ready());
                }
            }
        });
        frame(millis);
        assert_eq!(log.borrow().as_slice(), expected, "after t={millis}");
    }
    assert!(ready.get());
}

#[test]
fn awaiting_three_direct_futures_preserves_frame_deadlines_and_log_order() {
    await_three_controllers_or_cancel(false);
}

#[test]
fn awaiting_three_or_cancel_futures_preserves_frame_deadlines_and_log_order() {
    await_three_controllers_or_cancel(true);
}

#[test]
fn a_completed_ticker_future_can_drive_a_generic_await_continuation() {
    async fn accepts_any_unit_future(future: impl Future<Output = ()>) -> bool {
        future.await;
        true
    }

    let mut frame = frame_driver();
    let animation = controller();
    let future = animation.forward_on_next_tick();
    frame(0);
    frame(200);
    let continuation = accepts_any_unit_future(future);
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(pin!(continuation).poll(&mut context), Poll::Ready(true));
}
