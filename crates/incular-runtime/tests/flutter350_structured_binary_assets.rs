use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex},
};

use incular_assets::{AssetBundle, AssetError, CachingAssetBundle};
use incular_platform::message_codecs::{MessageValue, StandardMessageCodec};

#[derive(Default)]
struct Fixture {
    reads: Mutex<HashMap<String, usize>>,
}

impl AssetBundle for Fixture {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        let mut reads = self.reads.lock().unwrap();
        let count = reads.entry(key.to_owned()).or_default();
        *count += 1;
        match key {
            "counter" => Ok(Arc::from(count.to_string().into_bytes())),
            "one" => Ok(Arc::from(&b"1"[..])),
            "AssetManifest.json" => Ok(Arc::from(&br#"{"one":["one"]}"#[..])),
            "AssetManifest.bin" => {
                // Flutter StandardMessageCodec: a one-item map,
                // three-byte string "one", empty list.
                Ok(Arc::from(&[13, 1, 7, 3, b'o', b'n', b'e', 12, 0][..]))
            }
            _ => Err(AssetError::NotFound(key.into())),
        }
    }
}

fn json(bundle: &CachingAssetBundle<Fixture>, result: &'static str) -> &'static str {
    bundle
        .load_structured_data("AssetManifest.json", |text| {
            assert!(text.contains("one"));
            Ok::<_, Infallible>(result)
        })
        .unwrap()
}

fn binary(bundle: &CachingAssetBundle<Fixture>, result: &'static str) -> &'static str {
    bundle
        .load_structured_binary_data("AssetManifest.bin", |data| {
            assert_eq!(data.first(), Some(&13));
            Ok::<_, Infallible>(result)
        })
        .unwrap()
}

#[test]
fn both_structured_caches_and_strings_return_the_original_objects() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    for result in ["one", "two"] {
        assert_eq!(&*bundle.load_string("counter", true).unwrap(), "1");
        assert_eq!(json(&bundle, result), "one");
        assert_eq!(binary(&bundle, result), "one");
    }
}

#[test]
fn clear_reloads_the_string_and_reparses_both_structured_formats() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    assert_eq!(&*bundle.load_string("counter", true).unwrap(), "1");
    assert_eq!(json(&bundle, "one"), "one");
    assert_eq!(binary(&bundle, "one"), "one");
    bundle.clear();
    assert_eq!(&*bundle.load_string("counter", true).unwrap(), "2");
    assert_eq!(json(&bundle, "two"), "two");
    assert_eq!(binary(&bundle, "two"), "two");
}

#[test]
fn eviction_invalidates_only_the_selected_key_in_all_three_namespaces() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    for key in ["one", "counter"] {
        bundle.load_string(key, true).unwrap();
    }
    assert_eq!(json(&bundle, "one"), "one");
    assert_eq!(binary(&bundle, "one"), "one");
    bundle.evict("counter");
    assert_eq!(&*bundle.load_string("counter", true).unwrap(), "2");
    assert_eq!(json(&bundle, "two"), "one");
    assert_eq!(binary(&bundle, "two"), "one");
    bundle.evict("AssetManifest.json");
    assert_eq!(json(&bundle, "two"), "two");
    assert_eq!(binary(&bundle, "two"), "one");
    bundle.evict("AssetManifest.bin");
    assert_eq!(binary(&bundle, "two"), "two");
    assert_eq!(&*bundle.load_string("one", true).unwrap(), "1");
    assert_eq!(&*bundle.load_string("counter", true).unwrap(), "2");
}

#[test]
fn a_binary_hit_immediately_returns_the_existing_value_without_calling_the_parser() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    let binary = |n| {
        bundle
            .load_structured_binary_data("one", |_| Ok::<_, Infallible>(n))
            .unwrap()
    };
    let text = |n| {
        bundle
            .load_structured_data("one", |_| Ok::<_, Infallible>(n))
            .unwrap()
    };
    assert_eq!(binary(1), 1);
    assert_eq!(text("text"), "text");
    assert_eq!(binary(2), 1);
    assert_eq!(text("other"), "text");
    let mismatch = bundle.load_structured_binary_data("one", |_| Ok::<_, Infallible>(true));
    assert!(matches!(mismatch, Err(AssetError::TypeMismatch { key }) if key == "one"));
}

#[test]
fn a_binary_parse_failure_reports_its_asset_and_allows_a_successful_retry() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    let failure = bundle
        .load_structured_binary_data::<String, _>("AssetManifest.bin", |_| Err("buy more crystals"))
        .unwrap_err();
    assert!(
        matches!(failure, AssetError::Parse { key, message } if key == "AssetManifest.bin" && message == "buy more crystals")
    );
    assert_eq!(binary(&bundle, "recovered"), "recovered");
}

#[test]
fn the_platform_codec_reads_the_original_asset_manifest_bytes() {
    let bundle = CachingAssetBundle::new(Fixture::default());
    let decode = |bytes: &[u8]| StandardMessageCodec::decode_message(Some(bytes));
    let map = bundle
        .load_structured_binary_data("AssetManifest.bin", decode)
        .unwrap();
    assert_eq!(
        map,
        MessageValue::Map(vec![(
            MessageValue::String("one".into()),
            MessageValue::List(vec![])
        )])
    );
    let as_asset: &dyn AssetBundle = &bundle;
    assert_eq!(
        as_asset
            .load_structured_binary_data("AssetManifest.bin", decode)
            .unwrap(),
        map
    );
}
