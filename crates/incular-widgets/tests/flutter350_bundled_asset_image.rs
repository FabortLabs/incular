use std::sync::Arc;

use incular_assets::{CachingAssetBundle, MemoryAssetBundle};
use incular_config::Constraints;
use incular_core::{
    Size,
    standard_message_codec::{MessageValue, StandardMessageCodec},
};
use incular_rendering::PaintCommand;
use incular_widgets::{
    DefaultAssetBundle, Image, MediaQuery, MediaQueryData, Widget, internal::WidgetTree,
};

const LOGICAL: &str = "assets/foo.png";
const RED: &str = "assets/foo.9f64a747.png";
const BLUE: &str = "assets/2.0x/foo.74f81fe1.png";
const RED_PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/hashed_asset_1x.png"
);
const BLUE_PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/hashed_asset_2x.png"
);

#[test]
fn bundled_1x_and_2x_choose_their_own_pixels_and_keep_logical_size() {
    use MessageValue::{Float, List, Map, String as Text};
    let manifest = StandardMessageCodec::encode_message(&Map(vec![(
        Text(LOGICAL.into()),
        List(vec![
            Map(vec![(Text("asset".into()), Text(RED.into()))]),
            Map(vec![
                (Text("asset".into()), Text(BLUE.into())),
                (Text("dpr".into()), Float(2.0)),
            ]),
        ]),
    )]))
    .unwrap()
    .unwrap();
    let outer = Arc::new(MemoryAssetBundle::new().with("AssetManifest.bin", [255]));
    let bundle = Arc::new(CachingAssetBundle::new(
        MemoryAssetBundle::new()
            .with("AssetManifest.bin", manifest)
            .with(RED, RED_PNG)
            .with(BLUE, BLUE_PNG),
    ));
    let screen = |ratio| -> Widget {
        DefaultAssetBundle::new(
            outer.clone(),
            DefaultAssetBundle::new(
                bundle.clone(),
                MediaQuery::new(
                    MediaQueryData {
                        scale_factor: ratio,
                        ..Default::default()
                    },
                    Image::asset(LOGICAL),
                ),
            ),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(screen(1.0)).unwrap();
    for (ratio, dimension, pixel) in [
        (1.0, Size::new(3.0, 2.0), [255, 0, 0, 255]),
        (2.0, Size::new(6.0, 4.0), [0, 0, 255, 255]),
        (1.0, Size::new(3.0, 2.0), [255, 0, 0, 255]),
    ] {
        tree.update(root, screen(ratio)).unwrap();
        tree.layout(Constraints::loose(Size::new(100.0, 100.0)))
            .unwrap();
        assert_eq!(tree.element_bounds(root).unwrap().size, Size::new(3.0, 2.0));
        let list = tree.paint();
        let images: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Image {
                    image,
                    source,
                    destination,
                    ..
                } => Some((image, source.size, destination.size)),
                _ => None,
            })
            .collect();
        assert_eq!(images.len(), 1);
        let (image, source, destination) = images[0];
        assert_eq!(source, dimension);
        assert_eq!(destination, Size::new(3.0, 2.0));
        assert_eq!(image.scale(), ratio);
        assert_eq!(image.decoded().pixels()[..4], pixel);
    }
}

const IMAGE: &str = "assets/image.png";
const DENSITIES: [(&str, Option<f64>, &[u8]); 6] = [
    (
        IMAGE,
        None,
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_48.png"
        ),
    ),
    (
        "assets/1.5x/image.png",
        Some(1.5),
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_72.png"
        ),
    ),
    (
        "assets/2.0x/image.png",
        Some(2.0),
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_96.png"
        ),
    ),
    (
        "assets/3.0x/image.png",
        Some(3.0),
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_144.png"
        ),
    ),
    (
        "assets/4.0x/image.png",
        Some(4.0),
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_192.png"
        ),
    ),
    (
        "assets/1.0x/image.png",
        Some(1.0),
        include_bytes!(
            "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_480.png"
        ),
    ),
];

fn density_bundle(indices: &[usize]) -> Arc<dyn incular_assets::AssetBundle> {
    use MessageValue::{Float, List, Map, String as Text};
    let variants = indices
        .iter()
        .map(|&index| {
            let (asset, dpr, _) = DENSITIES[index];
            let mut row = vec![(Text("asset".into()), Text(asset.into()))];
            if let Some(dpr) = dpr {
                row.push((Text("dpr".into()), Float(dpr)));
            }
            Map(row)
        })
        .collect();
    let data =
        StandardMessageCodec::encode_message(&Map(vec![(Text(IMAGE.into()), List(variants))]))
            .unwrap()
            .unwrap();
    let mut bundle = MemoryAssetBundle::new().with("AssetManifest.bin", data);
    for (key, _, bytes) in DENSITIES {
        bundle.insert(key, bytes);
    }
    Arc::new(CachingAssetBundle::new(bundle))
}

fn assert_density(
    bundle: &Arc<dyn incular_assets::AssetBundle>,
    target: f64,
    scale: f64,
    decoded_side: f32,
) {
    for explicit in [true, false] {
        let provider = Image::asset(IMAGE).exclude_from_semantics(true);
        let provider = if explicit {
            provider
                .width(200.0)
                .height(200.0)
                .fit(incular_widgets::internal::ImageFit::Fill)
        } else {
            provider
        };
        let media = MediaQueryData {
            scale_factor: target,
            ..Default::default()
        };
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(MediaQuery::new(media, DefaultAssetBundle::new(bundle.clone(), provider)).into())
            .unwrap();
        tree.layout(Constraints::loose(Size::new(500.0, 500.0)))
            .unwrap();
        let expected_side = if explicit {
            200.0
        } else {
            decoded_side / scale as f32
        };
        assert_eq!(
            tree.element_bounds(root).unwrap().size,
            Size::new(expected_side, expected_side)
        );
        let display = tree.paint();
        let (image, source, destination) = display
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::Image {
                    image,
                    source,
                    destination,
                    ..
                } => Some((image, source, destination)),
                _ => None,
            })
            .unwrap();
        assert_eq!(image.scale(), scale);
        assert_eq!(source.size, Size::new(decoded_side, decoded_side));
        assert_eq!(destination.size, Size::new(expected_side, expected_side));
    }
}

#[test]
fn seven_pinned_widget_dpr_cases_match_both_explicit_and_inferred_dimensions() {
    let bundle = density_bundle(&[0, 1, 2, 3, 4]);
    for (target, scale, side) in [
        (1.0, 1.0, 48.0),
        (0.5, 1.0, 48.0),
        (1.5, 1.5, 72.0),
        (1.75, 2.0, 96.0),
        (2.3, 2.0, 96.0),
        (3.7, 4.0, 192.0),
        (5.1, 4.0, 192.0),
    ] {
        assert_density(&bundle, target, scale, side);
    }
}

#[test]
fn explicit_1x_variant_determines_the_decoded_and_inferred_size() {
    // The original Flutter manifest omits the physical base from this row
    // but its bundle can still load that file. Do not infer or inject it.
    let bundle = density_bundle(&[5, 1, 2, 3, 4]);
    assert_density(&bundle, 1.0, 1.0, 480.0);
}

#[test]
fn low_resolution_asset_sets_clamp_both_sides_and_bias_high_for_small_dpr() {
    let bundle = density_bundle(&[0, 1]);
    for (target, scale, side) in [
        (0.25, 1.0, 48.0),
        (0.5, 1.0, 48.0),
        (0.75, 1.0, 48.0),
        (1.0, 1.0, 48.0),
        (1.20, 1.5, 72.0),
        (1.25, 1.5, 72.0),
        (1.5, 1.5, 72.0),
        (1.75, 1.5, 72.0),
        (2.0, 1.5, 72.0),
        (2.25, 1.5, 72.0),
        (10.0, 1.5, 72.0),
    ] {
        assert_density(&bundle, target, scale, side);
    }
}
