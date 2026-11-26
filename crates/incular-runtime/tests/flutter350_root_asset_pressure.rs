use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use incular_assets::{
    AssetBundle, AssetError, CachingAssetBundle, replace_root_asset_bundle, root_asset_bundle,
};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Application;
use incular_widgets::{Builder, DefaultAssetBundle, Widget};

#[derive(Clone, Default)]
struct CountingBundle(Arc<AtomicUsize>);

impl CountingBundle {
    fn reads(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

impl AssetBundle for CountingBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::from(key.as_bytes()))
    }
}

struct Override(Option<Arc<dyn AssetBundle>>);

impl Override {
    fn install(bundle: Arc<dyn AssetBundle>) -> Self {
        Self(replace_root_asset_bundle(Some(bundle)))
    }
}

impl Drop for Override {
    fn drop(&mut self) {
        replace_root_asset_bundle(self.0.take());
    }
}

fn leaf() -> Widget {
    Widget::box_(Size::new(20., 20.), Color::WHITE)
}

#[test]
fn platform_memory_pressure_clears_then_rewarms_the_root_asset_cache() {
    let source = CountingBundle::default();
    let root = Arc::new(CachingAssetBundle::new(source.clone()));
    let _override = Override::install(root);
    let app = Application::new(|_| leaf()).unwrap();

    let bundle = root_asset_bundle();
    let both = || {
        for key in ["test_asset", "test_asset2"] {
            assert_eq!(&*bundle.load_string(key, true).unwrap(), key);
        }
    };
    assert_eq!(
        &*bundle.load_string("test_asset", true).unwrap(),
        "test_asset"
    );
    assert_eq!(source.reads(), 1);
    assert_eq!(
        &*bundle.load_string("test_asset2", true).unwrap(),
        "test_asset2"
    );
    assert_eq!(source.reads(), 2);
    both();
    assert_eq!(source.reads(), 2);

    assert!(!app.handle_system_message("flutter/other", br#"{"type":"memoryPressure"}"#));
    assert!(!app.handle_system_message("flutter/system", br#"{"type":"other"}"#));
    assert!(!app.handle_system_message("flutter/system", b"{"));
    both();
    assert_eq!(source.reads(), 2);

    assert!(app.handle_system_message("flutter/system", br#"{"type":"memoryPressure"}"#));
    assert_eq!(
        &*bundle.load_string("test_asset", true).unwrap(),
        "test_asset"
    );
    assert_eq!(source.reads(), 3);
    assert_eq!(
        &*bundle.load_string("test_asset2", true).unwrap(),
        "test_asset2"
    );
    assert_eq!(source.reads(), 4);
    both();
    assert_eq!(source.reads(), 4);
}

#[test]
fn widgets_fall_back_to_the_same_root_that_the_native_warning_clears() {
    let source = CountingBundle::default();
    let root: Arc<dyn AssetBundle> = Arc::new(CachingAssetBundle::new(source.clone()));
    let _override = Override::install(root.clone());
    let child_source = CountingBundle::default();
    let child: Arc<dyn AssetBundle> = Arc::new(CachingAssetBundle::new(child_source.clone()));
    let mut app = Application::new({
        let root = root.clone();
        let child = child.clone();
        move |_| {
            let scoped = Builder::new({
                let child = child.clone();
                move |context| {
                    let from_context = DefaultAssetBundle::of(context);
                    assert!(Arc::ptr_eq(&from_context, &child));
                    from_context.load_string("child-text", true).unwrap();
                    leaf()
                }
            });
            Builder::new({
                let expected_root = root.clone();
                let child = child.clone();
                move |context| {
                    let from_context = DefaultAssetBundle::of(context);
                    assert!(Arc::ptr_eq(&from_context, &expected_root));
                    from_context.load_string("root-text", true).unwrap();
                    DefaultAssetBundle::new(child.clone(), scoped.clone()).into()
                }
            })
            .into()
        }
    })
    .unwrap();

    app.run_window_frame_at(
        app.primary_window(),
        Constraints::tight(Size::new(120., 80.)),
        Instant::now(),
    )
    .unwrap()
    .unwrap();
    assert_eq!((source.reads(), child_source.reads()), (1, 1));

    app.handle_memory_pressure();
    root.load_string("root-text", true).unwrap();
    child.load_string("child-text", true).unwrap();
    assert_eq!((source.reads(), child_source.reads()), (2, 1));
}

#[test]
fn native_warning_also_forgets_root_structured_asset_values() {
    let source = CountingBundle::default();
    let root = Arc::new(CachingAssetBundle::new(source.clone()));
    let _override = Override::install(root.clone());
    let app = Application::new(|_| leaf()).unwrap();
    let load = || {
        root.load_structured_data("structured", |text| {
            Ok::<_, std::convert::Infallible>(text.len())
        })
        .unwrap()
    };
    assert_eq!(load(), 10);
    assert_eq!(load(), 10);
    assert_eq!(source.reads(), 1);
    app.handle_memory_pressure();
    assert_eq!(load(), 10);
    assert_eq!(source.reads(), 2);
}

#[test]
fn memory_pressure_evicts_decoded_bundle_images_without_dropping_live_pixels() {
    use incular_assets::MemoryAssetBundle;
    use incular_image::{ExactAssetImage, ImageProvider, shared_bundle_image_cache};
    const PNG: &[u8] = include_bytes!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
    );
    let app = Application::new(|_| leaf()).unwrap();
    let cache = shared_bundle_image_cache();
    cache.clear();
    let bundle = Arc::new(MemoryAssetBundle::new().with("icon.png", PNG));
    let image = ExactAssetImage::new("icon.png").bundle(bundle);

    let first = image.load().unwrap();
    assert_eq!(cache.retained(), (1, 256));
    assert_eq!(image.load().unwrap().id(), first.id());
    assert!(!app.handle_system_message("flutter/other", br#"{"type":"memoryPressure"}"#));
    assert_eq!(cache.retained(), (1, 256));
    assert!(app.handle_system_message("flutter/system", br#"{"type":"memoryPressure"}"#));
    assert_eq!(cache.retained(), (0, 0));
    assert_eq!(first.decoded().pixels()[..4], [28, 94, 193, 255]);
    let second = image.load().unwrap();
    assert_ne!(second.id(), first.id());
    assert_eq!(cache.retained(), (1, 256));
    cache.clear();
}
