use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{
    Priority, add_post_frame_callback, current_frame_time, current_system_frame_time,
    schedule_frame, schedule_frame_callback, schedule_frame_microtask,
    scheduler::{
        has_ready_scheduled_task, has_scheduled_frame, run_one_scheduled_task, schedule_task,
    },
};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{Builder, Widget, internal::TreeError};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100., 100.))
}

fn runtime() -> Runtime {
    Runtime::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).unwrap()
}

#[test]
fn warm_up_runs_microtasks_between_begin_and_actual_draw() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(
        Builder::new({
            let order = order.clone();
            move |_| {
                assert_eq!(*order.borrow(), ["begin", "microtask", "nested"]);
                order.borrow_mut().push("build");
                Widget::box_(Size::new(100., 100.), Color::BLACK)
            }
        })
        .into(),
    )
    .unwrap();
    schedule_frame_callback({
        let order = order.clone();
        move |_| {
            assert!(order.borrow().is_empty());
            order.borrow_mut().push("begin");
            schedule_frame_microtask(move || {
                order.borrow_mut().push("microtask");
                schedule_frame_microtask(move || order.borrow_mut().push("nested"));
            });
        }
    });
    add_post_frame_callback({
        let order = order.clone();
        move |_| order.borrow_mut().push("post")
    });
    assert!(runtime.schedule_warm_up_frame());
    assert!(runtime.draw_warm_up_frame(constraints()).unwrap().is_none());
    assert!(runtime.begin_warm_up_frame_at(Instant::now()));
    assert_eq!(*order.borrow(), ["begin"]);
    assert!(runtime.draw_warm_up_frame(constraints()).unwrap().is_some());
    assert_eq!(
        *order.borrow(),
        ["begin", "microtask", "nested", "build", "post"]
    );
    assert_eq!(current_frame_time(), None);
}

#[test]
fn repeated_requests_produce_one_begin_draw_pair_and_keep_tasks_locked() {
    let progress = Rc::new(Cell::new(0));
    let mut runtime = runtime();
    assert!(runtime.schedule_warm_up_frame());
    assert!(!runtime.schedule_warm_up_frame());
    let task = schedule_task(
        {
            let progress = progress.clone();
            move || progress.set(progress.get() + 10)
        },
        Priority::TOUCH,
    );
    let _task = task;
    assert!(!has_ready_scheduled_task());
    assert!(!run_one_scheduled_task());
    assert_eq!(progress.get(), 0);
    schedule_frame_callback({
        let progress = progress.clone();
        move |_| progress.set(progress.get() + 1)
    });
    add_post_frame_callback({
        let progress = progress.clone();
        move |_| progress.set(progress.get() + 2)
    });
    assert!(runtime.begin_warm_up_frame_at(Instant::now()));
    assert!(!runtime.begin_warm_up_frame_at(Instant::now()));
    assert!(!run_one_scheduled_task());
    assert_eq!(progress.get(), 1);
    assert!(runtime.draw_warm_up_frame(constraints()).unwrap().is_some());
    assert!(runtime.draw_warm_up_frame(constraints()).unwrap().is_none());
    assert_eq!(progress.get(), 3);
    assert!(has_ready_scheduled_task());
    let _ = run_one_scheduled_task();
    assert_eq!(progress.get(), 13);
}

#[test]
fn automatic_draw_preserves_native_request_and_resets_clock_epoch() {
    let mut runtime = runtime();
    let start = Instant::now();
    runtime.run_frame_at(constraints(), start).unwrap();
    let elapsed = Rc::new(RefCell::new(Vec::new()));
    let record = || {
        let elapsed = elapsed.clone();
        schedule_frame_callback(move |now| elapsed.borrow_mut().push(now));
    };
    schedule_frame();
    record();
    assert!(runtime.schedule_warm_up_frame());
    runtime
        .run_frame_at(constraints(), start + Duration::from_secs(5))
        .unwrap();
    assert!(
        has_scheduled_frame(),
        "already queued native frame survives the warm-up"
    );
    assert_eq!(current_system_frame_time(), Some(start));
    record();
    runtime
        .run_frame_at(constraints(), start + Duration::from_secs(10))
        .unwrap();
    record();
    runtime
        .run_frame_at(constraints(), start + Duration::from_millis(10016))
        .unwrap();
    assert_eq!(
        *elapsed.borrow(),
        [start, start, start + Duration::from_millis(16)]
    );
}

#[test]
fn an_abandoned_warm_up_releases_the_thread_for_other_windows() {
    let mut first = runtime();
    let mut second = runtime();
    assert!(first.schedule_warm_up_frame());
    assert!(!second.schedule_warm_up_frame());
    assert!(first.begin_warm_up_frame_at(Instant::now()));
    assert!(matches!(
        second.run_frame(constraints()),
        Err(TreeError::WarmUpFrameInProgress)
    ));
    drop(first);
    assert_eq!(current_frame_time(), None);
    second.run_frame(constraints()).unwrap();
    assert!(second.schedule_warm_up_frame());
    second.run_frame(constraints()).unwrap();
}

#[test]
fn a_microtask_registered_after_the_checkpoint_requests_a_following_frame() {
    let seen = Rc::new(Cell::new(false));
    let mut runtime = runtime();
    add_post_frame_callback({
        let seen = seen.clone();
        move |_| schedule_frame_microtask(move || seen.set(true))
    });
    runtime.run_frame(constraints()).unwrap();
    assert!(!seen.get());
    assert!(runtime.frame_requested());
    runtime.run_frame(constraints()).unwrap();
    assert!(seen.get());
}
