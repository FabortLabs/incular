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
    AssetBundleImageKey, BundleImageCache, ExactAssetImage, ImageAssetBundle, ImageCacheLimits,
    ImageConfiguration, ImageError, ImageHandle, ImageId, ImageLoadState, ImageProvider,
    clear_shared_bundle_image_cache, shared_bundle_image_cache,
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

#[test]
fn a_failed_asset_provider_resolution_is_removed_from_the_pending_cache() {
    clear_shared_bundle_image_cache();
    let cache = shared_bundle_image_cache();
    let bundle = Deferred::default();
    let provider = ExactAssetImage::new("does-not-exist").bundle(Arc::new(bundle.clone()));
    let configuration = ImageConfiguration::empty();
    let key = provider.obtain_key(&configuration);
    assert!(!cache.contains_key(&key));
    assert_eq!(cache.pending_image_count(), 0);

    let stream = provider.resolve(&configuration);
    assert!(matches!(stream.state(), ImageLoadState::Loading(None)));
    assert!(cache.contains_key(&key));
    assert_eq!(cache.pending_image_count(), 1);

    let finish = bundle.take("does-not-exist");
    finish(Err(AssetError::NotFound("does-not-exist".into())));
    let _ = incular_animation::scheduler::run_one_scheduled_task();
    assert!(matches!(
        stream.state(),
        ImageLoadState::Failed(ImageError::Asset(AssetError::NotFound(_)))
    ));
    assert!(!cache.contains_key(&key));
    assert_eq!(cache.pending_image_count(), 0);
    assert_eq!(cache.retained(), (0, 0));

    // Correcting the source retries rather than reusing a cached failure.
    let retry = provider.resolve(&configuration);
    assert_eq!(cache.pending_image_count(), 1);
    bundle.take("does-not-exist")(Ok(Arc::from(IMAGE)));
    let _ = incular_animation::scheduler::run_one_scheduled_task();
    let ImageLoadState::Ready(decoded) = retry.state() else {
        panic!("the corrected asset must complete")
    };
    assert_eq!(decoded.decoded().pixels().len(), BYTES);
    assert!(cache.contains_key(&key));
    assert_eq!(cache.pending_image_count(), 0);
    assert_eq!(cache.retained(), (1, BYTES));
    clear_shared_bundle_image_cache();
}

#[test]
fn a_blocking_decode_started_with_retention_off_does_not_enter_a_reenabled_cache() {
    struct EnableOnce {
        cache: Mutex<Option<BundleImageCache>>,
        reads: AtomicUsize,
    }

    impl AssetBundle for EnableOnce {
        fn load(&self, _key: &str) -> Result<Arc<[u8]>, AssetError> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            if let Some(cache) = self.cache.lock().unwrap().take() {
                cache.set_limits(ImageCacheLimits::new(4, 4096));
            }
            Ok(Arc::from(IMAGE))
        }
    }

    for limits in [ImageCacheLimits::new(0, 4096), ImageCacheLimits::new(4, 0)] {
        let cache = BundleImageCache::with_limits(limits);
        let source = Arc::new(EnableOnce {
            cache: Mutex::new(Some(cache.clone())),
            reads: AtomicUsize::new(0),
        });
        let key = AssetBundleImageKey::new(ImageAssetBundle::new(source.clone()), "asset", 1.);
        let first = cache.load(&key).unwrap();
        assert_eq!(first.decoded().pixels().len(), BYTES);
        assert!(
            !cache.contains_key(&key),
            "limits at request start: {limits:?}"
        );
        assert_eq!(cache.retained(), (0, 0));
        assert_eq!(cache.pending_image_count(), 0);

        let second = cache.load(&key).unwrap();
        assert_ne!(first.id(), second.id());
        assert_eq!(cache.retained(), (1, BYTES));
        assert_eq!(cache.load(&key).unwrap().id(), second.id());
        assert_eq!(source.reads.load(Ordering::Relaxed), 2);
    }
}

struct BlockingFirstRead {
    started: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    loads: AtomicUsize,
}
impl AssetBundle for BlockingFirstRead {
    fn load(&self, asset: &str) -> Result<Arc<[u8]>, AssetError> {
        if asset == "1" && self.loads.fetch_add(1, Ordering::Relaxed) == 0 {
            self.started.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
        }
        Ok(Arc::from(IMAGE))
    }
}

fn pause_first_read() -> (Arc<BlockingFirstRead>, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let source = Arc::new(BlockingFirstRead {
        started: started_tx,
        release: Mutex::new(release_rx),
        loads: AtomicUsize::new(0),
    });
    (source, started_rx, release_tx)
}

#[test]
fn evicting_one_key_does_not_discard_an_unrelated_in_flight_blocking_read() {
    let (source, started_rx, release_tx) = pause_first_read();
    let bundle = ImageAssetBundle::new(source.clone());
    let cache = BundleImageCache::default();
    let first = key(&bundle, 1);
    let second = key(&bundle, 2);
    let second_id = cache.load(&second).unwrap().id();
    let worker = {
        let (cache, first) = (cache.clone(), first.clone());
        std::thread::spawn(move || cache.load(&first).unwrap())
    };
    started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let evicted = cache.evict(&second);
    let missing = cache.evict(&key(&bundle, 3));
    release_tx.send(()).unwrap();
    let completed = worker.join().unwrap();

    assert!(evicted);
    assert!(!missing);
    assert_eq!(cache.load(&first).unwrap().id(), completed.id());
    assert_eq!(source.loads.load(Ordering::Relaxed), 1);
    assert_ne!(cache.load(&second).unwrap().id(), second_id);
}

#[test]
fn detaching_blocking_reads_never_replaces_a_new_generation() {
    for mode in 0..3 {
        let (source, started_rx, release_tx) = pause_first_read();
        let bundle = ImageAssetBundle::new(source.clone());
        let cache = BundleImageCache::default();
        let asset = key(&bundle, 1);
        let worker = {
            let (cache, asset) = (cache.clone(), asset.clone());
            std::thread::spawn(move || cache.load(&asset).unwrap())
        };
        started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let pending_before = cache.pending_image_count();
        let visible_before = cache.contains_key(&asset);
        let evicted = match mode {
            0 => cache.evict(&asset),
            1 => {
                cache.clear();
                true
            }
            2 => {
                let limits = cache.limits();
                cache.set_limits(ImageCacheLimits::new(0, BYTES));
                cache.set_limits(limits);
                true
            }
            _ => unreachable!(),
        };
        let pending_after = cache.pending_image_count();
        let visible_after = cache.contains_key(&asset);
        let replacement = cache.load(&asset).unwrap();
        release_tx.send(()).unwrap();
        let detached = worker.join().unwrap();

        assert_eq!(pending_before, 1, "mode {mode}");
        assert!(visible_before, "mode {mode}");
        assert!(evicted, "mode {mode}");
        assert_eq!(pending_after, 0, "mode {mode}");
        assert!(!visible_after, "mode {mode}");
        assert_ne!(detached.id(), replacement.id(), "mode {mode}");
        assert_eq!(cache.load(&asset).unwrap().id(), replacement.id());
        assert_eq!(cache.retained(), (1, BYTES));
        assert_eq!(cache.pending_image_count(), 0);
        assert_eq!(source.loads.load(Ordering::Relaxed), 2);
    }
}

#[test]
fn a_failed_blocking_read_drops_its_pending_ticket() {
    let bundle = ImageAssetBundle::new(Arc::new(Counting::default()));
    let cache = BundleImageCache::default();
    let key = AssetBundleImageKey::new(bundle, "missing", 1.0);
    assert!(matches!(
        cache.load(&key),
        Err(ImageError::Asset(AssetError::NotFound(_)))
    ));
    assert_eq!(cache.pending_image_count(), 0);
    assert!(!cache.contains_key(&key));
    assert!(!cache.evict(&key));
}
