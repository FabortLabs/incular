//! Flutter's `NetworkAssetBundle` (`services/asset_bundle.dart`,
//! support-005): keys resolve against a base URL as `Uri.resolve` does, a
//! `200 OK` response is the asset, and any other status fails with its code.
//! The transport is an injected fetcher, and the asynchronous load never
//! blocks.

use std::sync::{Arc, Mutex};

use incular_assets::{
    AssetBundle, AssetError, AssetFetchCallback, AssetFetcher, AssetResponse, CachingAssetBundle,
    MemoryAssetBundle, NetworkAssetBundle, resolve_url,
};

type Pending = Arc<Mutex<Vec<(String, AssetFetchCallback)>>>;

/// A fetcher that records each request and answers when the test says so.
#[derive(Clone, Default)]
struct Deferred {
    pending: Pending,
}

impl AssetFetcher for Deferred {
    fn fetch(&self, url: &str, done: AssetFetchCallback) {
        self.pending.lock().unwrap().push((url.to_owned(), done));
    }
}

/// A fetcher that can also block, answering every URL with its own text.
struct Echo;

impl AssetFetcher for Echo {
    fn fetch(&self, url: &str, done: AssetFetchCallback) {
        done(Ok(AssetResponse::new(200, url.as_bytes())));
    }

    fn fetch_blocking(&self, url: &str) -> Option<Result<AssetResponse, String>> {
        Some(Ok(AssetResponse::new(200, url.as_bytes())))
    }
}

type Outcome = Arc<Mutex<Option<Result<Arc<[u8]>, AssetError>>>>;

fn load_async(bundle: &dyn AssetBundle, key: &str) -> Outcome {
    let outcome: Outcome = Arc::default();
    let sink = outcome.clone();
    bundle.load_async(
        key,
        Box::new(move |result| *sink.lock().unwrap() = Some(result)),
    );
    outcome
}

#[test]
fn keys_resolve_like_dart_uri_resolve() {
    let base = "https://example.com/app/index.html";
    assert_eq!(resolve_url(base, "a.png"), "https://example.com/app/a.png");
    assert_eq!(
        resolve_url("https://example.com/app/", "assets/a.png"),
        "https://example.com/app/assets/a.png"
    );
    assert_eq!(resolve_url(base, "../b.txt"), "https://example.com/b.txt");
    assert_eq!(
        resolve_url(base, "./c/./d/../e"),
        "https://example.com/app/c/e"
    );
    assert_eq!(
        resolve_url(base, "/root.txt"),
        "https://example.com/root.txt"
    );
    assert_eq!(
        resolve_url("https://example.com", "x"),
        "https://example.com/x"
    );
    assert_eq!(
        resolve_url(base, "//cdn.example.org/f.bin"),
        "https://cdn.example.org/f.bin"
    );
    assert_eq!(resolve_url(base, "http://other/g"), "http://other/g");
    assert_eq!(
        resolve_url(base, "q.json?v=2#top"),
        "https://example.com/app/q.json?v=2#top"
    );
    assert_eq!(
        resolve_url("https://example.com/app/?x=1", ""),
        "https://example.com/app/?x=1"
    );
    assert_eq!(
        resolve_url(base, "a b/ü.png"),
        "https://example.com/app/a%20b/%C3%BC.png"
    );
}

#[test]
fn an_asynchronous_load_answers_when_the_fetch_completes() {
    let fetcher = Deferred::default();
    let bundle = NetworkAssetBundle::new("https://example.com/assets/", fetcher.clone());
    let outcome = load_async(&bundle, "data/config.json");
    assert!(outcome.lock().unwrap().is_none(), "the load does not block");
    let (url, done) = fetcher.pending.lock().unwrap().pop().expect("one request");
    assert_eq!(url, "https://example.com/assets/data/config.json");
    done(Ok(AssetResponse::new(200, &b"{}"[..])));
    assert_eq!(
        outcome.lock().unwrap().take(),
        Some(Ok(Arc::from(&b"{}"[..])))
    );
}

#[test]
fn failures_report_the_status_code_or_the_transport_error() {
    let fetcher = Deferred::default();
    let bundle = NetworkAssetBundle::new("https://example.com/", fetcher.clone());
    let missing = load_async(&bundle, "missing.txt");
    let broken = load_async(&bundle, "broken.txt");
    let mut pending = fetcher.pending.lock().unwrap();
    let (_, broken_done) = pending.pop().expect("second request");
    let (_, missing_done) = pending.pop().expect("first request");
    drop(pending);
    missing_done(Ok(AssetResponse::new(404, &b"not found"[..])));
    broken_done(Err("connection reset".to_owned()));
    assert_eq!(
        missing.lock().unwrap().take(),
        Some(Err(AssetError::Io {
            key: "missing.txt".into(),
            message: "HTTP status code: 404".into(),
        }))
    );
    assert_eq!(
        broken.lock().unwrap().take(),
        Some(Err(AssetError::Io {
            key: "broken.txt".into(),
            message: "connection reset".into(),
        }))
    );
}

#[test]
fn a_synchronous_load_needs_a_fetcher_that_can_block() {
    let non_blocking = NetworkAssetBundle::new("https://example.com/", Deferred::default());
    assert!(matches!(
        non_blocking.load("a.txt"),
        Err(AssetError::Io { .. })
    ));

    let blocking = CachingAssetBundle::new(NetworkAssetBundle::new("https://example.com/x/", Echo));
    assert_eq!(
        &*blocking.load_string("y.txt", true).expect("loads"),
        "https://example.com/x/y.txt"
    );
    // Caching bundles forward the asynchronous load.
    let outcome = load_async(&blocking, "z.txt");
    assert_eq!(
        outcome.lock().unwrap().take(),
        Some(Ok(Arc::from(&b"https://example.com/x/z.txt"[..])))
    );
    assert_eq!(
        format!("{:?}", NetworkAssetBundle::new("https://e/", Echo)),
        "NetworkAssetBundle(\"https://e/\")"
    );
}

#[test]
fn other_bundles_load_asynchronously_through_their_synchronous_load() {
    let bundle = MemoryAssetBundle::new().with("k", &b"v"[..]);
    let outcome = load_async(&bundle, "k");
    assert_eq!(
        outcome.lock().unwrap().take(),
        Some(Ok(Arc::from(&b"v"[..])))
    );
}

#[test]
fn display_uses_the_bundle_identity_and_base_url_without_fetching() {
    let fetcher = Deferred::default();
    let expected_url = "http://example.org/path";
    let bundle = NetworkAssetBundle::new(expected_url, fetcher.clone());
    let description = bundle.to_string();
    let suffix = format!("({expected_url})");
    let hash = description
        .strip_prefix("NetworkAssetBundle#")
        .and_then(|rest| rest.strip_suffix(&suffix))
        .expect("single-line Flutter bundle description");
    assert_eq!(hash.len(), 5);
    assert!(hash.bytes().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(bundle.clone().to_string(), description);
    assert_ne!(
        NetworkAssetBundle::new(expected_url, fetcher.clone()).to_string(),
        description
    );
    assert!(fetcher.pending.lock().unwrap().is_empty());
}
