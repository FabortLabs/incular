use incular_assets::{
    AssetBundle, AssetError, AssetManifestFormat, CachingAssetBundle, MemoryAssetBundle,
    replace_root_asset_bundle,
};
use incular_core::standard_message_codec::{MessageValue, StandardMessageCodec};
use incular_image::{
    AssetBundleImageKey, AssetImage, ExactAssetImage, ImageConfiguration, ImageError,
    ImageProvider, ImageSource,
};
use std::{
    collections::HashSet,
    sync::{Arc, mpsc},
    time::Duration,
};

const MAIN: &str = "assets/normalFolder/normalFile.png";
const VARIANT: &str = "assets/normalFolder/3.0x/normalFile.png";

fn row(asset: &str, dpr: Option<f64>) -> MessageValue {
    let mut fields = vec![(
        MessageValue::String("asset".into()),
        MessageValue::String(asset.into()),
    )];
    if let Some(dpr) = dpr {
        fields.push((MessageValue::String("dpr".into()), MessageValue::Float(dpr)));
    }
    MessageValue::Map(fields)
}

fn manifest(rows: &[(&str, Vec<MessageValue>)]) -> Vec<u8> {
    StandardMessageCodec::encode_message(&MessageValue::Map(
        rows.iter()
            .map(|(key, variants)| {
                (
                    MessageValue::String((*key).into()),
                    MessageValue::List(variants.clone()),
                )
            })
            .collect(),
    ))
    .unwrap()
    .unwrap()
}

fn bundle(rows: &[(&str, Vec<MessageValue>)]) -> Arc<dyn AssetBundle> {
    Arc::new(CachingAssetBundle::new(
        MemoryAssetBundle::new().with("AssetManifest.bin", manifest(rows)),
    ))
}

fn lookup(image: &AssetImage, cfg: &ImageConfiguration) -> Result<AssetBundleImageKey, ImageError> {
    let (tx, rx) = mpsc::channel();
    image.obtain_key(cfg, Box::new(move |key| tx.send(key).unwrap()));
    rx.recv_timeout(Duration::from_secs(3)).unwrap()
}

fn assert_key(key: &AssetBundleImageKey, expected_name: &str, expected_scale: f64) {
    assert_eq!(key.name, expected_name);
    assert_eq!(key.scale, expected_scale);
}

#[test]
fn all_six_1x_path_spellings_have_no_implied_density() {
    let bundle = bundle(&[]);
    for path in [
        MAIN,
        "assets/parentFolder/3.0x/normalFile.png",
        "assets/parentFolder/__3.0x__/leafFolder/normalFile.png",
        "assets/parentFolder/__3.0x_leaf_folder_/normalFile.png",
        "assets/parentFolder/__3.0x__/leafFolder/normalFile.png", // duplicate pinned registration
        "assets/parentFolder/3.0x/leafFolder/normalFile.png",
    ] {
        let image = AssetImage::new(path).bundle(bundle.clone());
        let key = lookup(&image, &ImageConfiguration::empty()).unwrap();
        assert_key(&key, path, 1.0);
    }
}

#[test]
fn variant_only_entry_is_picked_with_dpr_but_not_for_an_empty_configuration() {
    let bundle = bundle(&[(MAIN, vec![row(VARIANT, Some(3.0))])]);
    let image = AssetImage::new(MAIN).bundle(bundle.clone());
    assert_key(
        &lookup(&image, &ImageConfiguration::empty()).unwrap(),
        MAIN,
        1.0,
    );
    assert_key(
        &lookup(
            &image,
            &ImageConfiguration::new()
                .bundle(bundle)
                .device_pixel_ratio(3.0),
        )
        .unwrap(),
        VARIANT,
        3.0,
    );
}

#[test]
fn empty_manifest_row_falls_back_to_the_main_key_even_at_high_dpr() {
    let image = AssetImage::new(MAIN).bundle(bundle(&[(MAIN, vec![])]));
    for config in [
        ImageConfiguration::empty(),
        ImageConfiguration::new().device_pixel_ratio(3.0),
    ] {
        assert_key(&lookup(&image, &config).unwrap(), MAIN, 1.0);
    }
}

#[test]
fn all_six_1x_3x_boundary_controls_select_the_exact_key_and_scale() {
    let bundle = bundle(&[(MAIN, vec![row(MAIN, None), row(VARIANT, Some(3.0))])]);
    let image = AssetImage::new(MAIN).bundle(bundle.clone());
    for (dpr, name, scale) in [
        (1.0, MAIN, 1.0),
        (3.0, VARIANT, 3.0),
        (2.0, MAIN, 1.0),
        (2.01, VARIANT, 3.0),
        (2.9, VARIANT, 3.0),
        (4.0, VARIANT, 3.0),
        (0.8, MAIN, 1.0),
        (1.0001, VARIANT, 3.0),
        (1.999, VARIANT, 3.0),
    ] {
        let config = ImageConfiguration::new()
            .bundle(bundle.clone())
            .device_pixel_ratio(dpr);
        assert_key(&lookup(&image, &config).unwrap(), name, scale);
    }
}

#[test]
fn package_logical_key_last_duplicate_and_own_bundle_priority() {
    const NAMED: &str = "packages/icons/mark.png";
    let root = bundle(&[(NAMED, vec![row("root.png", None)])]);
    let old = replace_root_asset_bundle(Some(root.clone()));
    let from_context = bundle(&[(NAMED, vec![row("context.png", None)])]);
    let explicit = bundle(&[(
        NAMED,
        vec![
            row("4x.png", Some(4.0)),
            row("original.png", None),
            row("later 1x.png", Some(1.0)),
        ],
    )]);
    let image = AssetImage::new("mark.png").package("icons");
    assert_eq!(image.key_name().unwrap(), NAMED);
    assert_eq!(image, AssetImage::new(NAMED));
    let resolved_from_root = lookup(&image, &ImageConfiguration::new()).unwrap();
    assert_key(&resolved_from_root, "root.png", 1.0);
    assert!(Arc::ptr_eq(resolved_from_root.bundle.shared(), &root));
    let config = ImageConfiguration::new().bundle(from_context.clone());
    let resolved_from_context = lookup(&image, &config).unwrap();
    assert_key(&resolved_from_context, "context.png", 1.0);
    assert!(Arc::ptr_eq(
        resolved_from_context.bundle.shared(),
        &from_context
    ));
    let explicit = image.bundle(explicit.clone());
    let resolved_from_explicit = lookup(&explicit, &config).unwrap();
    assert_key(&resolved_from_explicit, "later 1x.png", 1.0);
    let same_variant_other_ratio =
        lookup(&explicit, &config.clone().device_pixel_ratio(0.8)).unwrap();
    assert_eq!(same_variant_other_ratio, resolved_from_explicit);
    let other = AssetImage::new("mark.png")
        .package("icons")
        .bundle(bundle(&[(NAMED, vec![row("later 1x.png", None)])]));
    let distinct_bundle = lookup(&other, &config).unwrap();
    assert_ne!(distinct_bundle, resolved_from_explicit);
    assert_eq!(
        HashSet::from([
            distinct_bundle,
            resolved_from_explicit,
            same_variant_other_ratio
        ])
        .len(),
        2
    );
    replace_root_asset_bundle(old);
}

#[test]
fn exact_asset_bypasses_an_unavailable_manifest() {
    let bundle = Arc::new(MemoryAssetBundle::new());
    let config = ImageConfiguration::new()
        .bundle(bundle)
        .device_pixel_ratio(3.0);
    let fixed = ExactAssetImage::new("marker.png")
        .scale(1.7)
        .package("icons");
    assert_key(&fixed.obtain_key(&config), "packages/icons/marker.png", 1.7);
    assert!(matches!(fixed.load_with(&config),
        Err(ImageError::Asset(AssetError::NotFound(key))) if key == "packages/icons/marker.png"));
    assert!(matches!(lookup(&AssetImage::new("marker.png"), &config),
        Err(ImageError::Asset(AssetError::NotFound(key))) if key == AssetManifestFormat::Binary.asset_key()));
}

#[test]
fn decoded_content_hash_variants_use_selected_physical_bytes_and_density() {
    const RED: &[u8] = include_bytes!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/hashed_asset_1x.png"
    );
    const BLUE: &[u8] = include_bytes!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/hashed_asset_2x.png"
    );
    const LOGICAL: &str = "assets/foo.png";
    const RED_KEY: &str = "assets/foo.9f64a747.png";
    const BLUE_KEY: &str = "assets/2.0x/foo.74f81fe1.png";
    let bundle = Arc::new(CachingAssetBundle::new(
        MemoryAssetBundle::new()
            .with(
                "AssetManifest.bin",
                manifest(&[(LOGICAL, vec![row(RED_KEY, None), row(BLUE_KEY, Some(2.0))])]),
            )
            .with(RED_KEY, RED)
            .with(BLUE_KEY, BLUE),
    ));
    let mut outcomes = Vec::new();
    for (ratio, key, width, height, pixel) in [
        (1.0, RED_KEY, 3, 2, [255, 0, 0, 255]),
        (2.0, BLUE_KEY, 6, 4, [0, 0, 255, 255]),
    ] {
        let provider = AssetImage::new(LOGICAL);
        let config = ImageConfiguration::new()
            .bundle(bundle.clone())
            .device_pixel_ratio(ratio);
        let handle = provider.load_with(&config).unwrap();
        assert_key(&lookup(&provider, &config).unwrap(), key, ratio);
        assert_eq!(handle.source(), &ImageSource::Asset(key.into()));
        assert_eq!(handle.scale(), ratio);
        assert_eq!(
            (handle.decoded().width(), handle.decoded().height()),
            (width, height)
        );
        assert_eq!(handle.decoded().pixels()[..4], pixel);
        outcomes.push((width as f64 / ratio, height as f64 / ratio));
    }
    assert_eq!(outcomes, [(3.0, 2.0), (3.0, 2.0)]);
}
