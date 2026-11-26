//! Flutter's image stream (`painting/image_stream.dart`, paint-012): a
//! provider resolution starts loading, reports chunk progress, and completes
//! once with an image or an error.

use incular_image::*;

fn pixel() -> ImageHandle {
    ImageHandle::from_rgba8(1, 1, vec![255, 255, 255, 255]).expect("one pixel")
}

#[test]
fn a_loading_stream_reports_progress_then_completes_once() {
    let (stream, completer) = ImageStream::loading();
    assert_eq!(stream.state(), ImageLoadState::Loading(None));
    assert!(!stream.is_completed());

    let chunk = ImageChunkEvent::new(10, Some(40));
    assert!(completer.report_chunk(chunk));
    assert_eq!(stream.state(), ImageLoadState::Loading(Some(chunk)));

    let image = pixel();
    assert!(completer.complete(image.clone()));
    assert_eq!(stream.state(), ImageLoadState::Ready(image.clone()));
    assert!(stream.is_completed());
    // A completed stream ignores later reports.
    assert!(!completer.report_chunk(chunk));
    assert!(!completer.fail(ImageError::InvalidDimensions));
    assert_eq!(stream.state(), ImageLoadState::Ready(image));
    assert_eq!(completer.stream(), stream);
}

#[test]
fn a_failed_stream_keeps_its_error() {
    let (stream, completer) = ImageStream::loading();
    assert!(completer.fail(ImageError::Decode("truncated".into())));
    assert_eq!(
        stream.state(),
        ImageLoadState::Failed(ImageError::Decode("truncated".into()))
    );
    assert!(!completer.complete(pixel()));
}

#[test]
fn synchronous_streams_are_complete_from_the_start() {
    let ready = ImageStream::ready(pixel());
    assert!(ready.is_completed());
    let failed = ImageStream::from_result(Err(ImageError::InvalidDimensions));
    assert!(failed.is_completed());
    assert_eq!(
        failed.state(),
        ImageLoadState::Failed(ImageError::InvalidDimensions)
    );
}

#[test]
fn default_resolve_loads_synchronously() {
    let png = {
        let mut bytes = Vec::new();
        let image = image::RgbaImage::from_pixel(2, 3, image::Rgba([0, 0, 0, 255]));
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode");
        bytes
    };
    let stream = MemoryImage::new(png).resolve(&ImageConfiguration::new());
    assert!(stream.is_completed());
    match stream.state() {
        ImageLoadState::Ready(image) => {
            assert_eq!((image.decoded().width(), image.decoded().height()), (2, 3));
        }
        other => panic!("expected a ready image, got {other:?}"),
    }
    let broken = MemoryImage::new(vec![1, 2, 3]).resolve(&ImageConfiguration::new());
    assert!(matches!(broken.state(), ImageLoadState::Failed(_)));
}

#[test]
fn erased_providers_compare_by_type_and_value() {
    let a = AnyImageProvider::new(MemoryImage::new(vec![1, 2, 3]));
    let same = AnyImageProvider::new(MemoryImage::new(vec![1, 2, 3]));
    let other_bytes = AnyImageProvider::new(MemoryImage::new(vec![4]));
    let other_type = AnyImageProvider::new(FileImage::new("a.png"));
    assert_eq!(a, same);
    assert_ne!(a, other_bytes);
    assert_ne!(a, other_type);
    assert_eq!(
        other_type.downcast_ref::<FileImage>().map(FileImage::path),
        Some(std::path::Path::new("a.png"))
    );
    assert!(a.downcast_ref::<FileImage>().is_none());
}

/// Flutter's `ImageStream.setCompleter`, used by `ScrollAwareImageProvider`
/// (scroll-008): a deferred stream hands over to the real resolution.
#[test]
fn a_loading_stream_follows_another_stream() {
    let (deferred, handoff) = ImageStream::loading();
    let (source, completer) = ImageStream::loading();
    assert!(handoff.follow(&source));
    assert_eq!(deferred.state(), ImageLoadState::Loading(None));
    assert!(!deferred.is_completed());
    // It now reports the source's progress and outcome, not its own.
    let chunk = ImageChunkEvent::new(3, None);
    completer.report_chunk(chunk);
    assert_eq!(deferred.state(), ImageLoadState::Loading(Some(chunk)));
    assert!(
        !handoff.complete(pixel()),
        "a following stream ignores reports"
    );
    let image = pixel();
    completer.complete(image.clone());
    assert_eq!(deferred.state(), ImageLoadState::Ready(image));
    assert!(deferred.is_completed());
    // It follows only once.
    assert!(!handoff.follow(&ImageStream::ready(pixel())));
}

#[test]
fn a_stream_cannot_follow_after_completing_or_in_a_cycle() {
    let (done, completer) = ImageStream::loading();
    completer.complete(pixel());
    assert!(!completer.follow(&ImageStream::ready(pixel())));
    assert!(done.is_completed());

    let (first, first_completer) = ImageStream::loading();
    let (second, second_completer) = ImageStream::loading();
    assert!(!first_completer.follow(&first), "not itself");
    assert!(second_completer.follow(&first));
    assert!(
        !first_completer.follow(&second),
        "not a stream following it"
    );
    assert_eq!(second.state(), ImageLoadState::Loading(None));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Ready(ImageHandle);

impl ImageProvider for Ready {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        Ok(self.0.clone())
    }
}

#[test]
fn erased_providers_load_through_their_provider() {
    let image = pixel();
    let provider = AnyImageProvider::new(Ready(image.clone()));
    assert_eq!(
        provider
            .load_with(&ImageConfiguration::new())
            .expect("loads"),
        image
    );
}

#[test]
fn listeners_catch_failed_provider_resolutions_even_if_the_error_already_exists() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct FailingProvider;
    impl ImageProvider for FailingProvider {
        fn load(&self) -> Result<ImageHandle, ImageError> {
            Err(ImageError::Decode("provider failed".into()))
        }
    }

    let result = std::rc::Rc::new(std::cell::RefCell::new(None));
    let stream = FailingProvider.resolve(&ImageConfiguration::empty());
    let _subscription = stream.listen_frame({
        let result = result.clone();
        move |outcome, synchronous| *result.borrow_mut() = Some((outcome, synchronous))
    });
    assert_eq!(
        result.take(),
        Some((Err(ImageError::Decode("provider failed".into())), true))
    );
}

#[test]
fn frame_listeners_follow_source_changes_without_replaying_progress_or_a_failure() {
    use std::{cell::RefCell, rc::Rc};

    let (outer, follow) = ImageStream::loading();
    let (source, finish) = ImageStream::loading();
    let mapped = outer.map_image(|_| panic!("failed images must not be transformed"));
    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = mapped.listen_frame({
        let events = events.clone();
        move |outcome, synchronous| events.borrow_mut().push((outcome, synchronous))
    });
    let canceled = mapped.listen_frame(|_, _| panic!("canceled handler ran"));
    drop(canceled);

    assert!(follow.follow(&source));
    assert!(finish.report_chunk(ImageChunkEvent::new(4, Some(10))));
    assert!(events.borrow().is_empty());
    assert!(finish.fail(ImageError::InvalidDimensions));
    assert_eq!(events.take(), [(Err(ImageError::InvalidDimensions), false)]);
    assert!(!finish.complete(pixel()));
    assert!(events.borrow().is_empty());
}

#[test]
fn frame_listener_distinguishes_synchronous_cache_hits_from_delayed_images() {
    use std::{cell::RefCell, rc::Rc};

    let image = pixel();
    let captures = Rc::new(RefCell::new(Vec::new()));
    let handler = || {
        let captures = captures.clone();
        move |image, synchronous| captures.borrow_mut().push((image, synchronous))
    };
    let _ready = ImageStream::ready(image.clone()).listen_frame(handler());
    assert_eq!(captures.take(), [(Ok(image.clone()), true)]);

    let (loading, completer) = ImageStream::loading();
    let _pending = loading.listen_frame(handler());
    assert!(captures.borrow().is_empty());
    assert!(completer.complete(image.clone()));
    assert_eq!(captures.take(), [(Ok(image), false)]);
}

#[test]
fn a_consumed_frame_listener_does_not_retain_decoded_pixels() {
    let image = pixel();
    let weak = image.downgrade();
    let (stream, completer) = ImageStream::loading();
    let _subscription = stream.listen_frame(|result, _| {
        assert_eq!(result.unwrap().decoded().pixels().as_ref(), [255, 255, 255, 255]);
    });
    drop(stream);
    assert!(completer.complete(image));
    drop(completer);
    assert!(weak.is_dropped());
}
