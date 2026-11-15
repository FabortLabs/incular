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
