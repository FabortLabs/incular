use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_animation::Ticker;
use incular_animation::scheduler::{
    Priority, has_ready_scheduled_task, reset_scheduling_strategy, run_one_scheduled_task,
    schedule_task, set_scheduling_strategy,
};
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::Widget;

#[test]
fn tasks_follow_flutter_priority_thresholds() {
    let threshold = Rc::new(Cell::new(100));
    set_scheduling_strategy({
        let threshold = threshold.clone();
        move |priority| priority.value() >= threshold.get()
    });
    let ran = Rc::new(RefCell::new(Vec::new()));
    let add = |value| {
        let ran = ran.clone();
        drop(schedule_task(
            move || ran.borrow_mut().push(value),
            Priority::IDLE + value,
        ));
    };
    for value in [2, 23, 23, 11, 0, 80, 3] {
        add(value);
    }
    let stage = |allowed, turns, expected: &[i64]| {
        threshold.set(allowed);
        for _ in 0..turns {
            assert!(run_one_scheduled_task());
        }
        assert_eq!(&*ran.borrow(), expected);
        ran.borrow_mut().clear();
    };
    stage(100, 3, &[]);
    stage(50, 3, &[80]);
    stage(20, 3, &[23, 23]);
    for value in [99, 19, 5, 97] {
        add(value);
    }
    stage(20, 3, &[99, 97]);
    stage(10, 3, &[19, 11]);
    stage(1, 4, &[5, 3, 2]);
    threshold.set(0);
    assert!(!run_one_scheduled_task());
    assert_eq!(&*ran.borrow(), &[0]);
    reset_scheduling_strategy();
}

#[test]
fn application_frames_service_one_eligible_task_before_animation() {
    let size = Size::new(100.0, 100.0);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!runtime.frame_requested());

    let ran = Rc::new(RefCell::new(Vec::new()));
    let ticker = Ticker::new({
        let ran = ran.clone();
        move |_| ran.borrow_mut().push("tick")
    });
    ticker.start();
    for (priority, label) in [(Priority::IDLE, "idle"), (Priority::ANIMATION, "animation")] {
        let ran = ran.clone();
        drop(schedule_task(
            move || ran.borrow_mut().push(label),
            priority,
        ));
    }
    runtime.run_frame(bounds).unwrap();
    assert_eq!(&*ran.borrow(), &["animation", "tick"]);
    assert!(!has_ready_scheduled_task());
    ticker.stop();
    assert!(runtime.frame_requested());
    runtime.run_frame(bounds).unwrap();
    assert_eq!(&*ran.borrow(), &["animation", "tick", "idle"]);
    assert!(!runtime.frame_requested());
}

#[test]
fn completion_preserves_sum_and_reports_a_panicking_task_without_blocking_others() {
    let mut ok = std::pin::pin!(schedule_task(|| 3 + 5, Priority::TOUCH));
    let mut broken = std::pin::pin!(schedule_task(|| panic!("broken task"), Priority::IDLE));
    let mut context = Context::from_waker(Waker::noop());
    assert!(ok.as_mut().poll(&mut context).is_pending());
    assert!(broken.as_mut().poll(&mut context).is_pending());
    assert!(run_one_scheduled_task());
    assert_eq!(ok.as_mut().poll(&mut context), Poll::Ready(Ok(8)));
    assert!(!run_one_scheduled_task());
    let Poll::Ready(Err(error)) = broken.as_mut().poll(&mut context) else {
        panic!("task panic should become a result");
    };
    assert_eq!(error.to_string(), "broken task");
}
