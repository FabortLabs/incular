//! Scoped access to the host's existing blocking executor for file images.

use std::{cell::RefCell, rc::Rc};

use incular_animation::scheduler::{Priority, schedule_task};

use crate::{
    FileImage, ImageConfiguration, ImageError, ImageHandle, ImageLoadCallback, ImageProvider,
    ImageResizeOptions,
};

type Job = Box<dyn FnOnce() + Send>;

thread_local! {
    static CURRENT: RefCell<Option<FileImageWorker>> = const { RefCell::new(None) };
}

/// Lets a UI host dispatch decoded file IO on its own blocking executor.
#[doc(hidden)]
#[derive(Clone)]
pub struct FileImageWorker(Rc<dyn Fn(Job)>);

impl FileImageWorker {
    #[must_use]
    pub fn new(submit: impl Fn(Job) + 'static) -> Self {
        Self(Rc::new(submit))
    }

    #[must_use]
    pub fn enter(&self) -> FileImageWorkerScope {
        FileImageWorkerScope(CURRENT.replace(Some(self.clone())))
    }

    pub(crate) fn available() -> bool {
        CURRENT.with(|slot| slot.borrow().is_some())
    }

    pub(crate) fn load(
        file: FileImage,
        resize: Option<ImageResizeOptions>,
        complete: ImageLoadCallback,
    ) {
        let work = move || {
            file.load().and_then(|image| match resize {
                Some(size) => size.resize(image, &ImageConfiguration::empty()),
                None => Ok(image),
            })
        };

        let Some(worker) = CURRENT.with(|slot| slot.borrow().clone()) else {
            complete(work());
            return;
        };
        let pending = PendingResult(Some(complete));
        // Publish the cache's pending entry before a worker can complete the
        // read. Even a zero-byte local file completes on a later UI turn.
        drop(schedule_task(
            move || (worker.0)(Box::new(move || pending.finish(work()))),
            Priority::ANIMATION,
        ));
    }
}

/// Restores the previous file worker on exit, including a nested runtime or
/// a failed frame.
#[doc(hidden)]
pub struct FileImageWorkerScope(Option<FileImageWorker>);

impl Drop for FileImageWorkerScope {
    fn drop(&mut self) {
        let _ = CURRENT.try_with(|slot| slot.replace(self.0.take()));
    }
}

struct PendingResult(Option<ImageLoadCallback>);

impl PendingResult {
    fn finish(mut self, result: Result<ImageHandle, ImageError>) {
        if let Some(complete) = self.0.take() {
            complete(result);
        }
    }
}

impl Drop for PendingResult {
    fn drop(&mut self) {
        if let Some(complete) = self.0.take() {
            complete(Err(ImageError::Io(
                "file image worker stopped before delivering a result".into(),
            )));
        }
    }
}
