use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use incular_assets::{
    AssetBundle, AssetError, AssetLoadCallback, AssetManifest, AssetManifestFormat, AssetMetadata,
    CachingAssetBundle, MemoryAssetBundle,
};

const NORMAL: &str = include_str!("fixtures/flutter350_asset_manifest_normal.base64");
const HASHED: &str = include_str!("fixtures/flutter350_asset_manifest_hashed.base64");
const EXTRA: &str = include_str!("fixtures/flutter350_asset_manifest_extra.base64");
const NATIVE: AssetManifestFormat = AssetManifestFormat::Binary;
const WEB: AssetManifestFormat = AssetManifestFormat::WebJson;
const FOO: &str = "assets/foo.png";
const BAR: &str = "assets/bar.png";

fn binary(fixture: &str) -> Vec<u8> {
    STANDARD.decode(fixture.trim()).unwrap()
}

fn web(fixture: &str) -> Vec<u8> {
    serde_json::to_vec(fixture.trim()).unwrap()
}

fn receive<T>(receiver: mpsc::Receiver<T>) -> T {
    receiver.recv_timeout(Duration::from_secs(3)).unwrap()
}

fn load(
    bundle: Arc<dyn AssetBundle>,
    format: AssetManifestFormat,
) -> mpsc::Receiver<Result<AssetManifest, AssetError>> {
    let (send, receive) = mpsc::channel();
    AssetManifest::load_from_asset_bundle_with_format(
        bundle,
        format,
        Box::new(move |result| {
            send.send(result).unwrap();
        }),
    );
    receive
}

fn native_fixture(fixture: &str) -> AssetManifest {
    let bundle = Arc::new(MemoryAssetBundle::new().with(NATIVE.asset_key(), binary(fixture)));
    let (tx, rx) = mpsc::channel();
    AssetManifest::load_from_asset_bundle(
        bundle,
        Box::new(move |manifest| tx.send(manifest).unwrap()),
    );
    receive(rx).unwrap()
}

fn keys(manifest: &AssetManifest) -> Vec<String> {
    manifest
        .list_assets()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn metadata(key: &str, dpr: Option<f64>, main: bool) -> AssetMetadata {
    AssetMetadata {
        key: key.to_owned(),
        target_device_pixel_ratio: dpr,
        main,
    }
}

#[test]
fn load_from_bundle_correctly_parses_a_binary_asset_manifest() {
    let manifest = native_fixture(NORMAL);
    assert_eq!(keys(&manifest), [FOO, BAR]);

    let foo = manifest.get_asset_variants(FOO).unwrap().unwrap();
    assert_eq!(
        &*foo,
        [
            metadata(FOO, None, true),
            metadata("assets/2x/foo.png", Some(2.0), false)
        ]
    );
    let bar = manifest.get_asset_variants(BAR).unwrap().unwrap();
    assert_eq!(&*bar, [metadata(BAR, None, true)]);
}

#[test]
fn get_asset_variants_returns_none_if_key_not_in_manifest() {
    let manifest = native_fixture(NORMAL);
    assert!(
        manifest
            .get_asset_variants("invalid asset key")
            .unwrap()
            .is_none()
    );
    assert!(
        manifest
            .get_asset_variants("assets/2x/foo.png")
            .unwrap()
            .is_none()
    );
}

#[test]
fn get_asset_variants_marks_primary_1x_variant_as_main_when_content_hashed() {
    let manifest = native_fixture(HASHED);

    assert_eq!(
        &*manifest.get_asset_variants(FOO).unwrap().unwrap(),
        [
            metadata("assets/foo.9f64a747.png", None, true),
            metadata("assets/2.0x/foo.74f81fe1.png", Some(2.0), false),
        ]
    );
    assert_eq!(
        &*manifest
            .get_asset_variants("assets/sub dir/space image.png")
            .unwrap()
            .unwrap(),
        [metadata(
            "assets/sub dir/space image.1a2b3c4d.png",
            None,
            true
        ),]
    );
}

#[test]
fn web_bundle_reads_the_json_string_envelope_with_the_same_wire_manifest() {
    let bundle = MemoryAssetBundle::new()
        .with(NATIVE.asset_key(), [255])
        .with(WEB.asset_key(), web(NORMAL));
    let manifest = receive(load(Arc::new(bundle), WEB)).unwrap();
    assert_eq!(keys(&manifest), [FOO, BAR]);
    assert_eq!(
        manifest.get_asset_variants(FOO).unwrap().unwrap()[1],
        metadata("assets/2x/foo.png", Some(2.0), false)
    );

    let escaped = format!(
        "\u{feff}\"\\u0044{}\"",
        NORMAL.trim()[1..].replace('=', "%3d")
    );
    let parsed = AssetManifest::from_bytes(escaped.as_bytes(), WEB).unwrap();
    assert_eq!(keys(&parsed), [FOO, BAR]);
}

#[test]
fn materialization_is_lazy_and_reorders_only_successfully_materialized_keys() {
    let manifest = native_fixture(EXTRA);
    let original = keys(&manifest);
    assert!(manifest.get_asset_variants("absent").unwrap().is_none());
    assert!(manifest.get_asset_variants("null").unwrap().is_none());
    assert!(
        matches!(manifest.get_asset_variants("bad"), Err(AssetError::Parse { key, message }) if key == NATIVE.asset_key() && message.contains("dpr"))
    );
    assert_eq!(keys(&manifest), original);
    let empty = manifest.get_asset_variants("empty").unwrap().unwrap();
    assert!(empty.is_empty());
    let other_handle = manifest.clone();
    assert!(Arc::ptr_eq(
        &empty,
        &other_handle.get_asset_variants("empty").unwrap().unwrap()
    ));
    assert_eq!(
        other_handle
            .get_asset_variants("web/raw")
            .unwrap()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        keys(&manifest),
        original
            .into_iter()
            .filter(|key| key != "empty" && key != "web/raw")
            .chain(["empty", "web/raw"].map(str::to_owned))
            .collect::<Vec<_>>()
    );
}

#[test]
fn hashes_obey_leftmost_exact_lowercase_and_source_map_suffix_rules() {
    let manifest = native_fixture(EXTRA);
    let original = keys(&manifest);
    for (i, key) in original.iter().take(9).enumerate() {
        assert_eq!(
            manifest.get_asset_variants(key).unwrap().unwrap()[0].main,
            i < 5,
            "{key}"
        );
    }
    assert!(manifest.get_asset_variants("emoji😃").unwrap().unwrap()[0].main);
}

#[derive(Default)]
struct DeferredReads {
    pending: Mutex<VecDeque<(String, AssetLoadCallback)>>,
}

impl DeferredReads {
    fn take(&self, format: AssetManifestFormat) -> AssetLoadCallback {
        let (key, resolve) = self.pending.lock().unwrap().pop_front().unwrap();
        assert_eq!(key, format.asset_key());
        resolve
    }

    fn count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }
}

struct Deferred(Arc<DeferredReads>);

impl AssetBundle for Deferred {
    fn load(&self, _: &str) -> Result<Arc<[u8]>, AssetError> {
        panic!("manifest must use the bundle's asynchronous transport");
    }

    fn load_async(&self, key: &str, done: AssetLoadCallback) {
        self.0
            .pending
            .lock()
            .unwrap()
            .push_back((key.to_owned(), done));
    }
}

#[test]
fn concurrent_cached_loads_share_one_inflight_request_and_warm_result() {
    for format in [NATIVE, WEB] {
        let reads = Arc::new(DeferredReads::default());
        let bundle = Arc::new(CachingAssetBundle::new(Deferred(reads.clone())));
        let first = load(bundle.clone(), format);
        let second = load(bundle.clone(), format);
        assert!(matches!(first.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(reads.count(), 1);
        let payload = if format == NATIVE {
            binary(NORMAL)
        } else {
            web(NORMAL)
        };
        reads.take(format)(Ok(Arc::from(payload)));
        let first = receive(first).unwrap();
        let variants = first.get_asset_variants(FOO).unwrap().unwrap();
        let second = receive(second).unwrap();
        let warm = load(bundle, format).try_recv().unwrap().unwrap();
        assert!(Arc::ptr_eq(
            &variants,
            &second.get_asset_variants(FOO).unwrap().unwrap()
        ));
        assert!(Arc::ptr_eq(
            &variants,
            &warm.get_asset_variants(FOO).unwrap().unwrap()
        ));
        assert_eq!(reads.count(), 0);
    }
}

#[test]
fn failures_reach_all_waiters_and_do_not_poison_the_cache() {
    let reads = Arc::new(DeferredReads::default());
    let bundle = Arc::new(CachingAssetBundle::new(Deferred(reads.clone())));
    let first = load(bundle.clone(), NATIVE);
    let shared = load(bundle.clone(), NATIVE);
    reads.take(NATIVE)(Ok(Arc::from([255])));
    assert!(matches!(receive(first), Err(AssetError::Parse { .. })));
    assert!(matches!(receive(shared), Err(AssetError::Parse { .. })));
    let retry = load(bundle.clone(), NATIVE);
    reads.take(NATIVE)(Ok(Arc::from(binary(NORMAL))));
    assert_eq!(keys(&receive(retry).unwrap()), [FOO, BAR]);
}

#[test]
fn invalidated_old_manifest_cannot_overwrite_a_new_request() {
    for all in [false, true] {
        let reads = Arc::new(DeferredReads::default());
        let bundle = Arc::new(CachingAssetBundle::new(Deferred(reads.clone())));
        let old = load(bundle.clone(), NATIVE);
        let complete_old = reads.take(NATIVE);
        if all {
            bundle.clear();
        } else {
            bundle.evict(NATIVE.asset_key());
        }
        let new = load(bundle.clone(), NATIVE);
        reads.take(NATIVE)(Ok(Arc::from(binary(HASHED))));
        complete_old(Ok(Arc::from(binary(NORMAL))));
        assert_eq!(keys(&receive(old).unwrap()), [FOO, BAR]);
        let expected = keys(&receive(new).unwrap());
        let warm = load(bundle, NATIVE).try_recv().unwrap().unwrap();
        assert_eq!(keys(&warm), expected);
        assert_eq!(expected, [FOO, "assets/sub dir/space image.png"]);
    }
}

#[test]
fn callbacks_may_reenter_and_the_unwrapped_bundle_does_not_cache() {
    let bundle = Arc::new(MemoryAssetBundle::new().with(NATIVE.asset_key(), binary(NORMAL)));
    let (tx, rx) = mpsc::channel();
    let reenter = bundle.clone();
    AssetManifest::load_from_asset_bundle(
        bundle,
        Box::new(move |first| {
            assert_eq!(
                first
                    .unwrap()
                    .get_asset_variants(FOO)
                    .unwrap()
                    .unwrap()
                    .len(),
                2
            );
            AssetManifest::load_from_asset_bundle(
                reenter,
                Box::new(move |second| {
                    tx.send(keys(&second.unwrap())).unwrap();
                }),
            );
        }),
    );
    assert_eq!(receive(rx), [FOO, BAR]);

    let bundle = Arc::new(CachingAssetBundle::new(
        MemoryAssetBundle::new().with(NATIVE.asset_key(), binary(NORMAL)),
    ));
    let (tx, rx) = mpsc::channel();
    let reenter = bundle.clone();
    AssetManifest::load_from_asset_bundle(
        bundle,
        Box::new(move |first| {
            assert_eq!(
                first
                    .unwrap()
                    .get_asset_variants(FOO)
                    .unwrap()
                    .unwrap()
                    .len(),
                2
            );
            AssetManifest::load_from_asset_bundle(
                reenter,
                Box::new(move |second| {
                    tx.send(keys(&second.unwrap())).unwrap();
                }),
            );
        }),
    );
    assert_eq!(receive(rx), [BAR, FOO]);
}

#[test]
fn malformed_envelopes_are_not_confused_with_an_empty_manifest() {
    for bad in [
        b"null".as_slice(),
        b"[]",
        b"\"AA\"",
        b"\"%%%=\"",
        b"\"/x==\"",
        b"\"AA==\" 0",
    ] {
        assert!(
            matches!(AssetManifest::from_bytes(bad, WEB), Err(AssetError::Parse { key, .. }) if key == WEB.asset_key())
        );
    }
    assert_eq!(
        AssetManifest::from_bytes(&[255], WEB).unwrap_err(),
        AssetError::InvalidUtf8(WEB.asset_key().into())
    );
    for bad in [&[12, 0][..], &[13, 1, 0, 0], &[13, 0, 0], &[0], &[]] {
        assert!(
            matches!(AssetManifest::from_bytes(bad, NATIVE), Err(AssetError::Parse { key, .. }) if key == NATIVE.asset_key())
        );
    }
    assert!(
        AssetManifest::from_bytes(&[13, 0], NATIVE)
            .unwrap()
            .list_assets()
            .is_empty()
    );
}
