//! The frame scheduler application code reaches without a runtime handle:
//! Flutter's `Ticker` and the `SchedulerBinding` frame callbacks.
//!
//! Everything here belongs to the UI thread. The runtime drives it once per
//! frame: [`begin_frame`] runs the transient frame callbacks and the tickers
//! before the build, and [`end_frame`] runs the post-frame callbacks after
//! paint. While a ticker runs or a frame callback waits,
//! [`has_scheduled_frame`] keeps the runtime producing frames.

use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

type FrameCallback = Box<dyn FnOnce(Instant)>;

#[derive(Default)]
struct Scheduler {
    tickers: Vec<Weak<TickerState>>,
    frame_callbacks: Vec<(u64, FrameCallback)>,
    post_frame_callbacks: Vec<FrameCallback>,
    persistent_callbacks: Vec<Rc<dyn Fn(Instant)>>,
    /// The timestamp of the frame being produced, from `begin_frame` until
    /// the post-frame callbacks (Flutter `currentFrameTimeStamp`).
    frame_time: Option<Instant>,
    next_callback: u64,
    frame_requested: bool,
}

thread_local! {
    static SCHEDULER: RefCell<Scheduler> = RefCell::new(Scheduler::default());
}

/// Identifies a callback registered with [`schedule_frame_callback`], for
/// [`cancel_frame_callback`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameCallbackId(u64);

/// Flutter `SchedulerBinding.scheduleFrameCallback`: runs `callback` with the
/// frame's timestamp at the start of the next frame, before the build, and
/// asks for that frame.
pub fn schedule_frame_callback(callback: impl FnOnce(Instant) + 'static) -> FrameCallbackId {
    SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        let id = scheduler.next_callback;
        scheduler.next_callback += 1;
        scheduler.frame_callbacks.push((id, Box::new(callback)));
        FrameCallbackId(id)
    })
}

/// Flutter `SchedulerBinding.cancelFrameCallbackWithId`. Returns whether the
/// callback was still waiting.
pub fn cancel_frame_callback(id: FrameCallbackId) -> bool {
    SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        let before = scheduler.frame_callbacks.len();
        scheduler
            .frame_callbacks
            .retain(|(callback, _)| *callback != id.0);
        before != scheduler.frame_callbacks.len()
    })
}

/// Flutter `SchedulerBinding.addPostFrameCallback`: runs `callback` once,
/// after the next frame has been laid out and painted. Like Flutter, this
/// does not ask for a frame; call [`schedule_frame`] when one is needed.
pub fn add_post_frame_callback(callback: impl FnOnce(Instant) + 'static) {
    SCHEDULER.with(|scheduler| {
        scheduler
            .borrow_mut()
            .post_frame_callbacks
            .push(Box::new(callback));
    });
}

/// Flutter `SchedulerBinding.addPersistentFrameCallback`: runs `callback`
/// after every frame for the rest of the thread's life. It does not ask for
/// frames.
pub fn add_persistent_frame_callback(callback: impl Fn(Instant) + 'static) {
    SCHEDULER.with(|scheduler| {
        scheduler
            .borrow_mut()
            .persistent_callbacks
            .push(Rc::new(callback));
    });
}

/// Flutter `SchedulerBinding.scheduleFrame`: asks for one more frame.
pub fn schedule_frame() {
    SCHEDULER.with(|scheduler| scheduler.borrow_mut().frame_requested = true);
}

/// Flutter `SchedulerBinding.currentFrameTimeStamp`: the timestamp of the
/// frame being produced, while one is (before its post-frame callbacks).
#[must_use]
pub fn current_frame_time() -> Option<Instant> {
    SCHEDULER.with(|scheduler| scheduler.borrow().frame_time)
}

/// Whether a frame is wanted: one was asked for, a frame callback waits, or
/// an unmuted ticker runs. The runtime keeps producing frames while true.
#[must_use]
pub fn has_scheduled_frame() -> bool {
    SCHEDULER.with(|scheduler| {
        let scheduler = scheduler.borrow();
        scheduler.frame_requested
            || !scheduler.frame_callbacks.is_empty()
            || scheduler
                .tickers
                .iter()
                .filter_map(Weak::upgrade)
                .any(|ticker| ticker.is_ticking())
    })
}

/// Flutter `handleBeginFrame`: runs the frame callbacks registered before
/// this frame, then every running ticker, with the frame's timestamp.
/// Runtime-internal; application code never calls it.
#[doc(hidden)]
pub fn begin_frame(now: Instant) {
    let callbacks = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.frame_requested = false;
        scheduler.frame_time = Some(now);
        std::mem::take(&mut scheduler.frame_callbacks)
    });
    for (_, callback) in callbacks {
        callback(now);
    }
    let tickers: Vec<Rc<TickerState>> = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.tickers.retain(|ticker| ticker.strong_count() > 0);
        scheduler
            .tickers
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|ticker| ticker.active.get())
            .collect()
    });
    for ticker in tickers {
        ticker.tick(now);
    }
}

/// Flutter `handleDrawFrame`'s post-frame phase: runs the persistent
/// callbacks and then the post-frame callbacks registered before this point.
/// Runtime-internal.
#[doc(hidden)]
pub fn end_frame(now: Instant) {
    let (persistent, post) = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.frame_time = None;
        (
            scheduler.persistent_callbacks.clone(),
            std::mem::take(&mut scheduler.post_frame_callbacks),
        )
    });
    for callback in persistent {
        callback(now);
    }
    for callback in post {
        callback(now);
    }
}

struct TickerState {
    on_tick: RefCell<Box<dyn FnMut(Instant, Duration)>>,
    active: Cell<bool>,
    muted: Cell<bool>,
    /// The frame the current run started on; unset until its first frame.
    started: Cell<Option<Instant>>,
    /// The future of the current run.
    future: RefCell<Option<TickerFuture>>,
}

impl TickerState {
    fn is_ticking(&self) -> bool {
        self.active.get() && !self.muted.get()
    }

    fn tick(&self, now: Instant) {
        let started = match self.started.get() {
            Some(started) => started,
            None => {
                self.started.set(Some(now));
                now
            }
        };
        if self.muted.get() {
            return;
        }
        let elapsed = now.saturating_duration_since(started);
        // A callback that restarts or stops its own ticker sees the new
        // state on the next frame; a re-entrant tick is skipped.
        if let Ok(mut on_tick) = self.on_tick.try_borrow_mut() {
            on_tick(now, elapsed);
        }
    }
}

/// Flutter `Ticker`: calls its callback once per frame while active, with the
/// time elapsed since the first frame after [`start`](Self::start).
///
/// A muted ticker keeps its clock but skips its callback and asks for no
/// frames, as `TickerMode` mutes the tickers of offstage subtrees. Dropping
/// the ticker stops it.
pub struct Ticker {
    state: Rc<TickerState>,
}

impl std::fmt::Debug for Ticker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Ticker")
            .field("active", &self.state.active.get())
            .field("muted", &self.state.muted.get())
            .finish()
    }
}

impl Ticker {
    /// Creates a stopped ticker that calls `on_tick` with the elapsed time.
    #[must_use]
    pub fn new(mut on_tick: impl FnMut(Duration) + 'static) -> Self {
        Self::with_frame_time(move |_, elapsed| on_tick(elapsed))
    }

    /// A ticker whose callback also receives the frame's timestamp.
    #[must_use]
    pub fn with_frame_time(on_tick: impl FnMut(Instant, Duration) + 'static) -> Self {
        let state = Rc::new(TickerState {
            on_tick: RefCell::new(Box::new(on_tick)),
            active: Cell::new(false),
            muted: Cell::new(false),
            started: Cell::new(None),
            future: RefCell::new(None),
        });
        SCHEDULER.with(|scheduler| {
            scheduler.borrow_mut().tickers.push(Rc::downgrade(&state));
        });
        Self { state }
    }

    /// Starts ticking. Like Flutter's `Ticker.start`, a ticker started while
    /// a frame is being produced (by a frame callback, a build or layout)
    /// times itself from that frame; otherwise from the next frame. Starting a
    /// running ticker does nothing; [`stop`](Self::stop) it first to restart
    /// the clock. Returns the run's [`TickerFuture`].
    pub fn start(&self) -> TickerFuture {
        if !self.state.active.replace(true) {
            self.state.started.set(current_frame_time());
            *self.state.future.borrow_mut() = Some(TickerFuture::pending());
        }
        self.state
            .future
            .borrow()
            .clone()
            .unwrap_or_else(TickerFuture::complete)
    }

    /// Flutter `stop()`: stops ticking and completes the run's future. The
    /// callback is not called again until a new start.
    pub fn stop(&self) {
        self.stop_with(true);
    }

    /// Flutter `stop(canceled: true)`: stops ticking and cancels the run's
    /// future.
    pub fn cancel(&self) {
        self.stop_with(false);
    }

    fn stop_with(&self, completed: bool) {
        self.state.active.set(false);
        self.state.started.set(None);
        let future = self.state.future.borrow_mut().take();
        if let Some(future) = future {
            future.resolve(completed);
        }
    }

    /// Flutter `Ticker.isActive`: started and not stopped, muted or not.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state.active.get()
    }

    /// Flutter `Ticker.isTicking`: active and not muted.
    #[must_use]
    pub fn is_ticking(&self) -> bool {
        self.state.is_ticking()
    }

    #[must_use]
    pub fn muted(&self) -> bool {
        self.state.muted.get()
    }

    /// Flutter `Ticker.muted`: a muted ticker keeps time but skips its
    /// callback.
    pub fn set_muted(&self, muted: bool) {
        self.state.muted.set(muted);
    }

    /// Flutter `Ticker.absorbTicker`: takes over `other`'s run, keeping its
    /// start time, and stops `other`.
    pub fn absorb_ticker(&self, other: &Ticker) {
        if other.is_active() {
            self.state.active.set(true);
            self.state.started.set(other.state.started.get());
            *self.state.future.borrow_mut() = other.state.future.borrow_mut().take();
        }
        other.stop();
    }
}

impl Drop for Ticker {
    /// Flutter `dispose`: a ticker dropped mid-run cancels its future.
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Pending,
    Complete,
    Canceled,
}

type Completion = Box<dyn FnOnce(bool)>;

struct FutureState {
    outcome: Outcome,
    /// Called with whether the run completed (`true`) or was canceled.
    callbacks: Vec<Completion>,
    wakers: Vec<std::task::Waker>,
}

/// Flutter `TickerFuture`: what [`Ticker::start`] and the animation
/// controller's runs (`forward`, `reverse`, `animate_to`, ...) return.
///
/// It completes when the run reaches its end. A run that is stopped or
/// replaced first is canceled: awaiting the future itself then never
/// finishes, as in Flutter, while [`or_cancel`](Self::or_cancel) reports the
/// cancellation and [`when_complete_or_cancel`](Self::when_complete_or_cancel)
/// runs either way.
#[derive(Clone)]
pub struct TickerFuture {
    state: Rc<RefCell<FutureState>>,
}

impl std::fmt::Debug for TickerFuture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("TickerFuture")
            .field(&self.state.borrow().outcome)
            .finish()
    }
}

/// The error of [`TickerFuture::or_cancel`] when the run was canceled:
/// Flutter `TickerCanceled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickerCanceled;

impl std::fmt::Display for TickerCanceled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("This ticker was canceled")
    }
}

impl std::error::Error for TickerCanceled {}

impl TickerFuture {
    fn with_outcome(outcome: Outcome) -> Self {
        Self {
            state: Rc::new(RefCell::new(FutureState {
                outcome,
                callbacks: Vec::new(),
                wakers: Vec::new(),
            })),
        }
    }

    pub(crate) fn pending() -> Self {
        Self::with_outcome(Outcome::Pending)
    }

    /// Flutter `TickerFuture.complete()`: an already completed future, for
    /// runs that finish at once.
    #[must_use]
    pub fn complete() -> Self {
        Self::with_outcome(Outcome::Complete)
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state.borrow().outcome == Outcome::Complete
    }

    #[must_use]
    pub fn is_canceled(&self) -> bool {
        self.state.borrow().outcome == Outcome::Canceled
    }

    /// Whether the run is still going.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.state.borrow().outcome == Outcome::Pending
    }

    /// Runs `callback` when the run completes; never if it is canceled
    /// (Flutter's `then` on the future).
    pub fn when_complete(&self, callback: impl FnOnce() + 'static) {
        self.on_outcome(move |completed| {
            if completed {
                callback();
            }
        });
    }

    /// Flutter `whenCompleteOrCancel`: runs `callback` once the run
    /// completes or is canceled.
    pub fn when_complete_or_cancel(&self, callback: impl FnOnce() + 'static) {
        self.on_outcome(move |_| callback());
    }

    fn on_outcome(&self, callback: impl FnOnce(bool) + 'static) {
        let outcome = self.state.borrow().outcome;
        match outcome {
            Outcome::Pending => self.state.borrow_mut().callbacks.push(Box::new(callback)),
            Outcome::Complete => callback(true),
            Outcome::Canceled => callback(false),
        }
    }

    /// Flutter `orCancel`: a future that resolves `Ok` when the run completes
    /// and `Err(TickerCanceled)` when it is canceled.
    #[must_use]
    pub fn or_cancel(&self) -> TickerOrCancel {
        TickerOrCancel(self.clone())
    }

    pub(crate) fn resolve(&self, completed: bool) {
        let (callbacks, wakers) = {
            let mut state = self.state.borrow_mut();
            if state.outcome != Outcome::Pending {
                return;
            }
            state.outcome = if completed {
                Outcome::Complete
            } else {
                Outcome::Canceled
            };
            (
                std::mem::take(&mut state.callbacks),
                std::mem::take(&mut state.wakers),
            )
        };
        for callback in callbacks {
            callback(completed);
        }
        for waker in wakers {
            waker.wake();
        }
    }

    fn poll_outcome(&self, context: &mut std::task::Context<'_>) -> Outcome {
        let mut state = self.state.borrow_mut();
        if state.outcome == Outcome::Pending
            && !state
                .wakers
                .iter()
                .any(|waker| waker.will_wake(context.waker()))
        {
            state.wakers.push(context.waker().clone());
        }
        state.outcome
    }
}

impl std::future::Future for TickerFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        match self.poll_outcome(context) {
            Outcome::Complete => std::task::Poll::Ready(()),
            // A canceled run never completes, as in Flutter.
            Outcome::Pending | Outcome::Canceled => std::task::Poll::Pending,
        }
    }
}

/// The future of [`TickerFuture::or_cancel`].
#[derive(Clone, Debug)]
pub struct TickerOrCancel(TickerFuture);

impl std::future::Future for TickerOrCancel {
    type Output = Result<(), TickerCanceled>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.0.poll_outcome(context) {
            Outcome::Complete => std::task::Poll::Ready(Ok(())),
            Outcome::Canceled => std::task::Poll::Ready(Err(TickerCanceled)),
            Outcome::Pending => std::task::Poll::Pending,
        }
    }
}
