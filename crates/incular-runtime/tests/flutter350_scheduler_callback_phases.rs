//! Flutter 3.50 SchedulerBinding callback ordering and timestamp lifetime.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{
    add_persistent_frame_callback, add_post_frame_callback, current_frame_time,
    schedule_frame_callback,
};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::Widget;

fn runtime() -> (Runtime, Constraints, Instant) {
    let size = Size::new(100., 100.);
    let bounds = Constraints::tight(size);
    let mut runtime = Runtime::new(Widget::box_(size, Color::WHITE)).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(bounds, base).unwrap();
    (runtime, bounds, base)
}

#[test]
fn persistent_registered_during_persistent_waits_for_next_frame() {
    // Exact boolean assertions from Flutter test/scheduler/binding_test.dart:44.
    let (mut runtime, bounds, base) = runtime();
    let called_back = Rc::new(Cell::new(false));
    add_persistent_frame_callback({
        let called_back = Rc::clone(&called_back);
        move |_| {
            if !called_back.get() {
                let called_back = Rc::clone(&called_back);
                add_persistent_frame_callback(move |_| called_back.set(true));
            }
        }
    });

    runtime
        .run_frame_at(bounds, base + Duration::from_millis(16))
        .unwrap();
    assert!(!called_back.get());
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(32))
        .unwrap();
    assert!(called_back.get());
}

#[test]
fn callback_registration_respects_active_phase_snapshots() {
    // Flutter scheduler/binding.dart:1259–1272,1345–1361: transient and post
    // snapshot their own queues independently; persistent precedes post.
    let (mut runtime, bounds, base) = runtime();
    let events = Rc::new(RefCell::new(Vec::new()));
    schedule_frame_callback({
        let events = Rc::clone(&events);
        move |_| {
            events.borrow_mut().push("transient");
            schedule_frame_callback(move |_| events.borrow_mut().push("next-transient"));
        }
    });
    add_persistent_frame_callback({
        let events = Rc::clone(&events);
        let first = Cell::new(true);
        move |_| {
            events.borrow_mut().push("persistent");
            if first.replace(false) {
                let events = Rc::clone(&events);
                add_post_frame_callback(move |_| events.borrow_mut().push("post-from-persistent"));
            }
        }
    });
    add_post_frame_callback({
        let events = Rc::clone(&events);
        move |_| {
            events.borrow_mut().push("post");
            add_post_frame_callback(move |_| events.borrow_mut().push("next-post"));
        }
    });

    runtime
        .run_frame_at(bounds, base + Duration::from_millis(16))
        .unwrap();
    let first_frame = std::mem::take(&mut *events.borrow_mut());
    runtime
        .run_frame_at(bounds, base + Duration::from_millis(32))
        .unwrap();

    assert_eq!(
        [first_frame.as_slice(), events.borrow().as_slice()],
        [
            &["transient", "persistent", "post", "post-from-persistent"][..],
            &["next-transient", "persistent", "next-post"][..],
        ],
    );
}

#[test]
fn current_timestamp_is_visible_in_each_callback_phase() {
    // Flutter scheduler/binding.dart:1348,1360,1377 clears the current
    // frame timestamp only after both callback phases have finished.
    let (mut runtime, bounds, base) = runtime();
    let visible = Rc::new(RefCell::new(Vec::new()));
    let record = |phase| {
        let visible = Rc::clone(&visible);
        move |time| {
            visible
                .borrow_mut()
                .push((phase, current_frame_time() == Some(time)))
        }
    };
    schedule_frame_callback(record("transient"));
    add_persistent_frame_callback(record("persistent"));
    add_post_frame_callback(record("post"));

    runtime
        .run_frame_at(bounds, base + Duration::from_millis(16))
        .unwrap();

    assert_eq!(current_frame_time(), None, "the frame is over");
    assert_eq!(
        visible.borrow().as_slice(),
        &[("transient", true), ("persistent", true), ("post", true)],
    );
}
