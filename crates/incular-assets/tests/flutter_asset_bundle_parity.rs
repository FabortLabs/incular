//! Observable equivalents of four pinned Flutter 3.50.0 asset bundle tests.
//! Reference: packages/flutter/test/services/asset_bundle_test.dart:59,178,197,234.

use incular_assets::{
    AssetBundle, AssetError, AssetFetchCallback, AssetFetcher, AssetResponse, CachingAssetBundle,
    MemoryAssetBundle, NetworkAssetBundle,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
struct Counting {
    calls: Arc<AtomicUsize>,
    inner: MemoryAssetBundle,
}
impl AssetBundle for Counting {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        if key == "one" {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.load(key)
    }
}
#[test]
fn flutter_asset_raw_and_text() {
    let calls = Arc::new(AtomicUsize::new(0));
    let b = CachingAssetBundle::new(Counting {
        calls: calls.clone(),
        inner: MemoryAssetBundle::new().with("one", vec![49]),
    });
    assert_eq!(&*b.load("one").unwrap(), &[49]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(&*b.load_string("one", true).unwrap(), "1");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        b.load_string("foo", true).unwrap_err(),
        AssetError::NotFound("foo".into())
    );
}
#[test]
fn flutter_asset_structured_cached() {
    let b = CachingAssetBundle::new(MemoryAssetBundle::new().with("one", vec![49]));
    assert_eq!(b.load_structured_data("one", |_| Ok::<_, &str>(1)), Ok(1));
    assert_eq!(b.load_structured_data("one", |_| Ok::<_, &str>(2)), Ok(1));
}
#[test]
fn flutter_asset_parser_exception() {
    let b = CachingAssetBundle::new(
        MemoryAssetBundle::new().with("AssetManifest.json", b"{}".to_vec()),
    );
    let r = b.load_structured_data("AssetManifest.json", |_| {
        Err::<String, _>("what do they say?")
    });
    assert_eq!(
        r,
        Err(AssetError::Parse {
            key: "AssetManifest.json".into(),
            message: "what do they say?".into()
        })
    );
    assert_eq!(
        b.load_structured_data("AssetManifest.json", |s| Ok::<_, &str>(s.to_string())),
        Ok("{}".into())
    );
}
struct Rejected;
impl AssetFetcher for Rejected {
    fn fetch(&self, url: &str, done: AssetFetchCallback) {
        assert_eq!(url, "http://example.org/key");
        done(Ok(AssetResponse::new(400, &b""[..])));
    }
}
#[test]
fn flutter_asset_http_error() {
    let b = NetworkAssetBundle::new("http://example.org/path", Rejected);
    let received = Arc::new(Mutex::new(None));
    let sink = received.clone();
    b.load_async("key", Box::new(move |r| *sink.lock().unwrap() = Some(r)));
    assert_eq!(
        received.lock().unwrap().take(),
        Some(Err(AssetError::Io {
            key: "key".into(),
            message: "HTTP status code: 400".into()
        }))
    );
}
