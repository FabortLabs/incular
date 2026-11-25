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
    collections::VecDeque,
    rc::{Rc, Weak},
    time::Duration,
};

type FrameCallback = Box<dyn FnOnce(Instant)>;

struct Scheduler {
    tickers: Vec<Weak<TickerState>>,
    /// Scratch buffer for [`begin_frame`], kept to reuse its allocation.
    ticking: Vec<Rc<TickerState>>,
    frame_callbacks: Vec<(u64, FrameCallback)>,
    /// Pending callbacks in each dispatch snapshot. The UI thread drops
    /// borrows while running callbacks, so they can cancel other members.
    active_frame_callbacks: Vec<VecDeque<(u64, FrameCallback)>>,
    post_frame_callbacks: Vec<FrameCallback>,
    persistent_callbacks: Vec<Rc<dyn Fn(Instant)>>,
    /// The timestamp remains valid through the last post-frame callback.
    frame_time: Option<Instant>,
    post_frame_active: bool,
    next_callback: u64,
    frame_requested: bool,
    /// Flutter `timeDilation`; 1 is real time.
    time_dilation: f64,
    /// The dilated time base since the dilation last changed. `None` until
    /// the first change: frame timestamps are the raw ones until then.
    epoch: Option<Epoch>,
    /// The raw timestamp of the last frame (Flutter `_lastRawTimeStamp`).
    last_raw: Option<Instant>,
}

/// Flutter `_epochStart` and `_firstRawTimeStampInEpoch`.
#[derive(Clone, Copy)]
struct Epoch {
    /// The dilated timestamp the epoch starts at. Unset when no frame
    /// preceded the change: the epoch then starts at its first raw frame.
    start: Option<Instant>,
    first_raw: Option<Instant>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            tickers: Vec::new(),
            ticking: Vec::new(),
            frame_callbacks: Vec::new(),
            active_frame_callbacks: Vec::new(),
            post_frame_callbacks: Vec::new(),
            persistent_callbacks: Vec::new(),
            frame_time: None,
            post_frame_active: false,
            next_callback: 0,
            frame_requested: false,
            time_dilation: 1.0,
            epoch: None,
            last_raw: None,
        }
    }
}

impl Scheduler {
    fn remove_frame_callback(&mut self, id: u64) -> Option<FrameCallback> {
        if let Some(index) = self
            .frame_callbacks
            .iter()
            .position(|(candidate, _)| *candidate == id)
        {
            return Some(self.frame_callbacks.remove(index).1);
        }
        for batch in self.active_frame_callbacks.iter_mut().rev() {
            if let Some(index) = batch.iter().position(|(candidate, _)| *candidate == id) {
                return batch.remove(index).map(|(_, callback)| callback);
            }
        }
        None
    }

    /// Flutter `_adjustForEpoch`: the dilated timestamp of a raw one. The
    /// first frame of an epoch lands on the epoch start, so the interval
    /// that spans a dilation change does not advance time.
    fn adjust(&mut self, raw: Instant) -> Instant {
        let dilation = self.time_dilation;
        let Some(epoch) = self.epoch.as_mut() else {
            return raw;
        };
        let first = *epoch.first_raw.get_or_insert(raw);
        let start = *epoch.start.get_or_insert(first);
        if raw >= first {
            let since = raw.duration_since(first).div_f64(dilation);
            start.checked_add(since).unwrap_or(start)
        } else {
            let before = first.duration_since(raw).div_f64(dilation);
            start.checked_sub(before).unwrap_or(start)
        }
    }
}

/// Flutter `timeDilation`: how much slower than real time frame timestamps
/// advance. 1 (the default) is real time; 2 runs every ticker, frame
/// callback and widget animation at half speed.
#[must_use]
pub fn time_dilation() -> f64 {
    SCHEDULER.with(|scheduler| scheduler.borrow().time_dilation)
}

/// A time dilation that is not finite and positive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvalidTimeDilation(pub f64);

impl std::fmt::Display for InvalidTimeDilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "time dilation must be finite and positive, got {}",
            self.0
        )
    }
}

impl std::error::Error for InvalidTimeDilation {}

/// Sets [`time_dilation`] for this thread's scheduler. As in Flutter, the
/// change starts a new epoch at the last frame's dilated timestamp, so time
/// does not jump; the next frame lands on that timestamp and later frames
/// advance by their raw interval divided by `dilation`.
///
/// # Panics
///
/// Panics if `dilation` is not finite and positive; use
/// [`try_set_time_dilation`] for untrusted input.
pub fn set_time_dilation(dilation: f64) {
    try_set_time_dilation(dilation).unwrap_or_else(|error| panic!("{error}"));
}

/// [`set_time_dilation`] returning an error instead of panicking.
///
/// # Errors
///
/// [`InvalidTimeDilation`] if `dilation` is not finite and positive; the
/// current dilation is left unchanged.
pub fn try_set_time_dilation(dilation: f64) -> Result<(), InvalidTimeDilation> {
    if !(dilation.is_finite() && dilation > 0.0) {
        return Err(InvalidTimeDilation(dilation));
    }
    SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        if scheduler.time_dilation == dilation {
            return;
        }
        // Flutter `resetEpoch`, with the old dilation.
        let start = scheduler.last_raw.map(|raw| scheduler.adjust(raw));
        scheduler.epoch = Some(Epoch {
            start,
            first_raw: None,
        });
        scheduler.time_dilation = dilation;
    });
    Ok(())
}

/// The dilated frame timestamp of the raw frame timestamp `raw` (Flutter
/// `_adjustForEpoch`). It is `raw` itself until [`set_time_dilation`] first
/// changes the dilation. Runtime-internal: the widget tree reads its
/// animation clock through it.
#[doc(hidden)]
#[must_use]
pub fn frame_timestamp(raw: Instant) -> Instant {
    SCHEDULER.with(|scheduler| scheduler.borrow_mut().adjust(raw))
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
    // Release the scheduler borrow before a captured value is dropped.
    let removed = SCHEDULER.with(|scheduler| scheduler.borrow_mut().remove_frame_callback(id.0));
    removed.is_some()
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
/// frame being produced, including its post-frame callbacks.
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
pub fn begin_frame(raw: Instant) {
    let (now, has_batch) = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        // Flutter `handleBeginFrame`: the raw timestamp, dilated.
        scheduler.last_raw = Some(raw);
        let now = scheduler.adjust(raw);
        scheduler.frame_requested = false;
        scheduler.frame_time = Some(now);
        scheduler.post_frame_active = false;
        let pending = std::mem::take(&mut scheduler.frame_callbacks);
        let has_batch = !pending.is_empty();
        if has_batch {
            scheduler.active_frame_callbacks.push(pending.into());
        }
        (now, has_batch)
    });
    if has_batch {
        let _batch = ActiveFrameCallbackBatch;
        while let Some((_, callback)) = SCHEDULER.with(|scheduler| {
            scheduler
                .borrow_mut()
                .active_frame_callbacks
                .last_mut()?
                .pop_front()
        }) {
            callback(now);
        }
    }
    let mut tickers =
        SCHEDULER.with(|scheduler| std::mem::take(&mut scheduler.borrow_mut().ticking));
    SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.tickers.retain(|ticker| ticker.strong_count() > 0);
        tickers.extend(
            scheduler
                .tickers
                .iter()
                .filter_map(Weak::upgrade)
                .filter(|ticker| ticker.active.get()),
        );
    });
    for ticker in tickers.drain(..) {
        ticker.tick(now);
    }
    SCHEDULER.with(|scheduler| scheduler.borrow_mut().ticking = tickers);
}

struct ActiveFrameCallbackBatch;

impl Drop for ActiveFrameCallbackBatch {
    fn drop(&mut self) {
        let retired =
            SCHEDULER.with(|scheduler| scheduler.borrow_mut().active_frame_callbacks.pop());
        drop(retired);
    }
}

struct EndFrameTimestamp;

impl Drop for EndFrameTimestamp {
    fn drop(&mut self) {
        SCHEDULER.with(|scheduler| {
            let mut scheduler = scheduler.borrow_mut();
            scheduler.frame_time = None;
            scheduler.post_frame_active = false;
        });
    }
}

/// Flutter `handleDrawFrame`'s post-frame phase: runs the persistent
/// callbacks and then the post-frame callbacks registered before this point.
/// Runtime-internal.
#[doc(hidden)]
pub fn end_frame(raw: Instant) {
    let now = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        let now = scheduler
            .frame_time
            .unwrap_or_else(|| scheduler.adjust(raw));
        scheduler.frame_time = Some(now);
        now
    });
    let _timestamp = EndFrameTimestamp;
    // Indexed, so no list is copied: callbacks added while this runs wait for
    // the next frame, as they would for a snapshot.
    let count = SCHEDULER.with(|scheduler| scheduler.borrow().persistent_callbacks.len());
    for index in 0..count {
        let callback =
            SCHEDULER.with(|scheduler| scheduler.borrow().persistent_callbacks.get(index).cloned());
        if let Some(callback) = callback {
            callback(now);
        }
    }
    // Each phase snapshots its own queue at entry. A persistent callback can
    // add post work for this frame; post callbacks only append to the next.
    let post = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.post_frame_active = true;
        std::mem::take(&mut scheduler.post_frame_callbacks)
    });
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
    /// a frame is being produced (before its post-frame callbacks)
    /// times itself from that frame; otherwise from the next frame. Starting a
    /// running ticker does nothing; [`stop`](Self::stop) it first to restart
    /// the clock. Returns the run's [`TickerFuture`].
    pub fn start(&self) -> TickerFuture {
        if !self.state.active.replace(true) {
            self.state.started.set(SCHEDULER.with(|scheduler| {
                let scheduler = scheduler.borrow();
                if scheduler.post_frame_active {
                    None
                } else {
                    scheduler.frame_time
                }
            }));
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
