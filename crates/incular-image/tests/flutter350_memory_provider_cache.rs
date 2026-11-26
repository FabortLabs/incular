use std::sync::Arc;

use incular_image::{
    ImageCache, ImageConfiguration, ImageHandle, ImageLoadState, ImageProvider, ImageStream,
    MemoryImage, clear_shared_memory_image_cache,
};

const PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

fn ready(stream: ImageStream) -> ImageHandle {
    let ImageLoadState::Ready(handle) = stream.state() else {
        panic!("valid PNG did not decode")
    };
    handle
}

#[test]
fn another_provider_using_the_same_bytes_evicts_the_shared_image() {
    clear_shared_memory_image_cache();
    let configuration = ImageConfiguration::empty();
    let bytes: Arc<[u8]> = Arc::from(PNG);
    let provider = MemoryImage::new(bytes.clone());
    let equivalent = MemoryImage::new(bytes);
    assert!(provider.resolve_cached(&configuration, None).is_none());
    let original = ready(provider.resolve(&configuration));
    assert_eq!(ready(equivalent.resolve(&configuration)).id(), original.id());
    assert!(equivalent.evict());
    assert!(!equivalent.evict());
    assert!(provider.resolve_cached(&configuration, None).is_none());

    let decoded_again = ready(provider.resolve(&configuration));
    assert_ne!(decoded_again.id(), original.id());
    assert_eq!(decoded_again.decoded().pixels(), original.decoded().pixels());
    clear_shared_memory_image_cache();
}

#[test]
fn eviction_respects_a_supplied_image_cache_without_touching_the_default() {
    clear_shared_memory_image_cache();
    let configuration = ImageConfiguration::empty();
    let provider = MemoryImage::new(PNG);
    let mut other_cache = ImageCache::default();
    let separate = ready(provider.resolve_in_cache(&mut other_cache));
    let shared = ready(provider.resolve(&configuration));
    assert_ne!(separate.id(), shared.id());
    assert_eq!(other_cache.len(), 1);
    assert_eq!(other_cache.resident_bytes(), PNG.len() + separate.size_bytes());

    assert!(provider.evict_from(&mut other_cache));
    assert!(!provider.evict_from(&mut other_cache));
    assert!(other_cache.is_empty());
    assert_eq!(other_cache.resident_bytes(), 0);
    assert_eq!(other_cache.diagnostics().evictions, 1);
    assert_eq!(
        ready(provider.resolve_cached(&configuration, None).unwrap()).id(),
        shared.id()
    );
    assert_eq!(separate.decoded().pixels(), shared.decoded().pixels());
    clear_shared_memory_image_cache();
}
