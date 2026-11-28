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
