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
