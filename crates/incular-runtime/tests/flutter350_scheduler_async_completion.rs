use std::{
    cell::{Cell, RefCell},
    future::{Future, poll_fn},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use incular_animation::scheduler::{
    Priority, has_ready_scheduled_task, reset_scheduling_strategy, run_one_scheduled_task,
    schedule_async_task, set_scheduling_strategy,
};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::{Runtime, RuntimeWake};
use incular_widgets::Widget;

async fn yield_once() {
    let mut yielded = false;
    poll_fn(|context| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

#[test]
fn scheduler_completion_waits_for_a_future_returning_task() {
    let completed = Rc::new(Cell::new(false));
    let mut task = std::pin::pin!(schedule_async_task(
        {
            let completed = completed.clone();
            move || async move {
                yield_once().await;
                yield_once().await;
                completed.set(true);
                1
            }
        },
        Priority::IDLE,
    ));
    let mut context = Context::from_waker(Waker::noop());
    assert!(task.as_mut().poll(&mut context).is_pending());

    set_scheduling_strategy(|_| false);
    assert!(run_one_scheduled_task());
    assert!(!completed.get());
    assert!(!has_ready_scheduled_task());
    reset_scheduling_strategy();
    assert!(run_one_scheduled_task()); // the initial callback reaches its first yield
    assert!(task.as_mut().poll(&mut context).is_pending());

    // After the callback starts, continuation wakes do not reenter its
    // original task priority gate.
    set_scheduling_strategy(|_| false);
    assert!(run_one_scheduled_task());
    assert!(task.as_mut().poll(&mut context).is_pending());
    assert!(!run_one_scheduled_task());
    reset_scheduling_strategy();
    assert!(completed.get());
    assert_eq!(task.as_mut().poll(&mut context), Poll::Ready(Ok(1)));
}

#[derive(Default)]
struct HostWakes(AtomicUsize);

impl RuntimeWake for HostWakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn cross_thread_wake_resumes_local_ui_work_without_spinning_the_host() {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let host = Arc::new(HostWakes::default());
    runtime.set_wake_handler(host.clone());
    runtime.run_frame(bounds).unwrap();
    assert!(!runtime.frame_requested());

    let ready = Arc::new(AtomicBool::new(false));
    let parked = Rc::new(RefCell::new(None::<Waker>));
    let ui_thread = std::thread::current().id();
    let mut task = std::pin::pin!(schedule_async_task(
        {
            let ready = ready.clone();
            let parked = parked.clone();
            move || {
                poll_fn(move |context| {
                    assert_eq!(std::thread::current().id(), ui_thread);
                    if ready.load(Ordering::SeqCst) {
                        Poll::Ready(7)
                    } else {
                        parked.borrow_mut().replace(context.waker().clone());
                        Poll::Pending
                    }
                })
            }
        },
        Priority::IDLE,
    ));
    assert!(runtime.frame_requested());
    runtime.run_frame(bounds).unwrap();
    assert!(
        !runtime.frame_requested(),
        "a parked future must not cause redraws"
    );
    let mut context = Context::from_waker(Waker::noop());
    assert!(task.as_mut().poll(&mut context).is_pending());

    let before = host.0.load(Ordering::SeqCst);
    let waker = parked.borrow_mut().take().unwrap();
    let late_waker = waker.clone();
    std::thread::spawn(move || {
        ready.store(true, Ordering::SeqCst);
        waker.wake_by_ref();
        waker.wake_by_ref();
    })
    .join()
    .unwrap();

    assert_eq!(host.0.load(Ordering::SeqCst), before + 1);
    assert!(runtime.frame_requested());
    runtime.run_frame(bounds).unwrap();
    assert_eq!(task.as_mut().poll(&mut context), Poll::Ready(Ok(7)));
    late_waker.wake();
    assert!(!runtime.frame_requested());
    assert_eq!(host.0.load(Ordering::SeqCst), before + 1);
}

#[test]
fn dropping_the_result_does_not_stop_work_and_a_poll_panic_is_isolated() {
    let finished = Rc::new(Cell::new(false));
    let callback = {
        let finished = finished.clone();
        move || async move {
            yield_once().await;
            finished.set(true);
        }
    };
    drop(schedule_async_task(callback, Priority::IDLE));

    let mut broken = std::pin::pin!(schedule_async_task(
        || async {
            yield_once().await;
            panic!("broken future");
        },
        Priority::ANIMATION,
    ));
    let mut context = Context::from_waker(Waker::noop());
    assert!(run_one_scheduled_task());
    assert!(broken.as_mut().poll(&mut context).is_pending());
    assert!(run_one_scheduled_task());
    let Poll::Ready(Err(error)) = broken.as_mut().poll(&mut context) else {
        panic!("the async panic must be reported as an error");
    };
    assert_eq!(error.to_string(), "broken future");
    assert!(run_one_scheduled_task());
    assert!(!finished.get());
    assert!(!run_one_scheduled_task());
    assert!(finished.get());
}
