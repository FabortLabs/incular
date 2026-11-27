use std::{cell::RefCell, rc::Rc, sync::Arc};

use incular_assets::MemoryAssetBundle;
use incular_image::{
    ExactAssetImage, FileImage, ImageConfiguration, ImageHandle, ImageProvider, ImageStream,
    ImageStreamListener, MemoryImage, ResizeImage,
};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);
const IMAGE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

#[test]
fn file_provider_reports_its_selected_path() {
    let file = FileImage::new(FIXTURE);
    let result = file.resolve(&ImageConfiguration::empty());
    assert_eq!(result.debug_label().as_deref(), Some(FIXTURE));
    assert!(result.is_completed());
}

#[test]
fn memory_provider_labels_its_byte_identity_on_warm_and_cold_streams() {
    let provider = MemoryImage::new(Arc::<[u8]>::from(IMAGE));
    let config = ImageConfiguration::empty();
    let label = provider.resolve(&config).debug_label().unwrap();
    let identity = label
        .strip_prefix("MemoryImage(Uint8List#")
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    assert_eq!(identity.len(), 5);
    assert!(identity.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        provider
            .clone()
            .resolve_cached(&config, None)
            .unwrap()
            .debug_label(),
        Some(label)
    );
}

#[test]
fn exact_asset_provider_labels_the_resolved_bundle_key() {
    let name = "images/blue.png";
    let provider =
        ExactAssetImage::new(name).bundle(Arc::new(MemoryAssetBundle::new().with(name, IMAGE)));
    assert_eq!(
        provider
            .resolve(&ImageConfiguration::empty())
            .debug_label()
            .as_deref(),
        Some(name)
    );
}

#[test]
fn resizing_keeps_source_label_and_adds_requested_dimensions() {
    let source = MemoryImage::new(Arc::<[u8]>::from(IMAGE));
    let config = ImageConfiguration::empty();
    let original = source.resolve(&config);
    let label = original.debug_label().unwrap();

    let resized = ResizeImage::new(source, Some(40), Some(40)).resolve(&config);
    let expected = format!("{label} - Resized(40×40)");
    assert_eq!(resized.debug_label().as_deref(), Some(expected.as_str()));
    assert_eq!(original.debug_label(), Some(label));

    let received = Rc::new(RefCell::new(None));
    let recorded = Rc::clone(&received);
    let _binding = resized.listen(ImageStreamListener::new(Rc::new(
        move |info, synchronous| {
            assert!(synchronous);
            *recorded.borrow_mut() = info.debug_label().map(str::to_owned);
        },
    )));
    assert_eq!(*received.borrow(), Some(expected));
}

#[test]
fn deferred_stream_listeners_see_labels_after_follow_and_mapping() {
    let (stream, completer) = ImageStream::loading();
    let (source, publisher) = ImageStream::loading();
    let mapped = source.with_debug_label("deferred").map_image(Ok);
    let seen = Rc::new(RefCell::new(None));
    let recorded = Rc::clone(&seen);
    let _binding = stream.listen(ImageStreamListener::new(Rc::new(
        move |info, synchronous| {
            assert!(!synchronous);
            *recorded.borrow_mut() = info.debug_label().map(str::to_owned);
        },
    )));
    assert!(completer.follow(&mapped));
    assert_eq!(stream.debug_label().as_deref(), Some("deferred"));
    assert!(publisher.complete(ImageHandle::from_bytes(IMAGE).unwrap()));
    assert_eq!(seen.borrow().as_deref(), Some("deferred"));
}
