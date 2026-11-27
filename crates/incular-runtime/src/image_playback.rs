use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc,
};

use incular_image::{
    AnimatedWebpDecoder, ImageConfiguration, ImageDecodeCompletion, ImageError, ImageFrameCodec,
    ImageInfo, ImagePlayback, ImageResizeOptions,
};

use crate::tasks::{RuntimeSpawner, TaskFailure, TaskScope};

impl RuntimeSpawner {
    /// Opens an animated WebP on the runtime's worker pool. Mount the
    /// returned source with `Image::playback` while the header is loading.
    /// At most one frame decode per source is submitted at a time.
    /// Still WebP images should use the ordinary image providers.
    #[must_use]
    pub fn load_animated_webp(&self, bytes: Arc<[u8]>) -> ImagePlayback {
        self.load_animated_webp_in(
            self.application_scope(),
            bytes,
            ImageResizeOptions::default(),
        )
    }

    pub(crate) fn load_animated_webp_in(
        &self,
        scope: &TaskScope,
        bytes: Arc<[u8]>,
        resize: ImageResizeOptions,
    ) -> ImagePlayback {
        let playback = ImagePlayback::scheduled();
        let weak = playback.downgrade();
        let spawner = self.clone();
        let decode_scope = scope.child();
        let _initialization = self.spawn_blocking_in(
            scope,
            move || AnimatedWebpDecoder::from_bytes(bytes),
            move |result, _| {
                let Some(playback) = weak.upgrade() else {
                    return;
                };
                match result {
                    Ok(Ok(Some(decoder))) => {
                        playback.attach(Rc::new(WorkerWebpCodec::new(
                            decoder,
                            spawner,
                            decode_scope,
                            resize,
                        )));
                    }
                    Ok(Ok(None)) => {
                        playback.attach_error(ImageError::Decode("WebP has no animation".into()));
                    }
                    Ok(Err(error)) => {
                        playback.attach_error(error);
                    }
                    Err(error) => {
                        playback.attach_error(worker_error(error));
                    }
                }
            },
        );
        playback
    }
}

fn worker_error(error: TaskFailure) -> ImageError {
    ImageError::Decode(format!("animated WebP worker failed: {error:?}"))
}

struct WorkerWebpCodec {
    work: Rc<Work>,
    frames: NonZeroUsize,
    repeats: Option<usize>,
}

struct Work {
    spawner: RuntimeSpawner,
    scope: TaskScope,
    ready: RefCell<Option<AnimatedWebpDecoder>>,
    resize: ImageResizeOptions,
    outstanding: RefCell<VecDeque<ImageDecodeCompletion>>,
    fatal: RefCell<Option<ImageError>>,
    active: Cell<bool>,
}

impl WorkerWebpCodec {
    fn new(
        decoder: AnimatedWebpDecoder,
        spawner: RuntimeSpawner,
        scope: TaskScope,
        resize: ImageResizeOptions,
    ) -> Self {
        let frames = decoder.frame_count();
        Self {
            frames,
            repeats: decoder.repetition_count(),
            work: Rc::new(Work {
                scope,
                spawner,
                // Flutter 3.50 passes target dimensions only to SingleFrameCodec.
                // The multi-frame engine decoder renders the encoded canvas size.
                resize: if frames.get() == 1 {
                    resize
                } else {
                    ImageResizeOptions::default()
                },
                ready: RefCell::new(Some(decoder)),
                outstanding: RefCell::new(VecDeque::new()),
                fatal: RefCell::new(None),
                active: Cell::new(false),
            }),
        }
    }
}

impl ImageFrameCodec for WorkerWebpCodec {
    fn frame_count(&self) -> NonZeroUsize {
        self.frames
    }

    fn repetition_count(&self) -> Option<usize> {
        self.repeats
    }

    fn decode_next(&self, complete: ImageDecodeCompletion) {
        self.work.outstanding.borrow_mut().push_back(complete);
        self.work.start_next();
    }
}

impl Drop for WorkerWebpCodec {
    fn drop(&mut self) {
        self.work.scope.cancel();
    }
}

impl Work {
    fn start_next(self: &Rc<Self>) {
        if self.active.get() || self.scope.is_cancelled() {
            return;
        }
        let fatal = self.fatal.borrow().clone();
        if let Some(error) = fatal {
            let callbacks = std::mem::take(&mut *self.outstanding.borrow_mut());
            for complete in callbacks {
                complete(Err(error.clone()));
            }
            return;
        }
        let Some(complete) = self.outstanding.borrow_mut().pop_front() else {
            return;
        };
        let mut decoder = self
            .ready
            .borrow_mut()
            .take()
            .expect("idle codec has a decoder");
        self.active.set(true);
        let weak = Rc::downgrade(self);
        let resize = self.resize;
        let _frame_task = self.spawner.spawn_blocking_in(
            &self.scope,
            move || {
                let result = decoder.decode_next().and_then(|mut frame| {
                    if resize.width.is_some() || resize.height.is_some() {
                        let scale = frame.image.scale();
                        let image = resize
                            .resize(frame.image.into_image(), &ImageConfiguration::empty())?;
                        frame.image = ImageInfo::new(image).with_scale(scale);
                    }
                    Ok(frame)
                });
                (decoder, result)
            },
            move |result, _| {
                let Some(work) = weak.upgrade() else {
                    return;
                };
                work.active.set(false);
                let frame = match result {
                    Ok((decoder, frame)) => {
                        *work.ready.borrow_mut() = Some(decoder);
                        frame
                    }
                    Err(error) => {
                        let error = worker_error(error);
                        *work.fatal.borrow_mut() = Some(error.clone());
                        Err(error)
                    }
                };
                complete(frame);
                work.start_next();
            },
        );
    }
}
