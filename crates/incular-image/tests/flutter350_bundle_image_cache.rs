use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

use incular_assets::{AssetBundle, AssetError, AssetLoadCallback};
use incular_image::{
    AssetBundleImageKey, BundleImageCache, ImageAssetBundle, ImageCacheLimits, ImageError,
    ImageHandle, ImageId,
};

const IMAGE: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);
const BYTES: usize = 256;

#[derive(Clone, Default)]
struct Counting(Arc<AtomicUsize>);

impl AssetBundle for Counting {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        if key == "missing" {
            return Err(AssetError::NotFound(key.into()));
        }
        Ok(Arc::from(IMAGE))
    }
}

fn key(bundle: &ImageAssetBundle, value: usize) -> AssetBundleImageKey {
    AssetBundleImageKey::new(bundle.clone(), value.to_string(), 1.0)
}

#[test]
fn maintains_cache_size_and_touches_the_lru_on_warm_reads() {
    let source = Counting::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    let cache = BundleImageCache::with_limits(ImageCacheLimits::new(3, BYTES * 32));
    let mut ids = std::collections::HashMap::<ImageId, usize>::new();
    for (attempt, (name, expected)) in [
        (1, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (2, 7),
        (1, 1),
        (3, 9),
        (1, 1),
        (4, 11),
        (1, 1),
        (2, 13),
        (3, 14),
        (4, 15),
        (1, 16),
    ]
    .into_iter()
    .enumerate()
    {
        let image = cache.load(&key(&bundle, name)).unwrap();
        assert_eq!(*ids.entry(image.id()).or_insert(attempt + 1), expected);
        assert!(cache.retained().0 <= 3);
    }
    assert_eq!(source.0.load(Ordering::Relaxed), 8);
    assert_eq!(cache.retained(), (3, BYTES * 3));
}

#[test]
fn clearing_and_single_key_eviction_release_only_their_cache_references() {
    let bundle = ImageAssetBundle::new(Arc::new(Counting::default()));
    let cache = BundleImageCache::default();
    assert_eq!(cache.retained(), (0, 0));
    let first = cache.load(&key(&bundle, 1)).unwrap();
    let second = cache.load(&key(&bundle, 2)).unwrap();
    assert_eq!(cache.retained(), (2, BYTES * 2));
    assert!(cache.contains_key(&key(&bundle, 1)));
    assert!(cache.evict(&key(&bundle, 1)));
    assert!(!cache.evict(&key(&bundle, 1)));
    assert_eq!(cache.retained(), (1, BYTES));
    assert_eq!(cache.load(&key(&bundle, 2)).unwrap().id(), second.id());
    cache.clear();
    assert_eq!(cache.retained(), (0, 0));
    assert!(!cache.contains_key(&key(&bundle, 2)));
    assert_eq!(first.decoded().pixels().len(), BYTES);
    assert_eq!(second.decoded().pixels().len(), BYTES);
    assert_ne!(cache.load(&key(&bundle, 2)).unwrap().id(), second.id());
}

#[test]
fn oversized_images_are_served_without_entering_the_cache_or_evicting_hits() {
    let bundle = ImageAssetBundle::new(Arc::new(Counting::default()));
    let cache = BundleImageCache::with_limits(ImageCacheLimits::new(1000, 1));
    assert_eq!(
        cache
            .load(&key(&bundle, 1))
            .unwrap()
            .decoded()
            .pixels()
            .len(),
        BYTES
    );
    assert_eq!(cache.retained(), (0, 0));
    assert_eq!(cache.limits().max_bytes, 1);
    // The exact byte limit is admissible; an oversized decode must not
    // displace it. Use the same bundle under two physical keys.
    const BIG: &[u8] = include_bytes!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/asset_resolution_48.png"
    );
    let bundle = ImageAssetBundle::new(Arc::new(
        incular_assets::MemoryAssetBundle::new()
            .with("small", IMAGE)
            .with("big", BIG),
    ));
    cache.set_limits(ImageCacheLimits::new(1000, BYTES));
    let small = AssetBundleImageKey::new(bundle.clone(), "small", 1.0);
    let big = AssetBundleImageKey::new(bundle, "big", 1.0);
    let warm = cache.load(&small).unwrap();
    assert_eq!(cache.load(&big).unwrap().decoded().width(), 48);
    assert_eq!(cache.load(&small).unwrap().id(), warm.id());
    assert!(!cache.contains_key(&big));
    assert_eq!(cache.retained(), (1, BYTES));
}

#[test]
fn count_and_byte_resizes_evict_reduce_to_zero_and_readmit() {
    for by_bytes in [false, true] {
        let budget = |entries| {
            if by_bytes {
                ImageCacheLimits::new(1000, BYTES * entries)
            } else {
                ImageCacheLimits::new(entries, BYTES * 32)
            }
        };
        let bundle = ImageAssetBundle::new(Arc::new(Counting::default()));
        let cache = BundleImageCache::with_limits(budget(2));
        let load = |name| cache.load(&key(&bundle, name)).unwrap().id();
        let first = load(1);
        assert_ne!(load(2), first);
        assert_ne!(load(3), first);
        let reloaded = load(1);
        assert_ne!(reloaded, first);
        assert_eq!(cache.retained(), (2, BYTES * 2));
        cache.set_limits(budget(0));
        assert_eq!(cache.retained(), (0, 0));
        let disabled = load(1);
        assert_ne!(disabled, load(1));
        cache.set_limits(budget(3));
        let warmed = load(1);
        assert_ne!(warmed, disabled);
        assert_eq!(load(1), warmed);
        cache.set_limits(budget(1));
        assert_eq!(cache.retained(), (1, BYTES));
    }
}

#[derive(Clone, Default)]
struct Deferred(Arc<Mutex<VecDeque<(String, AssetLoadCallback)>>>);

impl AssetBundle for Deferred {
    fn load(&self, asset: &str) -> Result<Arc<[u8]>, AssetError> {
        panic!("synchronous read of callback-only asset {asset}");
    }
    fn load_async(&self, asset: &str, done: AssetLoadCallback) {
        self.0.lock().unwrap().push_back((asset.into(), done));
    }
}

impl Deferred {
    fn take(&self, asset: &str) -> AssetLoadCallback {
        let (name, callback) = self.0.lock().unwrap().pop_front().unwrap();
        assert_eq!(name, asset);
        callback
    }

    fn pending(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

fn start(
    cache: &BundleImageCache,
    key: &AssetBundleImageKey,
) -> mpsc::Receiver<Result<ImageHandle, ImageError>> {
    let (tx, rx) = mpsc::channel();
    cache.load_async(
        key.clone(),
        Box::new(move |outcome| tx.send(outcome).unwrap()),
    );
    rx
}

fn receive(rx: mpsc::Receiver<Result<ImageHandle, ImageError>>) -> Result<ImageHandle, ImageError> {
    rx.recv_timeout(Duration::from_secs(3)).unwrap()
}

#[test]
fn concurrent_pending_requests_reuse_one_decode_and_retained_completion() {
    let harness = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(harness.clone()));
    let cache = BundleImageCache::default();
    let first_key = key(&bundle, 1);
    assert!(!cache.contains_key(&first_key));
    let first = start(&cache, &first_key);
    assert!(cache.contains_key(&first_key));
    let second = start(&cache, &first_key);
    assert_eq!(harness.pending(), 1);
    assert_eq!(cache.retained(), (0, 0));
    let finish = harness.take("1");
    std::thread::spawn(move || finish(Ok(Arc::from(IMAGE))))
        .join()
        .unwrap();
    let first_id = receive(first).unwrap().id();
    assert_eq!(receive(second).unwrap().id(), first_id);
    assert_eq!(
        start(&cache, &first_key).try_recv().unwrap().unwrap().id(),
        first_id
    );
    assert!(cache.contains_key(&first_key));
    assert_eq!(cache.retained(), (1, BYTES));
    assert_eq!(harness.pending(), 0);
}

#[test]
fn evicted_pending_work_cannot_recapture_the_replacement() {
    for all in [false, true] {
        let harness = Deferred::default();
        let bundle = ImageAssetBundle::new(Arc::new(harness.clone()));
        let cache = BundleImageCache::default();
        let first_key = key(&bundle, 1);
        let first = start(&cache, &first_key);
        let finish_first = harness.take("1");
        if all {
            cache.clear();
        } else {
            assert!(cache.evict(&first_key));
        }
        assert!(!cache.contains_key(&first_key));
        let second = start(&cache, &first_key);
        harness.take("1")(Ok(Arc::from(IMAGE)));
        finish_first(Ok(Arc::from(IMAGE)));
        let first = receive(first).unwrap();
        let second = receive(second).unwrap();
        assert_ne!(first.id(), second.id());
        assert_eq!(cache.load(&first_key).unwrap().id(), second.id());
        assert_eq!(cache.retained(), (1, BYTES));
    }
}

#[test]
fn same_names_from_distinct_bundles_or_scales_do_not_share_and_failures_retry() {
    let one = Counting::default();
    let two = Counting::default();
    let a = ImageAssetBundle::new(Arc::new(one.clone()));
    let b = ImageAssetBundle::new(Arc::new(two.clone()));
    let cache = BundleImageCache::default();
    let a1 = AssetBundleImageKey::new(a.clone(), "some.png", 1.0);
    let a2 = AssetBundleImageKey::new(a.clone(), "some.png", 2.0);
    let b1 = AssetBundleImageKey::new(b, "some.png", 1.0);
    let ids = [&a1, &a2, &b1].map(|key| cache.load(key).unwrap().id());
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
    assert_eq!(cache.load(&a2).unwrap().scale(), 2.0);
    assert_eq!(one.0.load(Ordering::Relaxed), 2);
    assert_eq!(two.0.load(Ordering::Relaxed), 1);
    let missing = AssetBundleImageKey::new(a, "missing", 1.0);
    for _ in 0..2 {
        assert!(matches!(
            cache.load(&missing),
            Err(ImageError::Asset(AssetError::NotFound(_)))
        ));
        assert!(!cache.contains_key(&missing));
    }
    assert_eq!(one.0.load(Ordering::Relaxed), 4);
}
