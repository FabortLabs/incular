//! Flutter's image stream (`painting/image_stream.dart`): the loading state
//! of one provider resolution, observed on the UI thread.
//!
//! An [`ImageStream`] starts loading and ends ready or failed. Providers
//! that load synchronously return a stream that is already complete; others
//! return a loading stream and complete it later through its
//! [`ImageStreamCompleter`], from a UI-thread callback such as the runtime's
//! task completion. Reading [`ImageStream::state`] inside a build makes the
//! build depend on the stream, so each change rebuilds it on the next frame.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use futures_channel::oneshot;
use incular_animation::scheduler::{Priority, schedule_async_task};
use incular_core::reactivity::DependencySource;

use crate::{ImageConfiguration, ImageError, ImageHandle, ImageProvider};

/// Flutter's `ImageChunkEvent`: progress of an image still loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageChunkEvent {
    /// Bytes received so far.
    pub cumulative_bytes_loaded: u64,
    /// The total, when the source knows it.
    pub expected_total_bytes: Option<u64>,
}

impl ImageChunkEvent {
    /// Progress with a known or unknown total.
    #[must_use]
    pub const fn new(cumulative_bytes_loaded: u64, expected_total_bytes: Option<u64>) -> Self {
        Self {
            cumulative_bytes_loaded,
            expected_total_bytes,
        }
    }
}

/// Where one image resolution stands.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageLoadState {
    /// Still loading, with the latest progress report if any.
    Loading(Option<ImageChunkEvent>),
    /// Decoded and ready to paint.
    Ready(ImageHandle),
    /// Loading or decoding failed.
    Failed(ImageError),
}

type FrameMap = Box<dyn FnOnce(ImageHandle) -> Result<ImageHandle, ImageError>>;

struct StreamCell {
    state: RefCell<ImageLoadState>,
    changes: DependencySource,
    completed: Cell<bool>,
    /// The stream this one follows after [`ImageStreamCompleter::follow`].
    source: RefCell<Option<ImageStream>>,
    map: RefCell<Option<FrameMap>>,
}

/// One provider resolution's loading state (Flutter `ImageStream`). Clones
/// share the state; two streams are equal only when one was cloned from the
/// other.
#[derive(Clone)]
pub struct ImageStream {
    cell: Rc<StreamCell>,
}

impl PartialEq for ImageStream {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cell, &other.cell)
    }
}

impl Eq for ImageStream {}

impl fmt::Debug for ImageStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageStream")
            .field("state", &*self.cell.state.borrow())
            .finish()
    }
}

impl ImageStream {
    fn with(state: ImageLoadState) -> Self {
        let completed = !matches!(state, ImageLoadState::Loading(_));
        Self {
            cell: Rc::new(StreamCell {
                state: RefCell::new(state),
                changes: DependencySource::default(),
                completed: Cell::new(completed),
                source: RefCell::new(None),
                map: RefCell::new(None),
            }),
        }
    }

    /// A loading stream and the completer that finishes it.
    #[must_use]
    pub fn loading() -> (Self, ImageStreamCompleter) {
        let stream = Self::with(ImageLoadState::Loading(None));
        let completer = ImageStreamCompleter {
            stream: stream.clone(),
        };
        (stream, completer)
    }

    /// A stream that is already ready, as a synchronous provider returns.
    #[must_use]
    pub fn ready(image: ImageHandle) -> Self {
        Self::with(ImageLoadState::Ready(image))
    }

    /// A stream that already failed.
    #[must_use]
    pub fn failed(error: ImageError) -> Self {
        Self::with(ImageLoadState::Failed(error))
    }

    /// A completed stream from a synchronous load.
    #[must_use]
    pub fn from_result(result: Result<ImageHandle, ImageError>) -> Self {
        match result {
            Ok(image) => Self::ready(image),
            Err(error) => Self::failed(error),
        }
    }

    /// Resolves a one-shot image callback into a UI-thread stream. A
    /// synchronous completion is already ready or failed when this returns;
    /// delayed completions wake the UI scheduler and invalidate listeners
    /// on the owning thread rather than mutating UI state from a loader.
    #[must_use]
    pub fn from_load_callback(start: impl FnOnce(crate::ImageLoadCallback)) -> Self {
        let (send, mut receive) = oneshot::channel();
        start(Box::new(move |outcome| {
            let _ = send.send(outcome);
        }));
        match receive.try_recv() {
            Ok(Some(result)) => return Self::from_result(result),
            Err(_) => return Self::failed(abandoned_image_load()),
            Ok(None) => {}
        }

        let stream = Self::with(ImageLoadState::Loading(None));
        let owner = Rc::downgrade(&stream.cell);
        // The scheduler owns the task independently of its returned future.
        // The weak cell prevents a transport from retaining an unmounted image.
        drop(schedule_async_task(
            move || async move {
                let result = receive
                    .await
                    .unwrap_or_else(|_| Err(abandoned_image_load()));
                if let Some(cell) = owner.upgrade() {
                    let state = match result {
                        Ok(image) => ImageLoadState::Ready(image),
                        Err(error) => ImageLoadState::Failed(error),
                    };
                    Self { cell }.set(state);
                }
            },
            Priority::ANIMATION,
        ));
        stream
    }

    /// Transforms a single decoded frame on the receiving UI thread.
    /// Pending progress and source failures are preserved. A ready frame
    /// is mapped once, even if several widget builds read this stream.
    #[must_use]
    pub fn map_image(
        self,
        map: impl FnOnce(ImageHandle) -> Result<ImageHandle, ImageError> + 'static,
    ) -> Self {
        if self.is_completed() {
            return match self.state() {
                ImageLoadState::Ready(image) => Self::from_result(map(image)),
                ImageLoadState::Failed(error) => Self::failed(error),
                ImageLoadState::Loading(_) => unreachable!("complete stream has an outcome"),
            };
        }
        let mapped = Self::with(ImageLoadState::Loading(None));
        *mapped.cell.source.borrow_mut() = Some(self);
        *mapped.cell.map.borrow_mut() = Some(Box::new(map));
        mapped
    }

    /// The current state. Reading it inside a build makes the build depend
    /// on this stream.
    #[must_use]
    pub fn state(&self) -> ImageLoadState {
        self.cell.changes.track();
        if let Some(source) = self.source() {
            let outcome = source.state();
            let map = if matches!(outcome, ImageLoadState::Loading(_)) {
                None
            } else {
                self.cell.map.borrow_mut().take()
            };
            if let Some(map) = map {
                let outcome = match outcome {
                    ImageLoadState::Ready(image) => match map(image) {
                        Ok(image) => ImageLoadState::Ready(image),
                        Err(error) => ImageLoadState::Failed(error),
                    },
                    other => other,
                };
                self.cell.completed.set(true);
                *self.cell.source.borrow_mut() = None;
                *self.cell.state.borrow_mut() = outcome.clone();
                return outcome;
            }
            return outcome;
        }
        self.cell.state.borrow().clone()
    }

    fn source(&self) -> Option<ImageStream> {
        self.cell.source.borrow().clone()
    }

    /// Whether `other` is this stream or one it follows.
    fn reaches(&self, other: &Self) -> bool {
        self == other || self.source().is_some_and(|source| source.reaches(other))
    }

    /// Whether the stream is ready or failed. A widget that finds its stream
    /// already complete when it subscribes treats the image as synchronously
    /// loaded, as Flutter's `synchronousCall` does for cached images.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.source()
            .map_or_else(|| self.cell.completed.get(), |source| source.is_completed())
    }

    fn set(&self, state: ImageLoadState) -> bool {
        if self.cell.completed.get() || self.cell.source.borrow().is_some() {
            return false;
        }
        let completes = !matches!(state, ImageLoadState::Loading(_));
        *self.cell.state.borrow_mut() = state;
        self.cell.completed.set(completes);
        self.cell.changes.notify();
        true
    }
}

fn abandoned_image_load() -> ImageError {
    ImageError::Io("image loader disconnected before delivering a result".into())
}

/// Reports progress and the outcome of a loading [`ImageStream`]
/// (Flutter's `ImageStreamCompleter`). A stream completes once; reports
/// after that are ignored and return `false`.
pub struct ImageStreamCompleter {
    stream: ImageStream,
}

impl fmt::Debug for ImageStreamCompleter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageStreamCompleter")
            .field("stream", &self.stream)
            .finish()
    }
}

impl ImageStreamCompleter {
    /// The stream this completer finishes.
    #[must_use]
    pub fn stream(&self) -> ImageStream {
        self.stream.clone()
    }

    /// Flutter `reportImageChunkEvent`.
    pub fn report_chunk(&self, event: ImageChunkEvent) -> bool {
        self.stream.set(ImageLoadState::Loading(Some(event)))
    }

    /// Completes the stream with a decoded image.
    pub fn complete(&self, image: ImageHandle) -> bool {
        self.stream.set(ImageLoadState::Ready(image))
    }

    /// Completes the stream with an error (Flutter `reportError`).
    pub fn fail(&self, error: ImageError) -> bool {
        self.stream.set(ImageLoadState::Failed(error))
    }

    /// Makes the stream follow `source` from now on, reporting its state
    /// instead of its own, as Flutter's `ImageStream.setCompleter` hands a
    /// stream to the completer of a deferred resolution. Builds that read
    /// the stream rebuild once now and then on each change of `source`.
    /// Returns `false` without effect when the stream already completed or
    /// follows a stream, or when `source` is or follows this stream.
    pub fn follow(&self, source: &ImageStream) -> bool {
        let stream = &self.stream;
        if stream.cell.completed.get()
            || stream.cell.source.borrow().is_some()
            || source.reaches(stream)
        {
            return false;
        }
        *stream.cell.source.borrow_mut() = Some(source.clone());
        stream.cell.changes.notify();
        true
    }
}

/// A type-erased [`ImageProvider`], so widgets can hold any provider and
/// compare a new one with the previous one. Two erased providers are equal
/// when they hold the same provider type with equal values.
#[derive(Clone)]
pub struct AnyImageProvider {
    provider: Rc<dyn Any>,
    eq: fn(&dyn Any, &dyn Any) -> bool,
    resolve: fn(&dyn Any, &ImageConfiguration) -> ImageStream,
    load: fn(&dyn Any, &ImageConfiguration) -> Result<ImageHandle, ImageError>,
    debug: fn(&dyn Any, &mut fmt::Formatter<'_>) -> fmt::Result,
}

fn eq_erased<P: ImageProvider + 'static>(left: &dyn Any, right: &dyn Any) -> bool {
    match (left.downcast_ref::<P>(), right.downcast_ref::<P>()) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn resolve_erased<P: ImageProvider + 'static>(
    provider: &dyn Any,
    configuration: &ImageConfiguration,
) -> ImageStream {
    provider.downcast_ref::<P>().map_or_else(
        || ImageStream::failed(ImageError::Io("image provider type mismatch".into())),
        |provider| provider.resolve(configuration),
    )
}

fn load_erased<P: ImageProvider + 'static>(
    provider: &dyn Any,
    configuration: &ImageConfiguration,
) -> Result<ImageHandle, ImageError> {
    provider.downcast_ref::<P>().map_or_else(
        || Err(ImageError::Io("image provider type mismatch".into())),
        |provider| provider.load_with(configuration),
    )
}

fn debug_erased<P: ImageProvider + 'static>(
    provider: &dyn Any,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match provider.downcast_ref::<P>() {
        Some(provider) => fmt::Debug::fmt(provider, formatter),
        None => formatter.write_str("AnyImageProvider(..)"),
    }
}

impl AnyImageProvider {
    /// Erases `provider`.
    #[must_use]
    pub fn new<P: ImageProvider + 'static>(provider: P) -> Self {
        Self {
            provider: Rc::new(provider),
            eq: eq_erased::<P>,
            resolve: resolve_erased::<P>,
            load: load_erased::<P>,
            debug: debug_erased::<P>,
        }
    }

    /// Resolves the provider for `configuration` (Flutter
    /// `ImageProvider.resolve`).
    #[must_use]
    pub fn resolve(&self, configuration: &ImageConfiguration) -> ImageStream {
        (self.resolve)(&*self.provider, configuration)
    }

    /// Loads the provider synchronously for `configuration` (its
    /// [`ImageProvider::load_with`]).
    pub fn load_with(&self, configuration: &ImageConfiguration) -> Result<ImageHandle, ImageError> {
        (self.load)(&*self.provider, configuration)
    }

    /// The provider, when it has type `P`.
    #[must_use]
    pub fn downcast_ref<P: ImageProvider + 'static>(&self) -> Option<&P> {
        self.provider.downcast_ref::<P>()
    }
}

impl PartialEq for AnyImageProvider {
    fn eq(&self, other: &Self) -> bool {
        (*self.provider).type_id() == (*other.provider).type_id()
            && (self.eq)(&*self.provider, &*other.provider)
    }
}

// Erased equality compares providers of one type with their own `Eq`.
impl Eq for AnyImageProvider {}

impl fmt::Debug for AnyImageProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.debug)(&*self.provider, formatter)
    }
}
