use incular_core::time::Instant;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt,
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::{Rc, Weak},
    time::Duration,
};

use crate::{ImageChunkEvent, ImageChunkListener, ImageError, ImageInfo};

mod scheduled;

/// A decoded image and the requested time it should remain visible.
pub struct DecodedImageFrame {
    pub image: ImageInfo,
    pub duration: Duration,
}

pub type ImageDecodeCompletion = Box<dyn FnOnce(Result<DecodedImageFrame, ImageError>)>;

/// A UI-thread frame source. Completion can run inline or later, on the same
/// thread. Successive requests read successive frames, wrapping as needed.
/// After idle playback restarts, a request can overlap an obsolete request;
/// the source should preserve its decode order even if completions arrive late.
pub trait ImageFrameCodec {
    fn frame_count(&self) -> NonZeroUsize;

    /// Repeats after the initial play, or `None` for unbounded playback.
    fn repetition_count(&self) -> Option<usize>;

    fn decode_next(&self, complete: ImageDecodeCompletion);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFrameDemand {
    /// Present at the next app frame.
    NextFrame,
    /// Arrange for an app frame no earlier than this instant.
    At(Instant),
}

type FrameResult = Result<ImageInfo, ImageError>;
type FrameObserver = Rc<dyn Fn(FrameResult, bool)>;
type SharedState = Rc<RefCell<State>>;

struct PlaybackObserver {
    frame: FrameObserver,
    chunks: Option<ImageChunkListener>,
}

struct Source {
    codec: Rc<dyn ImageFrameCodec>,
    frames: NonZeroUsize,
    repeats: Option<usize>,
}

#[derive(Default)]
struct State {
    source: Option<Source>,
    attached: bool,
    disposed: bool,
    listeners: BTreeMap<u64, PlaybackObserver>,
    next_listener: u64,
    last_listener_callbacks: BTreeMap<u64, Box<dyn FnOnce()>>,
    next_idle_callback: u64,
    keep_alive: usize,
    generation: u64,
    decoding: bool,
    pending: Option<DecodedImageFrame>,
    current: Option<ImageInfo>,
    error: Option<ImageError>,
    decode_failed: bool,
    visible: Option<(Instant, Duration)>,
    frame_in_cycle: usize,
    cycles: usize,
}

impl State {
    fn demand(&self) -> Option<ImageFrameDemand> {
        if self.listeners.is_empty() || self.pending.is_none() {
            return None;
        }
        Some(match self.visible {
            None => ImageFrameDemand::NextFrame,
            Some((start, duration)) => ImageFrameDemand::At(start.checked_add(duration)?),
        })
    }

    fn snapshot(&self) -> Vec<FrameObserver> {
        self.listeners
            .values()
            .map(|listener| listener.frame.clone())
            .collect()
    }

    fn needs_more(&mut self) -> bool {
        let Some(source) = &self.source else {
            return false;
        };
        self.frame_in_cycle += 1;
        if self.frame_in_cycle == source.frames.get() {
            self.frame_in_cycle = 0;
            if let Some(repeats) = source.repeats {
                if self.cycles == repeats {
                    return false;
                }
                self.cycles += 1;
            }
        }
        true
    }
}

/// Retained image playback driven by the host's monotonic app-frame clock.
/// Only one pending decoded frame is retained. No thread or timer is started
/// here: the host responds to demand changes and calls `advance` on app frames.
#[derive(Clone)]
pub struct ImagePlayback {
    state: SharedState,
    request_frame: Rc<dyn Fn()>,
}

/// A non-owning reference for asynchronous codec initialization. A pending
/// task can finish without keeping an unmounted animation alive.
#[derive(Clone)]
pub struct WeakImagePlayback {
    state: Weak<RefCell<State>>,
    request_frame: Weak<dyn Fn()>,
}

impl WeakImagePlayback {
    #[must_use]
    pub fn upgrade(&self) -> Option<ImagePlayback> {
        Some(ImagePlayback {
            state: self.state.upgrade()?,
            request_frame: self.request_frame.upgrade()?,
        })
    }
}

impl PartialEq for ImagePlayback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for ImagePlayback {}

impl fmt::Debug for ImagePlayback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImagePlayback")
            .finish_non_exhaustive()
    }
}

impl ImagePlayback {
    /// `request_frame` signals that the host should re-query `frame_demand`
    /// to schedule or cancel its next wake. The hook runs outside state borrows.
    pub fn new(request_frame: impl Fn() + 'static) -> Self {
        Self {
            state: Rc::new(RefCell::new(State::default())),
            request_frame: Rc::new(request_frame),
        }
    }

    /// Does not extend the playback's lifetime.
    #[must_use]
    pub fn downgrade(&self) -> WeakImagePlayback {
        WeakImagePlayback {
            state: Rc::downgrade(&self.state),
            request_frame: Rc::downgrade(&self.request_frame),
        }
    }

    /// Completes deferred attachment with a codec-initialization failure.
    /// Existing observers see it now; later observers can still replay it.
    /// An already attached or disposed playback refuses the update.
    pub fn attach_error(&self, error: ImageError) -> bool {
        let listeners = {
            let mut state = self.state.borrow_mut();
            if state.disposed || state.attached {
                return false;
            }
            state.attached = true;
            state.error = Some(error.clone());
            state.snapshot()
        };
        for listener in listeners {
            listener(Err(error.clone()), false);
        }
        true
    }

    /// Attach the codec once, including after listeners register. An unused
    /// playback does not request any frames. Returns false if already closed
    /// or previously attached.
    pub fn attach(&self, codec: Rc<dyn ImageFrameCodec>) -> bool {
        let frames = codec.frame_count();
        let repeats = codec.repetition_count();
        {
            let mut state = self.state.borrow_mut();
            if state.disposed || state.attached {
                return false;
            }
            state.attached = true;
            state.source = Some(Source {
                codec,
                frames,
                repeats,
            });
        }
        self.decode();
        true
    }

    /// Observe new frames until the subscription is dropped. The current
    /// frame and any retained error are replayed synchronously to new listeners.
    pub fn listen(
        &self,
        observer: impl Fn(Result<ImageInfo, ImageError>, bool) + 'static,
    ) -> Option<ImagePlaybackSubscription> {
        self.listen_with_chunks(observer, None)
    }

    pub(crate) fn listen_with_chunks(
        &self,
        observer: impl Fn(FrameResult, bool) + 'static,
        chunks: Option<ImageChunkListener>,
    ) -> Option<ImagePlaybackSubscription> {
        let observer: FrameObserver = Rc::new(observer);
        let (id, current, error) = {
            let mut state = self.state.borrow_mut();
            if state.disposed {
                return None;
            }
            let id = state.next_listener;
            state.next_listener = id.checked_add(1).expect("image listener id exhausted");
            state.listeners.insert(
                id,
                PlaybackObserver {
                    frame: Rc::clone(&observer),
                    chunks,
                },
            );
            (id, state.current.clone(), state.error.clone())
        };
        let subscription = ImagePlaybackSubscription {
            playback: self.clone(),
            id,
        };
        if let Some(current) = current {
            observer(Ok(current), true);
        }
        if let Some(error) = error {
            observer(Err(error), true);
        }
        self.decode();
        Some(subscription)
    }

    /// Delivers this live progress event to the active listener groups.
    /// Reports never replay for late listeners or wake the frame clock.
    pub fn report_chunk(&self, event: ImageChunkEvent) -> bool {
        let listeners = {
            let state = self.state.borrow();
            if state.disposed {
                return false;
            }
            state
                .listeners
                .values()
                .filter_map(|listener| listener.chunks.clone())
                .collect::<Vec<_>>()
        };
        for listener in listeners {
            listener(event);
        }
        true
    }

    /// Reports a transport error independently of frame decoding. The most
    /// recent error remains available to newly added listener groups.
    pub fn report_transport_error(&self, error: ImageError) -> bool {
        let listeners = {
            let mut state = self.state.borrow_mut();
            if state.disposed {
                return false;
            }
            state.error = Some(error.clone());
            state.snapshot()
        };
        for listener in listeners {
            listener(Err(error.clone()), false);
        }
        true
    }

    /// Runs once when the next active listener set becomes empty. This
    /// notification is independent of keep-alive handles, which retain
    /// the codec but do not count as active listeners.
    pub fn on_last_listener_removed(
        &self,
        callback: impl FnOnce() + 'static,
    ) -> Option<ImagePlaybackIdleSubscription> {
        let mut state = self.state.borrow_mut();
        if state.disposed {
            return None;
        }
        let id = state.next_idle_callback;
        state.next_idle_callback = id.checked_add(1).expect("image playback idle id exhausted");
        state.last_listener_callbacks.insert(id, Box::new(callback));
        Some(ImagePlaybackIdleSubscription {
            owner: Rc::downgrade(&self.state),
            id,
        })
    }

    /// Preserve the codec and previously shown frame while no listeners
    /// remain. The absence of listeners still prevents further scheduling.
    pub fn keep_alive(&self) -> Option<ImagePlaybackKeepAlive> {
        let mut state = self.state.borrow_mut();
        if state.disposed {
            return None;
        }
        state.keep_alive += 1;
        Some(ImagePlaybackKeepAlive(self.clone()))
    }

    #[must_use]
    pub fn frame_demand(&self) -> Option<ImageFrameDemand> {
        self.state.borrow().demand()
    }

    /// Present a pending frame if its predecessor's time has elapsed. A late
    /// host frame starts the next frame's duration at the actual presentation
    /// time, rather than catching up by skipping decoded animation frames.
    pub fn advance(&self, now: Instant) -> bool {
        let (result, listeners, finished) = {
            let mut state = self.state.borrow_mut();
            match state.demand() {
                None => return false,
                Some(ImageFrameDemand::At(at)) if now < at => return false,
                _ => {}
            }
            let frame = state
                .pending
                .take()
                .expect("frame demand requires a decode");
            state.visible = Some((now, frame.duration));
            state.current = Some(frame.image.clone());
            let finished = if state.needs_more() {
                None
            } else {
                state.source.take()
            };
            (frame.image, state.snapshot(), finished)
        };
        for listener in listeners {
            listener(Ok(result.clone()), false);
        }
        drop(finished);
        (self.request_frame)();
        self.decode();
        true
    }

    fn decode(&self) {
        let (codec, generation) = {
            let mut state = self.state.borrow_mut();
            if state.listeners.is_empty()
                || state.decoding
                || state.decode_failed
                || state.pending.is_some()
            {
                return;
            }
            let Some(source) = &state.source else {
                return;
            };
            let codec = Rc::clone(&source.codec);
            state.decoding = true;
            (codec, state.generation)
        };
        let weak = self.downgrade();
        codec.decode_next(Box::new(move |result| {
            if let Some(playback) = weak.upgrade() {
                playback.decoded(generation, result);
            }
        }));
    }

    fn decoded(&self, generation: u64, result: Result<DecodedImageFrame, ImageError>) {
        let (delivery, finished) = {
            let mut state = self.state.borrow_mut();
            if state.generation != generation || !state.decoding {
                return;
            }
            state.decoding = false;
            match result {
                Ok(frame)
                    if state
                        .source
                        .as_ref()
                        .is_some_and(|source| source.frames.get() > 1) =>
                {
                    state.pending = Some(frame);
                    (None, None)
                }
                Ok(frame) => {
                    state.current = Some(frame.image.clone());
                    (
                        Some((Ok(frame.image), state.snapshot())),
                        state.source.take(),
                    )
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    state.decode_failed = true;
                    // Keep the last good image and codec alive while there
                    // are owners. Adding a listener to the active set does
                    // not trigger an automatic retry loop.
                    (Some((Err(error), state.snapshot())), None)
                }
            }
        };
        if let Some((result, listeners)) = delivery {
            for listener in listeners {
                listener(result.clone(), false);
            }
        } else {
            (self.request_frame)();
        }
        drop(finished);
    }

    fn retire_if_idle(&self, remove: impl FnOnce(&mut State) -> Option<PlaybackObserver>) {
        let (removed, callbacks, cancel_wake) = {
            let mut state = self.state.borrow_mut();
            let had_listeners = !state.listeners.is_empty();
            let had_demand = state.demand().is_some();
            let removed = remove(&mut state);
            let became_idle = had_listeners && state.listeners.is_empty();
            let callbacks = if became_idle {
                state.generation = state.generation.wrapping_add(1);
                state.decoding = false;
                state.decode_failed = false;
                state.pending = None;
                std::mem::take(&mut state.last_listener_callbacks)
            } else {
                BTreeMap::new()
            };
            (removed, callbacks, became_idle && had_demand)
        };
        drop(removed);
        if cancel_wake {
            (self.request_frame)();
        }
        let mut failure = None;
        if !std::thread::panicking() {
            for callback in callbacks.into_values() {
                if let Err(panic) = catch_unwind(AssertUnwindSafe(callback)) {
                    failure.get_or_insert(panic);
                }
            }
        }
        let (source, remaining_callbacks) = {
            let mut state = self.state.borrow_mut();
            if state.listeners.is_empty() && state.keep_alive == 0 && state.next_listener != 0 {
                state.disposed = true;
                state.current = None;
                state.error = None;
                (
                    state.source.take(),
                    std::mem::take(&mut state.last_listener_callbacks),
                )
            } else {
                (None, BTreeMap::new())
            }
        };
        drop(source);
        drop(remaining_callbacks);
        if let Some(panic) = failure {
            resume_unwind(panic);
        }
    }
}

#[must_use = "keep the subscription while observing image playback"]
pub struct ImagePlaybackSubscription {
    playback: ImagePlayback,
    id: u64,
}

impl Drop for ImagePlaybackSubscription {
    fn drop(&mut self) {
        self.playback
            .retire_if_idle(|state| state.listeners.remove(&self.id));
    }
}

/// Owns a one-shot playback idle callback. Dropping before the last
/// listener leaves cancels the callback.
#[must_use = "keep the subscription until the last active listener leaves"]
pub struct ImagePlaybackIdleSubscription {
    owner: Weak<RefCell<State>>,
    id: u64,
}

impl Drop for ImagePlaybackIdleSubscription {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let removed = owner.borrow_mut().last_listener_callbacks.remove(&self.id);
            drop(removed);
        }
    }
}

#[must_use = "keep the handle while preserving idle image playback"]
pub struct ImagePlaybackKeepAlive(ImagePlayback);

impl Drop for ImagePlaybackKeepAlive {
    fn drop(&mut self) {
        self.0.retire_if_idle(|state| {
            state.keep_alive -= 1;
            None
        });
    }
}
