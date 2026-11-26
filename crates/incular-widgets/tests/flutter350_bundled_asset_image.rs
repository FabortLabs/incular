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
