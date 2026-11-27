use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc,
};

use incular_image::{
    AnimatedWebpDecoder, ImageDecodeCompletion, ImageError, ImageFrameCodec, ImagePlayback,
};

use crate::tasks::{RuntimeSpawner, TaskFailure, TaskScope};

impl RuntimeSpawner {
    /// Opens an animated WebP on the runtime's worker pool. Mount the
    /// returned source with `Image::playback` while the header is loading.
    /// At most one frame decode per source is submitted at a time.
    /// Still WebP images should use the ordinary image providers.
    #[must_use]
    pub fn load_animated_webp(&self, bytes: Arc<[u8]>) -> ImagePlayback {
        let playback = ImagePlayback::scheduled();
        let weak = playback.downgrade();
        let spawner = self.clone();
        let _initialization = self.spawn_blocking(
            move || AnimatedWebpDecoder::from_bytes(bytes),
            move |result, _| {
                let Some(playback) = weak.upgrade() else {
                    return;
                };
                match result {
                    Ok(Ok(Some(decoder))) => {
                        playback.attach(Rc::new(WorkerWebpCodec::new(decoder, spawner)));
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
    outstanding: RefCell<VecDeque<ImageDecodeCompletion>>,
    fatal: RefCell<Option<ImageError>>,
    active: Cell<bool>,
}

impl WorkerWebpCodec {
    fn new(decoder: AnimatedWebpDecoder, spawner: RuntimeSpawner) -> Self {
        Self {
            frames: decoder.frame_count(),
            repeats: decoder.repetition_count(),
            work: Rc::new(Work {
                scope: spawner.scope(),
                spawner,
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
        let _frame_task = self.spawner.spawn_blocking_in(
            &self.scope,
            move || {
                let result = decoder.decode_next();
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
