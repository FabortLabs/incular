//! Hot-reload invalidation hooks, gated behind `hot-reload`.
#![cfg(feature = "hot-reload")]

use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicUsize, Ordering},
};

use incular_assets::{
    AssetBundle, AssetError, CachingAssetBundle, MemoryAssetBundle, PathOverrideBundle,
    hot_reload::{self, AssetInvalidation},
};

/// A shared, mutable in-memory bundle so tests can change a source after
/// caching and observe reloading.
#[derive(Clone, Default)]
struct Shared {
    inner: Arc<Mutex<MemoryAssetBundle>>,
    loads: Arc<AtomicUsize>,
}

impl Shared {
    fn set(&self, key: &str, bytes: &[u8]) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, bytes.to_vec());
    }
}

impl AssetBundle for Shared {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .load(key)
    }
}

#[test]
fn hot_reload_invalidate_drops_cached_assets_for_reloading() {
    let source = Shared::default();
    source.set("config", b"one");
    let bundle = CachingAssetBundle::new(source.clone());

    // Warm the cache.
    assert_eq!(&*bundle.load_string("config", true).unwrap(), "one");
    assert_eq!(source.loads.load(Ordering::SeqCst), 1);
    assert_eq!(&*bundle.load_string("config", true).unwrap(), "one");
    assert_eq!(source.loads.load(Ordering::SeqCst), 1, "cached");

    // The source changes; cached value is stale until invalidated.
    source.set("config", b"two");
    assert_eq!(&*bundle.load_string("config", true).unwrap(), "one");

    // `hot_reload_invalidate` (defaulting to `evict`) forces reload.
    bundle.hot_reload_invalidate("config");
    assert_eq!(&*bundle.load_string("config", true).unwrap(), "two");
    assert_eq!(source.loads.load(Ordering::SeqCst), 2);

    // Invalidate everything.
    source.set("config", b"three");
    bundle.hot_reload_invalidate_all();
    assert_eq!(&*bundle.load_string("config", true).unwrap(), "three");
    assert_eq!(source.loads.load(Ordering::SeqCst), 3);
}

#[test]
fn invalidation_events_are_broadcast_to_subscribers() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let _sub = hot_reload::subscribe(move |invalidation| {
        sink.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(invalidation.clone());
    });

    hot_reload::invalidate_key("assets/a.txt");
    hot_reload::invalidate_path("assets/a.txt");
    hot_reload::invalidate(AssetInvalidation::all());
    hot_reload::invalidate_all();

    let got = events
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        got,
        vec![
            AssetInvalidation::Key("assets/a.txt".into()),
            AssetInvalidation::Path("assets/a.txt".into()),
            AssetInvalidation::All,
            AssetInvalidation::All,
        ]
    );

    // `affects_key` semantics.
    assert!(AssetInvalidation::key("a").affects_key("a"));
    assert!(!AssetInvalidation::key("a").affects_key("b"));
    assert!(AssetInvalidation::All.affects_key("anything"));
    assert!(AssetInvalidation::path("x").affects_key("anything"));
}

#[test]
fn dropping_subscription_stops_delivery_and_unsubscribe_is_explicit() {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    let sub = hot_reload::subscribe(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    hot_reload::invalidate_all();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    sub.unsubscribe();
    hot_reload::invalidate_all();
    assert_eq!(count.load(Ordering::SeqCst), 1);

    {
        let counter = count.clone();
        let _sub = hot_reload::subscribe(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        hot_reload::invalidate_all();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
    hot_reload::invalidate_all();
    assert_eq!(count.load(Ordering::SeqCst), 2, "drop unsubscribes");
}

#[test]
fn watch_invalidates_a_shared_bundle_automatically() {
    let source = Shared::default();
    source.set("k", b"v1");
    let bundle = Arc::new(CachingAssetBundle::new(source.clone()));

    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v1");
    source.set("k", b"v2");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v1", "cached");

    let watcher = hot_reload::watch(&bundle);
    hot_reload::invalidate_key("k");
    assert_eq!(
        &*bundle.load_string("k", true).unwrap(),
        "v2",
        "key invalidation reloads"
    );

    // `Path` and `All` conservatively clear.
    source.set("k", b"v3");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v2");
    hot_reload::invalidate_path("k");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v3");

    source.set("k", b"v4");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v3");
    hot_reload::invalidate_all();
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v4");

    // Unrelated key invalidation does not drop our cached entry.
    source.set("k", b"v5");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v4");
    hot_reload::invalidate_key("other");
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v4");

    // Dropping the watcher stops delivery; a dead bundle is skipped.
    drop(watcher);
    source.set("k", b"v6");
    hot_reload::invalidate_all();
    assert_eq!(&*bundle.load_string("k", true).unwrap(), "v4");
}

#[test]
fn key_invalidation_resolves_path_overrides() {
    let source = Shared::default();
    source.set("physical", b"one");
    let bundle = Arc::new(
        PathOverrideBundle::new(CachingAssetBundle::new(source.clone()))
            .with_path_override("logical", "physical"),
    );

    assert_eq!(&*bundle.load_string("logical", true).unwrap(), "one");
    source.set("physical", b"two");
    assert_eq!(&*bundle.load_string("logical", true).unwrap(), "one");

    let _watcher = hot_reload::watch(&bundle);
    // Invalidate by logical key; the physical cached entry is evicted too.
    hot_reload::invalidate_key("logical");
    assert_eq!(&*bundle.load_string("logical", true).unwrap(), "two");

    // Invalidate by physical key as broadcast.
    source.set("physical", b"three");
    assert_eq!(&*bundle.load_string("logical", true).unwrap(), "two");
    hot_reload::invalidate_key("physical");
    assert_eq!(&*bundle.load_string("logical", true).unwrap(), "three");
}

#[test]
fn reentrant_invalidation_from_a_hook_does_not_deadlock() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    let _sub = hot_reload::subscribe(move |invalidation| {
        hits2.fetch_add(1, Ordering::SeqCst);
        if matches!(invalidation, AssetInvalidation::Key(_)) {
            // Re-entering while notified is allowed.
            hot_reload::invalidate_all();
        }
    });
    hot_reload::invalidate_key("x");
    assert!(hits.load(Ordering::SeqCst) >= 2);
}
