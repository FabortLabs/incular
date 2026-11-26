//! Completion timing from Flutter 3.50.0 widgets/page_view_test.dart:413.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};

use incular_animation::Curve;
use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, PageController, PageView, Placeholder, ScrollAnimationCompletion, Widget,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const ANIMATION: Duration = Duration::from_millis(150);

#[derive(Default)]
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn poll(completion: &mut ScrollAnimationCompletion, waker: &Waker) -> Poll<()> {
    Pin::new(completion).poll(&mut Context::from_waker(waker))
}

fn scene(controller: &PageController) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        PageView::new((0..4).map(|_| Placeholder::new())).controller(controller.clone()),
    )
    .into()
}

fn step(runtime: &mut Runtime, time: Instant, milliseconds: u64) {
    runtime
        .run_frame_at(
            Constraints::tight(VIEW),
            time + Duration::from_millis(milliseconds),
        )
        .expect("frame");
}

#[test]
fn page_animation_completions_wait_for_the_driven_frames() {
    let controller = PageController::new();
    let mut runtime = Runtime::new(scene(&controller)).expect("runtime");
    let time = Instant::now();
    step(&mut runtime, time, 0);
    let wakes = Arc::new(WakeCount::default());
    let waker = Waker::from(wakes.clone());

    let mut next = controller
        .next_page_async(ANIMATION, Curve::Ease)
        .expect("attached");
    assert_eq!(poll(&mut next, &waker), Poll::Pending);
    step(&mut runtime, time, 200);
    assert_eq!(poll(&mut next, &waker), Poll::Pending);
    assert_eq!(wakes.0.load(Ordering::Relaxed), 0);
    step(&mut runtime, time, 400);
    assert_eq!(poll(&mut next, &waker), Poll::Ready(()));
    assert_eq!(wakes.0.load(Ordering::Relaxed), 1);
    assert_eq!(controller.page(), Some(1.));

    let mut previous = controller
        .previous_page_async(ANIMATION, Curve::Ease)
        .expect("attached");
    assert_eq!(poll(&mut previous, &waker), Poll::Pending);
    step(&mut runtime, time, 600);
    assert_eq!(poll(&mut previous, &waker), Poll::Pending);
    step(&mut runtime, time, 800);
    assert_eq!(poll(&mut previous, &waker), Poll::Ready(()));
    assert_eq!(wakes.0.load(Ordering::Relaxed), 2);
    assert_eq!(controller.page(), Some(0.));
}

#[test]
fn completion_is_scoped_to_the_request_that_was_replaced_or_interrupted() {
    let controller = PageController::new();
    let mut runtime = Runtime::new(scene(&controller)).expect("runtime");
    let time = Instant::now();
    step(&mut runtime, time, 0);

    let replaced = controller
        .animate_to_page_async(1, ANIMATION, Curve::Ease)
        .unwrap();
    let active = controller
        .animate_to_page_async(2, ANIMATION, Curve::Ease)
        .unwrap();
    assert!(replaced.is_complete());
    assert!(!active.is_complete());

    step(&mut runtime, time, 25);
    step(&mut runtime, time, 50);
    assert!(!active.is_complete());
    let replacement = controller
        .animate_to_page_async(1, ANIMATION, Curve::Ease)
        .unwrap();
    step(&mut runtime, time, 75);
    assert!(
        active.is_complete(),
        "the previous driver released its own completion"
    );
    assert!(!replacement.is_complete());
    assert!(controller.jump_to_page(3));
    step(&mut runtime, time, 100);
    assert!(replacement.is_complete());
    assert_eq!(controller.page(), Some(3.));

    let pending_jump = controller
        .animate_to_page_async(1, ANIMATION, Curve::Ease)
        .unwrap();
    assert!(
        !controller.jump_to_page(3),
        "the same-position jump cancels only the queued motion"
    );
    assert!(pending_jump.is_complete());

    let pending = controller
        .animate_to_page_async(0, ANIMATION, Curve::Ease)
        .unwrap();
    drop(runtime);
    assert!(
        pending.is_complete(),
        "pending request resolves when its viewport goes away"
    );
    assert!(controller.next_page_async(ANIMATION, Curve::Ease).is_none());
}

#[test]
fn zero_viewport_deferred_page_completes_without_waiting_for_a_ticker() {
    let controller = PageController::new();
    let mut runtime = Runtime::new(scene(&controller)).unwrap();
    let time = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(0., 600.)), time)
        .unwrap();
    let completion = controller
        .animate_to_page_async(2, ANIMATION, Curve::Ease)
        .unwrap();
    assert!(completion.is_complete());
    step(&mut runtime, time, 25);
    assert_eq!(controller.page(), Some(2.));
}
