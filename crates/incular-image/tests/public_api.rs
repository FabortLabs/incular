use std::sync::Arc;

use incular_config::TextDirection;
use incular_image::{ImageCache, ImageConfiguration, ImageHandle, ImageProvider, MemoryImage};

#[test]
fn generated_rgba_is_shared_and_validated() {
    let image = ImageHandle::from_rgba8(2, 1, Arc::<[u8]>::from([1, 2, 3, 4, 5, 6, 7, 8])).unwrap();
    assert_eq!(image.decoded().width(), 2);
    assert_eq!(image.clone().id(), image.id());
    assert!(ImageHandle::from_rgba8(2, 1, Arc::<[u8]>::from([0; 7])).is_err());
}

#[test]
fn rgba_decoder_preserves_channels_and_alpha_across_scanlines() {
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    let pixels = [12, 34, 56, 0, 78, 90, 123, 1, 4, 3, 2, 127, 1, 5, 6, 255];
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(&pixels, 2, 2, ExtendedColorType::Rgba8)
        .unwrap();
    let handle = ImageHandle::from_bytes(&encoded).unwrap();
    let decoded = handle.decoded();
    assert_eq!((decoded.width(), decoded.height()), (2, 2));
    assert_eq!(decoded.pixels().as_ref(), pixels);
}

#[test]
fn rgb_decoder_widens_to_opaque_rgba_across_scanlines() {
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    // 3x2 so each row is 9 bytes: no accidental 4-byte alignment.
    let rgb = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, //
        10, 11, 12, 13, 14, 15, 16, 17, 18,
    ];
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(&rgb, 3, 2, ExtendedColorType::Rgb8)
        .unwrap();
    let handle = ImageHandle::from_bytes(&encoded).unwrap();
    let decoded = handle.decoded();
    assert_eq!((decoded.width(), decoded.height()), (3, 2));
    let expected: Vec<u8> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[r, g, b]| [r, g, b, 255])
        .collect();
    assert_eq!(decoded.pixels().as_ref(), expected.as_slice());
}

#[test]
fn cache_reuses_a_successfully_decoded_payload() {
    // This 2x2 PNG is deliberately decoded through the public cache API.
    const PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6,
        0, 0, 0, 114, 182, 13, 36, 0, 0, 0, 24, 73, 68, 65, 84, 120, 156, 5, 193, 129, 1, 0, 0, 4,
        192, 160, 248, 220, 229, 83, 34, 105, 71, 226, 30, 63, 110, 6, 127, 180, 47, 0, 167, 0, 0,
        0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let mut cache = ImageCache::new();
    let first = cache.load_bytes(PNG).unwrap();
    let second = cache.load_bytes(PNG).unwrap();
    assert_eq!(first, second);
    assert_eq!(cache.diagnostics().image_decodes, 1);
    assert_eq!(cache.diagnostics().load_cache_hits, 1);
}

#[test]
fn memory_provider_decodes_on_demand() {
    const PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6,
        0, 0, 0, 114, 182, 13, 36, 0, 0, 0, 24, 73, 68, 65, 84, 120, 156, 5, 193, 129, 1, 0, 0, 4,
        192, 160, 248, 220, 229, 83, 34, 105, 71, 226, 30, 63, 110, 6, 127, 180, 47, 0, 167, 0, 0,
        0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let provider = MemoryImage::new(Arc::<[u8]>::from(PNG));
    assert_eq!(provider.load().unwrap().decoded().width(), 2);
}

#[test]
fn image_configuration_resolution() {
    let configuration = ImageConfiguration::new()
        .size(incular_core::Size {
            width: -10.,
            height: f32::NAN,
        })
        .device_pixel_ratio(f64::NAN)
        .text_direction(TextDirection::Rtl)
        .locale("ar".parse().expect("valid locale"))
        .normalized();
    assert_eq!(configuration.size, Some(incular_core::Size::ZERO));
    assert_eq!(configuration.device_pixel_ratio, 1.);
    assert_eq!(configuration.text_direction, Some(TextDirection::Rtl));
    assert_eq!(
        configuration.locale.as_ref().map(ToString::to_string),
        Some("ar".into())
    );
}

#[test]
fn provider_load_with_preserves_single_source_resolution() {
    let provider = MemoryImage::new(Arc::<[u8]>::from([
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6,
        0, 0, 0, 114, 182, 13, 36, 0, 0, 0, 24, 73, 68, 65, 84, 120, 156, 5, 193, 129, 1, 0, 0, 4,
        192, 160, 248, 220, 229, 83, 34, 105, 71, 226, 30, 63, 110, 6, 127, 180, 47, 0, 167, 0, 0,
        0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]));
    let configuration = ImageConfiguration::new().device_pixel_ratio(2.);
    assert_eq!(
        provider
            .load_with(&configuration)
            .unwrap()
            .decoded()
            .height(),
        2
    );
}

#[test]
fn weak_handle_reports_when_every_handle_is_dropped() {
    let image = ImageHandle::from_rgba8(1, 1, Arc::<[u8]>::from([1, 2, 3, 4])).unwrap();
    let weak = image.downgrade();
    let clone = image.clone();
    drop(image);
    assert!(!weak.is_dropped());
    assert_eq!(weak.upgrade().map(|handle| handle.id()), Some(clone.id()));
    drop(clone);
    assert!(weak.is_dropped());
    assert!(weak.upgrade().is_none());
}

#[test]
fn image_info_size_counts_four_bytes_per_decoded_pixel() {
    for (width, height, expected) in [(10, 20, 800), (20, 100, 8_000), (100, 200, 80_000)] {
        let image = ImageHandle::from_rgba8(width, height, vec![0_u8; expected]).unwrap();
        assert_eq!(image.size_bytes(), expected);
        assert_eq!(image.clone().size_bytes(), expected);
    }
}

#[test]
fn decoding_a_transparent_one_pixel_png_preserves_rgba() {
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(&[0; 4], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    let image = ImageHandle::from_bytes(encoded).unwrap();
    assert_eq!((image.decoded().width(), image.decoded().height()), (1, 1));
    assert_eq!(image.decoded().pixels().as_ref(), [0; 4]);
    assert_eq!(image.size_bytes(), 4);
}
