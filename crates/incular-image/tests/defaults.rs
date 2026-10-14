use incular_image::{ImageCache, ImageCacheLimits, ImageConfiguration};

#[test]
fn image_configuration_default_preserves_public_values() {
    let configuration = ImageConfiguration::default();

    assert_eq!(configuration.size, None);
    assert_eq!(configuration.device_pixel_ratio, 1.0);
    assert_eq!(configuration.text_direction, None);
    assert_eq!(configuration.locale, None);
    assert_eq!(ImageConfiguration::new(), configuration);
}

#[test]
fn image_cache_limits_default_preserves_public_budget() {
    let limits = ImageCacheLimits::default();

    assert_eq!(limits.max_entries, 1000);
    assert_eq!(limits.max_bytes, 100 * 1024 * 1024);
    assert_eq!(ImageCache::new().limits(), limits);
}
