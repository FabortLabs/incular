use incular_image::{
    ImageChunkEvent, ImageConfiguration, ImageError, ImageHandle, ImageLoadState, ImageProvider,
    ImageResizeOptions, ImageStream, MAX_DECODE_IMAGE_DIMENSION, MemoryImage, ResizeImage,
    ResizeImagePolicy,
};

const UNIT: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_1x1.png"
);
const SQUARE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_50x50.png"
);
const PORTRAIT: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_50x100.png"
);
const LANDSCAPE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/resize_source_100x50.png"
);

fn size(image: &ImageHandle) -> (u32, u32) {
    (image.decoded().width(), image.decoded().height())
}

fn ready(stream: &ImageStream) -> ImageHandle {
    match stream.state() {
        ImageLoadState::Ready(image) => image,
        other => panic!("image not ready: {other:?}"),
    }
}

fn sized(raw: &'static [u8], w: Option<u32>, h: Option<u32>) -> ResizeImage {
    ResizeImage::new(MemoryImage::new(raw), w, h)
}

#[test]
fn exact_upscale_and_downscale_and_default_clamping() {
    assert_eq!(size(&MemoryImage::new(UNIT).load().unwrap()), (1, 1));
    assert_eq!(size(&MemoryImage::new(SQUARE).load().unwrap()), (50, 50));
    for (raw, requested, expected, upscale) in [
        (UNIT, (14, 7), (14, 7), true),
        (SQUARE, (25, 25), (25, 25), true),
        (UNIT, (50, 50), (1, 1), false),
        (PORTRAIT, (75, 90), (50, 90), false),
    ] {
        let provider = sized(raw, Some(requested.0), Some(requested.1)).allow_upscaling(upscale);
        let cfg = ImageConfiguration::new().size(incular_core::Size::new(
            requested.0 as f32,
            requested.1 as f32,
        ));
        let decoded = ready(&provider.resolve(&cfg));
        assert_eq!(size(&decoded), expected);
    }
}

#[test]
fn logical_size_uses_ceil_and_dpr_physical_size_and_default_no_upscale() {
    for (requested, dpr, expected, logical, fit, allow) in [
        ((25, 25), 2.0, (50, 50), true, false, true),
        ((25, 25), 3.0, (50, 50), true, false, false),
        ((25, 25), 2.0, (25, 25), false, false, false),
        ((12, 25), 2.0, (24, 24), true, true, false),
        ((3, 4), 1.25, (4, 5), true, false, false),
    ] {
        let mut provider = sized(SQUARE, Some(requested.0), Some(requested.1))
            .use_logical_size(logical)
            .allow_upscaling(allow);
        if fit {
            provider = provider.policy(ResizeImagePolicy::Fit);
        }
        assert_eq!(
            size(
                &provider
                    .load_with(&ImageConfiguration::new().device_pixel_ratio(dpr))
                    .unwrap()
            ),
            expected
        );
    }
}

type Bounds = (Option<u32>, Option<u32>);

fn fit_cases(raw: &'static [u8], upscale: bool, cases: &[(Bounds, (u32, u32))]) {
    for &((w, h), expected) in cases {
        let provider = sized(raw, w, h)
            .policy(ResizeImagePolicy::Fit)
            .allow_upscaling(upscale);
        assert_eq!(
            size(&ready(&provider.resolve(&ImageConfiguration::empty()))),
            expected,
            "{w:?}x{h:?}, upscale={upscale}"
        );
    }
}

#[test]
fn all_sixteen_no_upscale_fit_controls_decode_the_expected_size() {
    fit_cases(
        SQUARE,
        false,
        &[
            ((Some(25), Some(50)), (25, 25)),
            ((Some(50), Some(25)), (25, 25)),
            ((Some(25), Some(25)), (25, 25)),
            ((Some(25), None), (25, 25)),
            ((None, Some(25)), (25, 25)),
            ((Some(120), Some(100)), (50, 50)),
        ],
    );
    fit_cases(
        PORTRAIT,
        false,
        &[
            ((Some(25), Some(60)), (25, 50)),
            ((Some(60), Some(25)), (12, 25)),
            ((Some(25), Some(25)), (12, 25)),
            ((Some(25), None), (25, 50)),
            ((None, Some(25)), (12, 25)),
        ],
    );
    fit_cases(
        LANDSCAPE,
        false,
        &[
            ((Some(25), Some(60)), (25, 12)),
            ((Some(60), Some(25)), (50, 25)),
            ((Some(25), Some(25)), (25, 12)),
            ((Some(25), None), (25, 12)),
            ((None, Some(25)), (50, 25)),
        ],
    );
}

#[test]
fn all_sixteen_upscale_fit_controls_decode_the_expected_size() {
    fit_cases(
        SQUARE,
        true,
        &[
            ((Some(100), Some(200)), (100, 100)),
            ((Some(200), Some(100)), (100, 100)),
            ((Some(100), Some(100)), (100, 100)),
            ((Some(100), None), (100, 100)),
            ((None, Some(100)), (100, 100)),
            ((Some(25), Some(30)), (25, 25)),
        ],
    );
    fit_cases(
        PORTRAIT,
        true,
        &[
            ((Some(100), Some(250)), (100, 200)),
            ((Some(400), Some(200)), (100, 200)),
            ((Some(200), Some(200)), (100, 200)),
            ((Some(100), None), (100, 200)),
            ((None, Some(200)), (100, 200)),
        ],
    );
    fit_cases(
        LANDSCAPE,
        true,
        &[
            ((Some(200), Some(400)), (200, 100)),
            ((Some(250), Some(100)), (200, 100)),
            ((Some(200), Some(200)), (200, 100)),
            ((Some(200), None), (200, 100)),
            ((None, Some(100)), (200, 100)),
        ],
    );
}

#[test]
fn exact_single_height_rounds_but_width_and_fit_floor() {
    assert_eq!(
        size(&sized(PORTRAIT, None, Some(25)).load().unwrap()),
        (13, 25)
    );
    assert_eq!(
        size(&sized(LANDSCAPE, Some(25), None).load().unwrap()),
        (25, 12)
    );
    assert_eq!(
        size(
            &sized(PORTRAIT, None, Some(25))
                .policy(ResizeImagePolicy::Fit)
                .load()
                .unwrap()
        ),
        (12, 25)
    );
}

#[test]
fn invisible_neighbor_does_not_add_a_tinted_border() {
    let source = ImageHandle::from_rgba8(2, 1, [255, 0, 0, 255, 0, 0, 255, 0]).unwrap();
    let pixels = ImageResizeOptions {
        width: Some(1),
        height: Some(1),
        ..Default::default()
    }
    .resize(source, &ImageConfiguration::new())
    .unwrap();
    assert_eq!(pixels.decoded().pixels().as_ref(), [255, 0, 0, 128]);
}

#[test]
fn impossible_dimensions_fail_before_allocation() {
    assert_eq!(
        sized(SQUARE, Some(0), None).load().unwrap_err(),
        ImageError::InvalidDimensions
    );
    assert_eq!(
        sized(LANDSCAPE, Some(1), None).load().unwrap_err(),
        ImageError::InvalidDimensions
    );
    let too_wide = MAX_DECODE_IMAGE_DIMENSION + 1;
    assert!(
        matches!(sized(UNIT, Some(too_wide), Some(1)).allow_upscaling(true).load(),
        Err(ImageError::DecodeTooLarge { width, height: 1 }) if width == too_wide)
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Deferred(ImageStream);

impl ImageProvider for Deferred {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("resizing must follow the asynchronous stream");
    }

    fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
        self.0.clone()
    }
}

#[test]
fn a_pending_frame_keeps_progress_and_is_resized_once_when_ready() {
    let (base, complete) = ImageStream::loading();
    let provider = ResizeImage::new(Deferred(base), Some(25), Some(25));
    let output = provider.resolve(&ImageConfiguration::new());
    assert_eq!(output.state(), ImageLoadState::Loading(None));
    let progress = ImageChunkEvent::new(5, Some(10));
    assert!(complete.report_chunk(progress));
    assert_eq!(output.state(), ImageLoadState::Loading(Some(progress)));
    assert!(complete.complete(MemoryImage::new(SQUARE).load().unwrap()));
    assert!(output.is_completed());
    let decoded = ready(&output);
    assert_eq!(size(&decoded), (25, 25));
    assert_eq!(ready(&output).id(), decoded.id());
}

#[test]
fn resize_if_needed_does_not_wrap_a_provider_without_dimensions() {
    let result = ResizeImage::resize_if_needed(MemoryImage::new(UNIT), None, None, true);
    assert!(result.downcast_ref::<MemoryImage>().is_some());
    assert_eq!(
        size(&ready(&result.resolve(&ImageConfiguration::new()))),
        (1, 1)
    );
}

#[test]
fn resize_keys_use_effective_physical_dimensions_and_the_resolved_source() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let source = MemoryImage::new(SQUARE);
    let logical = ResizeImage::new(source.clone(), Some(25), Some(25)).use_logical_size(true);
    let physical = ResizeImage::new(source.clone(), Some(50), Some(50));
    let device = ImageConfiguration::new().device_pixel_ratio(2.0);
    let logical_key = logical.key_for(source.clone(), &device);
    let physical_key = physical.key_for(source.clone(), &device);
    let hash = |key: &incular_image::ResizeImageKey<MemoryImage>| {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(logical_key, physical_key);
    assert_eq!(hash(&logical_key), hash(&physical_key));
    assert_ne!(
        logical.key_for(source.clone(), &ImageConfiguration::new()),
        logical_key
    );
    assert_ne!(
        logical.key_for(MemoryImage::new(PORTRAIT), &device),
        logical_key
    );
    assert_ne!(
        logical.clone(),
        physical.clone(),
        "providers differ even when a resolved key agrees"
    );
    assert_eq!(logical.clone(), logical);
    assert_ne!(
        physical.clone(),
        ResizeImage::new(source, Some(150), Some(50))
    );
}

#[test]
fn bundle_cache_only_retains_resized_frames_and_reuses_equivalent_requests() {
    use incular_assets::{AssetBundle, AssetError};
    use incular_image::{ExactAssetImage, shared_bundle_image_cache};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Default)]
    struct Counting(Arc<AtomicUsize>);
    impl AssetBundle for Counting {
        fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
            assert_eq!(key, "photo.png", "exact provider never loads a manifest");
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::from(SQUARE))
        }
    }

    let bundle = Counting::default();
    let provider = ExactAssetImage::new("photo.png")
        .bundle(Arc::new(bundle.clone()))
        .scale(2.0);
    let cache = shared_bundle_image_cache();
    cache.clear();
    let device = ImageConfiguration::new().device_pixel_ratio(2.0);
    let logical = ResizeImage::new(provider.clone(), Some(8), Some(10)).use_logical_size(true);
    let physical = ResizeImage::new(provider.clone(), Some(16), Some(20));
    let pixels = logical.load_with(&device).unwrap();
    assert_eq!(size(&pixels), (16, 20));
    assert_eq!(pixels.scale(), 2.0);
    assert_eq!(cache.retained(), (1, 16 * 20 * 4));
    let warm = ready(&physical.resolve(&ImageConfiguration::new()));
    assert_eq!(warm.id(), pixels.id());
    assert_eq!(bundle.0.load(Ordering::Relaxed), 1);
    let different = ready(&logical.resolve(&ImageConfiguration::new()));
    assert_ne!(different.id(), warm.id());
    assert_eq!(size(&different), (8, 10));
    assert_eq!(cache.retained(), (2, (16 * 20 + 8 * 10) * 4));

    let full_size = provider.load().unwrap();
    assert_eq!(size(&full_size), (50, 50));
    assert_ne!(full_size.id(), warm.id());
    assert_eq!(cache.retained(), (3, (50 * 50 + 16 * 20 + 8 * 10) * 4));
    cache.clear();
}
