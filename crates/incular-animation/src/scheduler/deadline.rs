use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

type TimedCallback = Box<dyn FnOnce(Instant)>;

#[derive(Default)]
struct Deadlines {
    next_id: u64,
    queue: BTreeMap<(Instant, u64), Weak<Scheduled>>,
}

thread_local! {
    static DEADLINES: RefCell<Deadlines> = RefCell::new(Deadlines::default());
}

struct Scheduled {
    wake: (Instant, u64),
    callback: RefCell<Option<TimedCallback>>,
    queued: Cell<bool>,
}

/// Owns a one-shot callback scheduled against the raw monotonic clock.
/// Dropping the handle withdraws its deadline and cancels its callback.
#[must_use = "keep the handle until the deadline or drop it to cancel"]
pub struct FrameDeadlineHandle(Rc<Scheduled>);

impl Drop for FrameDeadlineHandle {
    fn drop(&mut self) {
        if self.0.queued.replace(false) {
            // Thread-local teardown can drop captures that own more handles.
            let _ =
                DEADLINES.try_with(|deadlines| deadlines.borrow_mut().queue.remove(&self.0.wake));
        }
        let callback = self.0.callback.borrow_mut().take();
        drop(callback);
    }
}

/// Runs at the start of an app frame at or after `raw_deadline`.
/// The callback receives the normal, dilation-adjusted frame timestamp.
/// Registering a future deadline does not request continuous redraws.
pub fn schedule_frame_deadline(
    raw_deadline: Instant,
    callback: impl FnOnce(Instant) + 'static,
) -> FrameDeadlineHandle {
    DEADLINES.with(|registry| {
        let mut state = registry.borrow_mut();
        let wake = (raw_deadline, state.next_id);
        state.next_id = state
            .next_id
            .checked_add(1)
            .expect("frame deadline id exhausted");
        let scheduled = Rc::new(Scheduled {
            wake,
            callback: RefCell::new(Some(Box::new(callback))),
            queued: Cell::new(true),
        });
        state.queue.insert(wake, Rc::downgrade(&scheduled));
        FrameDeadlineHandle(scheduled)
    })
}

/// The earliest raw deadline the UI-thread host can sleep until. Pausing
/// ordinary frames preserves registered deadlines without waking for them.
#[must_use]
pub fn next_scheduled_frame_deadline() -> Option<Instant> {
    if !super::frames_enabled() {
        return None;
    }
    DEADLINES.with(|registry| {
        registry
            .borrow()
            .queue
            .first_key_value()
            .map(|(key, _)| key.0)
    })
}

pub(super) fn enqueue_due(raw: Instant) {
    // Take a snapshot so a due callback registering a new deadline does
    // not re-enter the frame currently being produced.
    let promoted = DEADLINES.with(|registry| {
        let mut state = registry.borrow_mut();
        let mut promoted = Vec::new();
        while state
            .queue
            .first_key_value()
            .is_some_and(|(key, _)| key.0 <= raw)
        {
            let (_, weak) = state.queue.pop_first().expect("peeked a due deadline");
            if let Some(scheduled) = weak.upgrade() {
                scheduled.queued.set(false);
                promoted.push(scheduled);
            }
        }
        promoted
    });
    for scheduled in promoted {
        super::schedule_frame_callback_with_request(
            move |timestamp| {
                let callback = scheduled.callback.borrow_mut().take();
                if let Some(callback) = callback {
                    callback(timestamp);
                }
            },
            false,
        );
    }
}
