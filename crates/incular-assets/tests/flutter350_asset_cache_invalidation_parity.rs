//! Text-cache behavior from Flutter 3.50.0 services/asset_bundle_test.dart.
//! Structured binary parsing has no public Incular counterpart.

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use incular_assets::{AssetBundle, AssetError, CachingAssetBundle};

const MANIFEST: &str = "AssetManifest.json";

#[derive(Default)]
struct TestAssets {
    counter: AtomicUsize,
}

impl AssetBundle for TestAssets {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        match key {
            "counter" => {
                let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
                Ok(count.to_string().into_bytes().into())
            }
            MANIFEST => Ok(Arc::from(&br#"{"one":["one"]}"#[..])),
            _ => Err(AssetError::NotFound(key.to_owned())),
        }
    }
}

fn counter(bundle: &CachingAssetBundle<TestAssets>) -> Arc<str> {
    bundle.load_string("counter", true).unwrap()
}

fn parsed(bundle: &CachingAssetBundle<TestAssets>, value: &'static str) -> &'static str {
    bundle
        .load_structured_data(MANIFEST, |_| Ok::<_, Infallible>(value))
        .unwrap()
}

// Upstream :116, text and structured-text portions. The caches are both
// populated before a single clear; both sources must be evaluated again.
#[test]
fn clear_invalidates_cached_strings_and_parsed_text() {
    let bundle = CachingAssetBundle::new(TestAssets::default());
    assert_eq!(&*counter(&bundle), "1");
    assert_eq!(parsed(&bundle, "one"), "one");
    assert_eq!(&*counter(&bundle), "1");
    assert_eq!(parsed(&bundle, "ignored"), "one");

    bundle.clear();

    assert_eq!(&*counter(&bundle), "2");
    assert_eq!(parsed(&bundle, "two"), "two");
}

// Upstream :147, text and structured-text portions. Each eviction has an
// explicit unaffected-key control.
#[test]
fn evict_reloads_only_the_named_string_or_parsed_text_key() {
    let bundle = CachingAssetBundle::new(TestAssets::default());
    assert_eq!(&*counter(&bundle), "1");
    assert_eq!(parsed(&bundle, "one"), "one");

    bundle.evict("counter");
    assert_eq!(&*counter(&bundle), "2");
    assert_eq!(parsed(&bundle, "ignored"), "one");

    bundle.evict(MANIFEST);
    assert_eq!(&*counter(&bundle), "2");
    assert_eq!(parsed(&bundle, "two"), "two");
}
