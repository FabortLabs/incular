use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::scheduler::{
    FrameCallbackId, FrameDeadlineHandle, cancel_frame_callback, current_system_frame_time,
    schedule_frame_callback, schedule_frame_deadline, time_dilation,
};
use incular_core::time::Instant;

use super::{ImageFrameDemand, ImagePlayback, State};

enum ScheduledWake {
    Frame(FrameCallbackId),
    Deadline { _owned: FrameDeadlineHandle },
}

impl Drop for ScheduledWake {
    fn drop(&mut self) {
        if let Self::Frame(id) = self {
            cancel_frame_callback(*id);
        }
    }
}

struct PlaybackDriver {
    state: Weak<RefCell<State>>,
    request_frame: RefCell<Option<Weak<dyn Fn()>>>,
    wake: RefCell<Option<ScheduledWake>>,
}

impl PlaybackDriver {
    fn changed(self: &Rc<Self>) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        if state.borrow().demand().is_none() {
            let wake = self.wake.borrow_mut().take();
            drop(wake);
        } else if self.wake.borrow().is_none() {
            // First consult the app's frame timestamp even if the duration
            // appears still pending; time dilation may just have changed.
            let driver = Rc::downgrade(self);
            let id = schedule_frame_callback(move |at| {
                if let Some(driver) = driver.upgrade() {
                    driver.on_frame(at);
                }
            });
            *self.wake.borrow_mut() = Some(ScheduledWake::Frame(id));
        }
    }

    fn on_frame(self: &Rc<Self>, at: Instant) {
        let fired = self.wake.borrow_mut().take();
        drop(fired);
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let Some(request_frame) = self.request_frame.borrow().as_ref().and_then(Weak::upgrade)
        else {
            return;
        };
        let playback = ImagePlayback {
            state,
            request_frame,
        };
        if playback.advance(at) {
            return;
        }
        let Some(ImageFrameDemand::At(due)) = playback.frame_demand() else {
            return;
        };
        let remaining = due.saturating_duration_since(at);
        let Ok(wait) = Duration::try_from_secs_f64(remaining.as_secs_f64() * time_dilation())
        else {
            return;
        };
        let Some(wake_at) = current_system_frame_time().unwrap_or(at).checked_add(wait) else {
            return;
        };
        let driver = Rc::downgrade(self);
        let owned = schedule_frame_deadline(wake_at, move |at| {
            if let Some(driver) = driver.upgrade() {
                driver.on_frame(at);
            }
        });
        *self.wake.borrow_mut() = Some(ScheduledWake::Deadline { _owned: owned });
    }
}

impl ImagePlayback {
    /// Drives the decoder on Incular's app frame clock. A waiting frame
    /// uses one cancelable deadline rather than requesting each display
    /// refresh. Delay follows the same scheduler time dilation as tickers.
    #[must_use]
    pub fn scheduled() -> Self {
        let state = Rc::new(RefCell::new(State::default()));
        let driver = Rc::new(PlaybackDriver {
            state: Rc::downgrade(&state),
            request_frame: RefCell::new(None),
            wake: RefCell::new(None),
        });
        let request_frame: Rc<dyn Fn()> = {
            let driver = driver.clone();
            Rc::new(move || driver.changed())
        };
        *driver.request_frame.borrow_mut() = Some(Rc::downgrade(&request_frame));
        Self {
            state,
            request_frame,
        }
    }
}
