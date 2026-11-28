use std::{
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{
    ImageChunkEvent, ImageChunkSubscription, ImageError, ImageFrameSubscription, ImageInfo,
    ImagePlayback, ImagePlaybackSubscription, ImageStream,
};

pub type ImageListener = Rc<dyn Fn(ImageInfo, bool)>;
pub type ImageChunkListener = Rc<dyn Fn(ImageChunkEvent)>;
pub type ImageErrorListener = Rc<dyn Fn(ImageError)>;

/// A reusable group of image, chunk and error handlers.
///
/// Listener identity is the identity of each shared callback allocation,
/// not the values captured by closures. Clone the descriptor or its
/// callback references to keep that identity.
#[derive(Clone)]
pub struct ImageStreamListener {
    on_image: ImageListener,
    on_chunk: Option<ImageChunkListener>,
    on_error: Option<ImageErrorListener>,
    report_errors: bool,
}

impl ImageStreamListener {
    #[must_use]
    pub fn new(on_image: ImageListener) -> Self {
        Self {
            on_image,
            on_chunk: None,
            on_error: None,
            report_errors: true,
        }
    }

    #[must_use]
    pub fn on_chunk(mut self, handler: ImageChunkListener) -> Self {
        self.on_chunk = Some(handler);
        self
    }

    #[must_use]
    pub fn on_error(mut self, handler: ImageErrorListener) -> Self {
        self.on_error = Some(handler);
        self
    }

    /// Whether unhandled failures may be reported after this listener
    /// is removed. Registering a listener with this disabled suppresses
    /// the completer's unhandled-error report permanently, without
    /// suppressing delivery to active error callbacks.
    #[must_use]
    pub const fn report_errors(mut self, report: bool) -> Self {
        self.report_errors = report;
        self
    }
}

fn address<T: ?Sized>(callback: &Rc<T>) -> *const () {
    Rc::as_ptr(callback).cast()
}

impl PartialEq for ImageStreamListener {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.on_image, &other.on_image)
            && self.on_chunk.as_ref().map(address) == other.on_chunk.as_ref().map(address)
            && self.on_error.as_ref().map(address) == other.on_error.as_ref().map(address)
            && self.report_errors == other.report_errors
    }
}

impl Eq for ImageStreamListener {}

impl Hash for ImageStreamListener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        address(&self.on_image).hash(state);
        self.on_chunk.as_ref().map(address).hash(state);
        self.on_error.as_ref().map(address).hash(state);
        self.report_errors.hash(state);
    }
}

impl fmt::Debug for ImageStreamListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageStreamListener")
            .field("has_chunk_handler", &self.on_chunk.is_some())
            .field("has_error_handler", &self.on_error.is_some())
            .field("report_errors", &self.report_errors)
            .finish_non_exhaustive()
    }
}

/// Holds the frame and progress registrations for one stream listener.
#[must_use = "keep the subscription while the image is being resolved"]
pub struct ImageStreamSubscription {
    _frame: ImageFrameSubscription,
    _chunks: Option<ImageChunkSubscription>,
}

impl ImageStream {
    /// Registers the handlers for one provider resolution. A previously
    /// ready frame is delivered synchronously; progress is never replayed.
    /// An error without an error handler is still available in stream state.
    pub fn listen(&self, listener: ImageStreamListener) -> ImageStreamSubscription {
        let ImageStreamListener {
            on_image,
            on_chunk,
            on_error,
            report_errors,
        } = listener;
        if !report_errors {
            self.suppress_unhandled_errors();
        }
        let handles_error = on_error.is_some();
        let chunks = on_chunk.map(|handle| self.listen_chunks(move |event| handle(event)));
        let source = self.clone();
        let frame =
            self.listen_frame_with_error_handler(handles_error, move |result, synchronous| {
                match result {
                    Ok(image) => {
                        let scale = image.scale();
                        let mut info = ImageInfo::new(image).with_scale(scale);
                        if let Some(label) = source.debug_label() {
                            info = info.with_debug_label(label);
                        }
                        on_image(info, synchronous);
                    }
                    Err(error) => {
                        if let Some(handle) = on_error {
                            handle(error);
                        }
                    }
                }
            });
        ImageStreamSubscription {
            _frame: frame,
            _chunks: chunks,
        }
    }
}

impl ImagePlayback {
    /// Registers one group for both animated frame delivery and live
    /// progress reports. Keep the returned token while observing.
    pub fn listen_group(&self, listener: ImageStreamListener) -> Option<ImagePlaybackSubscription> {
        let ImageStreamListener {
            on_image,
            on_chunk,
            on_error,
            report_errors,
        } = listener;
        let handles_error = on_error.is_some();
        self.listen_with_chunks(
            move |result, synchronous| match result {
                Ok(image) => on_image(image, synchronous),
                Err(error) => {
                    if let Some(handle) = &on_error {
                        handle(error);
                    }
                }
            },
            on_chunk,
            handles_error,
            report_errors,
        )
    }
}
