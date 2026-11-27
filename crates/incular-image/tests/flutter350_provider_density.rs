use incular_image::{
    FileImage, ImageCache, ImageConfiguration, ImageHandle, ImageLoadState, ImageProvider,
    ImageStream, MemoryImage, ResizeImage,
};

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);
const IMAGE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

fn ready(stream: ImageStream) -> ImageHandle {
    match stream.state() {
        ImageLoadState::Ready(image) => image,
        unexpected => panic!("{unexpected:?}"),
    }
}

#[test]
fn memory_density_does_not_multiply_decodes_or_change_pixel_identity() {
    let provider = MemoryImage::new(IMAGE);
    let density2 = provider.clone().scale(2.);
    let density4 = provider.clone().scale(4.);
    assert_ne!(provider, density2);
    let mut cache = ImageCache::new();
    let original = ready(provider.resolve_in_cache(&mut cache));
    let two = ready(density2.resolve_in_cache(&mut cache));
    let four = ready(density4.resolve_in_cache(&mut cache));
    for (image, scale) in [(&original, 1.), (&two, 2.), (&four, 4.)] {
        assert_eq!((image.decoded().width(), image.decoded().height()), (8, 8));
        assert_eq!(image.scale(), scale);
        assert_eq!(image.id(), original.id());
        assert_eq!(image.downgrade().upgrade().unwrap().scale(), scale);
    }
    assert_ne!(
        original, two,
        "logical-density changes must not be interned away"
    );
    assert_eq!(cache.diagnostics().image_decodes, 1);
}

#[test]
fn file_density_and_resized_memory_keep_their_logical_scale() {
    let config = ImageConfiguration::empty();
    for (file, scale) in [
        (FileImage::new(FILE), 1.),
        (FileImage::new(FILE).scale(4.), 4.),
    ] {
        let image = ready(file.resolve(&config));
        assert_eq!(image.scale(), scale);
        assert_eq!((image.decoded().width(), image.decoded().height()), (8, 8));
    }
    let memory = ResizeImage::new(MemoryImage::new(IMAGE).scale(2.), Some(4), Some(4));
    let resized = ready(memory.resolve(&config));
    assert_eq!(resized.scale(), 2.);
    assert_eq!(
        (resized.decoded().width(), resized.decoded().height()),
        (4, 4)
    );
}
