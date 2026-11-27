use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::scheduler::{
    FrameAvailability, begin_frame, end_frame, has_scheduled_frame, next_scheduled_frame_deadline,
    schedule_frame_callback, schedule_frame_deadline,
};

fn frame(at: Instant) {
    begin_frame(at);
    end_frame(at);
}

#[test]
fn a_future_deadline_does_not_request_frames_until_its_time() {
    let now = Instant::now();
    let due = now + Duration::from_millis(200);
    let timestamp = Rc::new(Cell::new(None));
    let output = timestamp.clone();
    let _deadline = schedule_frame_deadline(due, move |at| output.set(Some(at)));
    assert!(!has_scheduled_frame());
    assert_eq!(next_scheduled_frame_deadline(), Some(due));
    frame(due - Duration::from_millis(1));
    assert_eq!(timestamp.get(), None);
    frame(due);
    assert_eq!(timestamp.get(), Some(due));
    assert_eq!(next_scheduled_frame_deadline(), None);
}

#[test]
fn deadlines_are_cancelable_before_and_during_the_due_frame() {
    let now = Instant::now();
    let canceled = schedule_frame_deadline(now, |_| panic!("canceled timer fired"));
    drop(canceled);
    assert_eq!(next_scheduled_frame_deadline(), None);

    let due = Rc::new(RefCell::new(Some(schedule_frame_deadline(now, |_| {
        panic!("timer canceled in the transient queue still fired")
    }))));
    let slot = due.clone();
    schedule_frame_callback(move |_| drop(slot.borrow_mut().take()));
    frame(now);
    assert!(due.borrow().is_none());
    assert_eq!(next_scheduled_frame_deadline(), None);
}

#[test]
fn a_paused_scheduler_withholds_but_preserves_the_deadline() {
    let now = Instant::now();
    let mut owner = FrameAvailability::default();
    let fired = Rc::new(Cell::new(false));
    let output = fired.clone();
    let _pending = schedule_frame_deadline(now, move |_| output.set(true));
    owner.set_enabled(false);
    assert_eq!(next_scheduled_frame_deadline(), None);
    frame(now);
    assert!(!fired.get());
    owner.set_enabled(true);
    assert_eq!(next_scheduled_frame_deadline(), Some(now));
    frame(now);
    assert!(fired.get());
}
