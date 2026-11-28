//! File-image failure classification and retry against Flutter 3.50.0.
use incular_image::{
    FileImage, ImageConfiguration, ImageError, ImageLoadState, ImageProvider, ImageSource,
    ImageStreamListener, ResizeImage,
};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

const PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "incular-file-image-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn path(&self) -> PathBuf {
        self.0.join("image.png")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_empty_file_reports_the_path_instead_of_a_corrupt_image_error() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, []).unwrap();
    let provider = FileImage::new(&path);
    let err = provider.load().unwrap_err();
    assert_eq!(err, ImageError::EmptyFile { path: path.clone() });
    assert_eq!(
        err.to_string(),
        format!(
            "{} is empty and cannot be loaded as an image.",
            path.display()
        )
    );
    let stream = provider.resolve(&ImageConfiguration::empty());
    assert_eq!(stream.debug_label().as_deref(), path.to_str());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _observer = stream.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("an empty file has no frame"))).on_error({
            let seen = seen.clone();
            Rc::new(move |error| seen.borrow_mut().push(error.clone()))
        }),
    );
    assert_eq!(seen.borrow().as_slice(), std::slice::from_ref(&err));
    let resized = ResizeImage::new(provider, Some(4), Some(4));
    assert_eq!(
        resized.resolve(&ImageConfiguration::empty()).state(),
        ImageLoadState::Failed(err),
    );
}

#[test]
fn an_initially_empty_file_can_be_reopened_after_the_image_is_written() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, []).unwrap();
    let provider = FileImage::new(&path).scale(2.);
    let config = ImageConfiguration::empty();
    let first = provider.resolve(&config);
    assert!(matches!(first.state(), ImageLoadState::Failed(_)));

    fs::write(&path, PNG).unwrap();
    let second = provider.resolve(&config);
    assert_ne!(second, first);
    let ImageLoadState::Ready(image) = second.state() else {
        panic!(
            "a retry must see the newly written content: {:?}",
            second.state()
        );
    };
    assert_eq!((image.decoded().width(), image.decoded().height()), (8, 8));
    assert_eq!(image.scale(), 2.);
    assert_eq!(image.source(), &ImageSource::FileSystem(path));
    assert!(matches!(first.state(), ImageLoadState::Failed(_)));
}

#[test]
fn unreadable_and_nonempty_nonimage_files_are_not_classified_as_empty() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let provider = FileImage::new(&path);
    assert!(matches!(provider.load(), Err(ImageError::Io(_))));

    fs::write(&path, b"not a raster image").unwrap();
    assert!(matches!(provider.load(), Err(ImageError::Decode(_))));
}

type WorkerJob = Box<dyn FnOnce() + Send>;

fn flush_image_tasks() {
    use incular_animation::scheduler::{has_ready_scheduled_task, run_one_scheduled_task};

    for _ in 0..12 {
        if !has_ready_scheduled_task() {
            return;
        }
        let _ = run_one_scheduled_task();
    }
    panic!("image UI scheduler did not settle");
}

fn held_worker() -> (
    incular_image::FileImageWorker,
    Rc<RefCell<std::collections::VecDeque<WorkerJob>>>,
) {
    let jobs = Rc::new(RefCell::new(std::collections::VecDeque::new()));
    let worker = incular_image::FileImageWorker::new({
        let jobs = jobs.clone();
        move |job| jobs.borrow_mut().push_back(job)
    });
    (worker, jobs)
}

fn complete_next_file_job(queue: &RefCell<std::collections::VecDeque<WorkerJob>>) {
    flush_image_tasks();
    let job = queue
        .borrow_mut()
        .pop_front()
        .expect("one waiting file load");
    std::thread::spawn(job).join().unwrap();
    flush_image_tasks();
}

#[test]
fn a_hosted_empty_file_is_pending_then_untracked_before_an_error_observer_retries() {
    use incular_image::BundleImageCacheStatus;

    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, []).unwrap();
    let provider = FileImage::new(&path).scale(2.);
    let cfg = ImageConfiguration::empty();
    let (worker, queue) = held_worker();
    let _scope = worker.enter();

    assert_eq!(provider.cache_status(), BundleImageCacheStatus::default());
    let pending = provider.resolve(&cfg);
    assert!(matches!(pending.state(), ImageLoadState::Loading(_)));
    assert_eq!(provider.resolve(&cfg), pending);
    assert!(provider.cache_status().pending);
    assert!(
        queue.borrow().is_empty(),
        "worker starts on a later UI turn"
    );

    let seen = Rc::new(RefCell::new(None));
    let _observer = pending.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("empty image has no frame"))).on_error({
            let seen = seen.clone();
            let provider = provider.clone();
            Rc::new(move |error| {
                *seen.borrow_mut() = Some((error.clone(), provider.cache_status()))
            })
        }),
    );
    complete_next_file_job(&queue);
    assert_eq!(
        *seen.borrow(),
        Some((
            ImageError::EmptyFile { path: path.clone() },
            BundleImageCacheStatus::default(),
        ))
    );
    assert_eq!(provider.cache_status(), BundleImageCacheStatus::default());

    fs::write(&path, PNG).unwrap();
    let retry = provider.resolve(&cfg);
    assert_ne!(retry, pending);
    assert!(provider.cache_status().pending);
    complete_next_file_job(&queue);
    assert!(queue.borrow().is_empty());
    let ImageLoadState::Ready(decoded) = retry.state() else {
        panic!("{retry:?}")
    };
    assert_eq!(
        (
            decoded.decoded().width(),
            decoded.decoded().height(),
            decoded.scale()
        ),
        (8, 8, 2.)
    );
    assert_eq!(decoded.source(), &ImageSource::FileSystem(path));
    assert!(provider.cache_status().keep_alive);
}

#[test]
fn an_invalid_nonempty_file_keeps_the_failed_pending_key_until_eviction() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, b"garbage image bytes").unwrap();
    let provider = FileImage::new(&path);
    let cfg = ImageConfiguration::empty();
    let (worker, queue) = held_worker();
    let _scope = worker.enter();

    let stream = provider.resolve(&cfg);
    assert!(provider.cache_status().pending);
    complete_next_file_job(&queue);
    assert!(matches!(
        stream.state(),
        ImageLoadState::Failed(ImageError::Decode(reason)) if reason.starts_with("Invalid image data:")
    ));
    assert!(provider.cache_status().pending);
    assert!(!provider.cache_status().keep_alive);

    fs::write(&path, PNG).unwrap();
    assert_eq!(provider.resolve(&cfg), stream);
    flush_image_tasks();
    assert!(
        queue.borrow().is_empty(),
        "failed decode prevents another read"
    );
    assert!(provider.evict());
    assert!(!provider.cache_status().pending);
    let retry = provider.resolve(&cfg);
    assert_ne!(retry, stream);
    complete_next_file_job(&queue);
    assert!(matches!(retry.state(), ImageLoadState::Ready(_)));
}

#[test]
fn discarding_a_file_worker_job_does_not_leak_a_pending_key() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, PNG).unwrap();
    let provider = FileImage::new(&path);
    let worker = incular_image::FileImageWorker::new(drop);
    let _scope = worker.enter();
    let stream = provider.resolve(&ImageConfiguration::empty());
    assert!(provider.cache_status().pending);
    flush_image_tasks();
    assert!(matches!(
        stream.state(),
        ImageLoadState::Failed(ImageError::Io(_))
    ));
    assert!(!provider.cache_status().pending);
}

#[test]
fn a_hosted_resize_preserves_the_empty_file_failure_without_storing_pixels() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, []).unwrap();
    let file = FileImage::new(&path);
    let provider = ResizeImage::new(file.clone(), Some(4), Some(4));
    let cfg = ImageConfiguration::empty();
    let (worker, queue) = held_worker();
    let _scope = worker.enter();
    let first = provider.resolve(&cfg);
    assert!(matches!(first.state(), ImageLoadState::Loading(_)));
    assert_eq!(provider.resolve(&cfg), first);
    complete_next_file_job(&queue);
    assert_eq!(
        first.state(),
        ImageLoadState::Failed(ImageError::EmptyFile { path: path.clone() })
    );
    assert_eq!(file.cache_status(), Default::default());
    assert!(queue.borrow().is_empty());
    incular_animation::scheduler::flush_frame_microtasks();

    fs::write(&path, PNG).unwrap();
    let retry = provider.resolve(&cfg);
    assert_ne!(retry, first);
    complete_next_file_job(&queue);
    let ImageLoadState::Ready(image) = retry.state() else {
        panic!("{retry:?}")
    };
    assert_eq!((image.decoded().width(), image.decoded().height()), (4, 4));
}

#[test]
fn failed_resized_decode_is_evicted_after_observers_without_touching_the_raw_key() {
    use incular_animation::scheduler::flush_frame_microtasks;

    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, b"not a raster image").unwrap();
    let file = FileImage::new(&path);
    let provider = ResizeImage::new(file.clone(), Some(4), Some(4));
    let cfg = ImageConfiguration::empty();
    let (worker, queue) = held_worker();
    let _scope = worker.enter();

    let raw = file.resolve(&cfg);
    complete_next_file_job(&queue);
    assert!(matches!(
        raw.state(),
        ImageLoadState::Failed(ImageError::Decode(_))
    ));
    assert!(file.cache_status().pending);

    let first = provider.resolve(&cfg);
    let saw_original_in_callback = Rc::new(std::cell::Cell::new(false));
    let listener = first.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("corrupt file has no image"))).on_error({
            let seen = saw_original_in_callback.clone();
            let provider = provider.clone();
            let first = first.clone();
            let cfg = cfg.clone();
            Rc::new(move |_| seen.set(provider.resolve(&cfg) == first))
        }),
    );
    complete_next_file_job(&queue);
    assert!(saw_original_in_callback.get());
    assert_eq!(provider.resolve(&cfg), first);
    drop(listener);
    flush_frame_microtasks();

    fs::write(&path, PNG).unwrap();
    let second = provider.resolve(&cfg);
    assert_ne!(second, first);
    complete_next_file_job(&queue);
    let ImageLoadState::Ready(image) = second.state() else {
        panic!("{second:?}")
    };
    assert_eq!((image.decoded().width(), image.decoded().height()), (4, 4));
    assert!(file.cache_status().pending);
    assert_eq!(file.resolve(&cfg), raw);
}

#[test]
fn a_stale_resize_failure_cannot_evict_the_replacement_pending_job() {
    use incular_animation::scheduler::flush_frame_microtasks;
    use incular_image::shared_bundle_image_cache;

    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, b"invalid raster").unwrap();
    let provider = ResizeImage::new(FileImage::new(&path), Some(4), Some(4));
    let cfg = ImageConfiguration::empty();
    let (worker, queue) = held_worker();
    let _scope = worker.enter();

    let old = provider.resolve(&cfg);
    complete_next_file_job(&queue);
    assert!(matches!(
        old.state(),
        ImageLoadState::Failed(ImageError::Decode(_))
    ));
    drop(old);
    shared_bundle_image_cache().clear();
    fs::write(&path, PNG).unwrap();
    let replacement = provider.resolve(&cfg);
    assert!(matches!(replacement.state(), ImageLoadState::Loading(_)));
    flush_frame_microtasks();
    assert_eq!(provider.resolve(&cfg), replacement);
    complete_next_file_job(&queue);
    assert!(matches!(replacement.state(), ImageLoadState::Ready(_)));
}
