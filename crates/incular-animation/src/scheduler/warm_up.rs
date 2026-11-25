//! Two-phase warm-up frame reservation and the begin/draw checkpoint.

use super::{Epoch, Instant, PhantomData, Rc, RefCell, SCHEDULER, VecDeque, begin_frame_with_mode};

type Microtask = Box<dyn FnOnce()>;

thread_local! {
    static MICROTASKS: RefCell<VecDeque<Microtask>> = const { RefCell::new(VecDeque::new()) };
}

/// Schedules a short callback for the checkpoint between transient frame
/// callbacks and draw. Microtasks added by microtasks are drained in order
/// before draw. This queue is independent from prioritized UI tasks.
pub fn schedule_frame_microtask(callback: impl FnOnce() + 'static) {
    MICROTASKS.with(|tasks| tasks.borrow_mut().push_back(Box::new(callback)));
    SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        if scheduler.microtask_checkpoint_passed && scheduler.frame_time.is_some()
            || scheduler.frame_time.is_none() && !scheduler.warm_up_pending
        {
            scheduler.frame_requested = true;
        }
    });
}

#[doc(hidden)]
pub fn flush_frame_microtasks() {
    while let Some(callback) = MICROTASKS.with(|tasks| tasks.borrow_mut().pop_front()) {
        callback();
    }
    SCHEDULER.with(|scheduler| scheduler.borrow_mut().microtask_checkpoint_passed = true);
}

#[doc(hidden)]
pub fn warm_up_frame_pending() -> bool {
    SCHEDULER.with(|scheduler| scheduler.borrow().warm_up_pending)
}

/// One UI-thread warm-up reservation. The runtime drives begin and draw;
/// abandoning the reservation unlocks ordinary tasks without a stale frame.
#[doc(hidden)]
pub struct WarmUpFrame {
    began: Option<Instant>,
    adjusted: Option<Instant>,
    requested: bool,
    forced: bool,
    _ui_thread: PhantomData<Rc<()>>,
}

impl WarmUpFrame {
    #[must_use]
    pub fn reserve() -> Option<Self> {
        let reservation = SCHEDULER.with(|scheduler| {
            let mut scheduler = scheduler.borrow_mut();
            if scheduler.warm_up_pending || scheduler.frame_time.is_some() {
                return None;
            }
            scheduler.warm_up_pending = true;
            Some(Self {
                began: None,
                adjusted: None,
                requested: scheduler.frame_requested || scheduler.queued_ticker_count != 0,
                forced: scheduler.forced_frame_requested,
                _ui_thread: PhantomData,
            })
        });
        if reservation.is_some() {
            crate::ui_tasks::set_warm_up_task_lock(true);
        }
        reservation
    }

    /// Starts the transient phase once, using the last real frame's raw
    /// timestamp rather than adding startup or hot-reload latency to it.
    pub fn begin(&mut self, host_now: Instant) -> bool {
        if self.began.is_some() {
            return false;
        }
        let raw = SCHEDULER.with(|scheduler| scheduler.borrow().last_raw.unwrap_or(host_now));
        self.began = Some(raw);
        begin_frame_with_mode(raw, true);
        self.adjusted = super::current_frame_time();
        true
    }

    #[must_use]
    pub fn began_at(&self) -> Option<Instant> {
        self.began
    }

    /// Resets the epoch after the draw and makes the next real frame begin
    /// at the warm-up callback time without a wall-clock jump.
    pub fn complete(self) {
        if let Some(adjusted) = self.adjusted {
            SCHEDULER.with(|scheduler| {
                scheduler.borrow_mut().epoch = Some(Epoch {
                    start: Some(adjusted),
                    first_raw: None,
                });
            });
        }
    }
}

impl Drop for WarmUpFrame {
    fn drop(&mut self) {
        // The UI-thread local may already be gone during thread shutdown.
        let unlocked = SCHEDULER.try_with(|scheduler| {
            let mut scheduler = scheduler.borrow_mut();
            if self.began.is_some() {
                scheduler.frame_time = None;
                scheduler.post_frame_active = false;
            }
            scheduler.warm_up_pending = false;
            scheduler.frame_requested |= self.requested;
            scheduler.forced_frame_requested |= self.forced;
        });
        if unlocked.is_ok() {
            crate::ui_tasks::set_warm_up_task_lock(false);
        }
    }
}
