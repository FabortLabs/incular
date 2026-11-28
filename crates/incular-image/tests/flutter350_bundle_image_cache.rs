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

fn finish_pending_streams() {
    use incular_animation::scheduler::{has_ready_scheduled_task, run_one_scheduled_task};

    for _ in 0..12 {
        if !has_ready_scheduled_task() {
            return;
        }
        let _ = run_one_scheduled_task();
    }
    assert!(!has_ready_scheduled_task(), "pending image streams settle");
}

#[test]
fn subsequent_uses_of_a_pending_bundle_key_receive_the_same_stream() {
    let source = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    let cache = BundleImageCache::default();
    let key = key(&bundle, 1);
    assert!(!cache.contains_key(&key));
    let first = cache.resolve_stream(key.clone());
    assert!(cache.contains_key(&key));
    assert_eq!(cache.pending_image_count(), 1);
    assert!(matches!(first.state(), ImageLoadState::Loading(_)));

    let second = cache.resolve_stream(key.clone());
    assert_eq!(first, second);
    assert_eq!(source.pending(), 1);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    let ImageLoadState::Ready(image) = first.state() else {
        panic!("shared stream never received its decoded result");
    };
    assert_eq!(second.state(), ImageLoadState::Ready(image));
    assert_eq!(cache.pending_image_count(), 0);
    assert_eq!(cache.retained(), (1, BYTES));
}

#[test]
fn clearing_a_pending_key_keeps_existing_consumers_but_replaces_its_stream() {
    let source = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    let cache = BundleImageCache::default();
    let key = key(&bundle, 1);
    let before = cache.resolve_stream(key.clone());
    cache.clear();
    assert!(!cache.contains_key(&key));

    let after = cache.resolve_stream(key.clone());
    assert_ne!(before, after);
    assert_eq!(source.pending(), 2);
    assert_eq!(cache.resolve_stream(key), after);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(before.state(), ImageLoadState::Ready(_)));
    assert!(matches!(after.state(), ImageLoadState::Loading(_)));
    assert_eq!(cache.retained(), (0, 0));
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(after.state(), ImageLoadState::Ready(_)));
    assert_ne!(before.state(), after.state());
    assert_eq!(cache.retained(), (1, BYTES));
}

#[test]
fn exact_asset_resolution_reuses_a_live_pending_stream_and_its_label() {
    let source = Deferred::default();
    let image = ExactAssetImage::new("1").bundle(Arc::new(source.clone()));
    let configuration = ImageConfiguration::empty();
    clear_shared_bundle_image_cache();
    let first = image.resolve(&configuration);
    let second = image.resolve(&configuration);
    assert_eq!(first, second);
    assert_eq!(first.debug_label().as_deref(), Some("1"));
    assert_eq!(second.debug_label().as_deref(), Some("1"));
    assert_eq!(source.pending(), 1);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(second.state(), ImageLoadState::Ready(_)));
}

fn live_bundle_fixture() -> (BundleImageCache, Deferred, AssetBundleImageKey) {
    let source = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    (BundleImageCache::default(), source, key(&bundle, 1))
}

fn observe_frame_group(
    stream: &incular_image::ImageStream,
) -> incular_image::ImageStreamSubscription {
    use incular_image::ImageStreamListener;
    use std::rc::Rc;
    stream.listen(ImageStreamListener::new(Rc::new(|_, _| {})))
}

#[test]
fn clearing_pending_storage_does_not_remove_live_consumers_until_live_clear() {
    let (cache, source, asset) = live_bundle_fixture();
    let original = cache.resolve_stream(asset.clone());
    let _observer = observe_frame_group(&original);
    let status = cache.status_for_key(&asset);
    assert!(status.pending && status.live && !status.keep_alive);
    cache.clear();
    let status = cache.status_for_key(&asset);
    assert!(!status.pending && status.live && !status.keep_alive);
    cache.clear_live_images();
    assert_eq!(cache.status_for_key(&asset), Default::default());
    let replacement = cache.resolve_stream(asset.clone());
    assert_ne!(original, replacement);
    source.take("1")(Ok(Arc::from(IMAGE)));
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(original.state(), ImageLoadState::Ready(_)));
    assert!(matches!(replacement.state(), ImageLoadState::Ready(_)));
}

#[test]
fn evicting_a_pending_stream_by_default_clears_pending_and_live_entries() {
    let (cache, source, asset) = live_bundle_fixture();
    let stream = cache.resolve_stream(asset.clone());
    assert!(cache.status_for_key(&asset).pending);
    assert!(cache.status_for_key(&asset).live);
    assert!(!cache.status_for_key(&asset).keep_alive);
    assert!(cache.evict(&asset));
    assert_eq!(cache.status_for_key(&asset), Default::default());
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(stream.state(), ImageLoadState::Ready(_)));
    assert_eq!(cache.retained(), (0, 0));
}

#[test]
fn evicting_pending_without_live_only_preserves_external_observers() {
    for observed in [false, true] {
        let (cache, source, asset) = live_bundle_fixture();
        let stream = cache.resolve_stream(asset.clone());
        let before = cache.status_for_key(&asset);
        assert!(before.pending && before.live && !before.keep_alive);
        let observer = observed.then(|| observe_frame_group(&stream));
        assert!(cache.evict_with_live(&asset, false));
        let after = cache.status_for_key(&asset);
        assert!(!after.pending && !after.keep_alive);
        assert_eq!(after.live, observed);
        if observed {
            assert_eq!(cache.resolve_stream(asset.clone()), stream);
        }
        source.take("1")(Ok(Arc::from(IMAGE)));
        finish_pending_streams();
        assert!(matches!(stream.state(), ImageLoadState::Ready(_)));
        drop(observer);
        assert_eq!(cache.live_image_count(), 0);
    }
}

#[test]
fn evicting_a_completed_stream_distinguishes_live_and_kept_alive() {
    for include_live in [true, false] {
        let (cache, source, asset) = live_bundle_fixture();
        let stream = cache.resolve_stream(asset.clone());
        let observer = observe_frame_group(&stream);
        source.take("1")(Ok(Arc::from(IMAGE)));
        finish_pending_streams();
        let before = cache.status_for_key(&asset);
        assert!(!before.pending && before.live && before.keep_alive);
        assert!(cache.evict_with_live(&asset, include_live));
        let after = cache.status_for_key(&asset);
        assert!(!after.pending && !after.keep_alive);
        assert_eq!(after.live, !include_live);
        if !include_live {
            assert_eq!(cache.resolve_stream(asset.clone()), stream);
            assert!(cache.status_for_key(&asset).keep_alive);
            assert_eq!(source.pending(), 0);
        }
        drop(observer);
        assert!(!cache.status_for_key(&asset).live);
    }
}

#[test]
fn releasing_an_old_live_group_cannot_remove_the_replacement_tracking() {
    let (cache, source, asset) = live_bundle_fixture();
    let original = cache.resolve_stream(asset.clone());
    let original_observer = observe_frame_group(&original);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(cache.live_image_count(), 1);
    cache.clear();
    cache.clear_live_images();
    assert_eq!(cache.live_image_count(), 0);
    let replacement = cache.resolve_stream(asset.clone());
    assert_ne!(original, replacement);
    let _replacement_observer = observe_frame_group(&replacement);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(cache.live_image_count(), 1);
    drop(original_observer);
    assert_eq!(cache.live_image_count(), 1);
    assert!(cache.status_for_key(&asset).live);
}

#[test]
fn live_tracking_releases_all_six_unobserved_streams_with_or_without_cache_retention() {
    for maximum in [3, 0] {
        let source = Deferred::default();
        let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
        let cache = BundleImageCache::with_limits(ImageCacheLimits::new(maximum, BYTES * 6));
        for i in 1..=6 {
            drop(cache.resolve_stream(key(&bundle, i)));
            source.take(&i.to_string())(Ok(Arc::from(IMAGE)));
            finish_pending_streams();
        }
        assert_eq!(cache.retained(), (maximum, maximum * BYTES));
        assert_eq!(cache.live_image_count(), 0);
        assert_eq!(cache.pending_image_count(), 0);
    }
}

#[test]
fn clearing_only_a_cache_owned_pending_listener_never_leaves_live_tracking() {
    let source = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    let cache = BundleImageCache::with_limits(ImageCacheLimits::new(1, BYTES * 2));
    let first = key(&bundle, 1);
    let second = key(&bundle, 2);
    let _original = cache.resolve_stream(first.clone());
    assert!(cache.status_for_key(&first).pending && cache.status_for_key(&first).live);
    cache.clear();
    assert_eq!(cache.status_for_key(&first), Default::default());
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(cache.status_for_key(&first), Default::default());
    let _second = cache.resolve_stream(second.clone());
    source.take("2")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(cache.status_for_key(&first), Default::default());
    assert!(cache.status_for_key(&second).keep_alive);
}

#[test]
fn a_live_completed_image_can_lose_the_only_lru_slot_without_losing_its_listener() {
    let source = Deferred::default();
    let bundle = ImageAssetBundle::new(Arc::new(source.clone()));
    let cache = BundleImageCache::with_limits(ImageCacheLimits::new(1, BYTES * 4));
    let first = key(&bundle, 1);
    let second = key(&bundle, 2);
    let observed = cache.resolve_stream(first.clone());
    let _observer = observe_frame_group(&observed);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    let status = cache.status_for_key(&first);
    assert!(!status.pending && status.keep_alive && status.live);
    assert_eq!(cache.status_for_key(&second), Default::default());

    let unobserved = cache.resolve_stream(second.clone());
    source.take("2")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(unobserved.state(), ImageLoadState::Ready(_)));
    let old = cache.status_for_key(&first);
    let new = cache.status_for_key(&second);
    assert!(!old.pending && !old.keep_alive && old.live);
    assert!(!new.pending && new.keep_alive && !new.live);
}

#[test]
fn restoring_a_live_8_by_8_frame_recovers_exactly_its_decoded_bytes() {
    let (cache, source, asset) = live_bundle_fixture();
    let stream = cache.resolve_stream(asset.clone());
    let _observer = observe_frame_group(&stream);
    assert_eq!(cache.retained(), (0, 0));
    assert!(cache.status_for_key(&asset).pending && cache.status_for_key(&asset).live);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(cache.retained(), (1, BYTES));
    assert!(cache.evict_with_live(&asset, false));
    assert_eq!(cache.retained(), (0, 0));
    assert!(cache.status_for_key(&asset).live);
    assert_eq!(cache.resolve_stream(asset.clone()), stream);
    assert_eq!(cache.retained(), (1, BYTES));
    assert!(cache.status_for_key(&asset).keep_alive);
    assert_eq!(source.pending(), 0);
}

fn empty_manifest_bytes() -> Arc<[u8]> {
    use incular_core::standard_message_codec::{MessageValue, StandardMessageCodec};

    StandardMessageCodec::encode_message(&MessageValue::Map(Vec::new()))
        .unwrap()
        .unwrap()
        .into()
}

#[test]
fn a_deferred_asset_manifest_handoff_preserves_the_mounted_groups_live_key() {
    use incular_assets::CachingAssetBundle;
    use incular_image::AssetImage;

    clear_shared_bundle_image_cache();
    let cache = shared_bundle_image_cache();
    let transport = Deferred::default();
    let bundle = Arc::new(CachingAssetBundle::new(transport.clone()));
    let provider = AssetImage::new("1").bundle(bundle.clone());
    let key = key(&ImageAssetBundle::new(bundle), 1);
    let configuration = ImageConfiguration::empty();
    let stream = provider.resolve(&configuration);
    let consumer = observe_frame_group(&stream);
    assert_eq!(cache.status_for_key(&key), Default::default());

    transport.take("AssetManifest.bin")(Ok(empty_manifest_bytes()));
    assert_eq!(transport.pending(), 1, "image fetch starts on key delivery");
    finish_pending_streams();
    assert!(cache.status_for_key(&key).pending && cache.status_for_key(&key).live);
    assert!(cache.evict_with_live(&key, false));
    assert!(!cache.status_for_key(&key).pending);
    assert!(cache.status_for_key(&key).live);
    let reuse = provider.resolve(&configuration);
    assert_eq!(transport.pending(), 1);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(stream.state(), ImageLoadState::Ready(_)));
    assert_eq!(reuse.state(), stream.state());
    assert!(cache.status_for_key(&key).live);
    drop(consumer);
    assert!(!cache.status_for_key(&key).live);
    clear_shared_bundle_image_cache();
}

#[test]
fn a_discarded_asset_view_can_still_warm_the_cache_after_deferred_key_resolution() {
    use incular_assets::CachingAssetBundle;
    use incular_image::AssetImage;

    clear_shared_bundle_image_cache();
    let cache = shared_bundle_image_cache();
    let transport = Deferred::default();
    let bundle = Arc::new(CachingAssetBundle::new(transport.clone()));
    let provider = AssetImage::new("1").bundle(bundle.clone());
    let key = key(&ImageAssetBundle::new(bundle), 1);
    drop(provider.resolve(&ImageConfiguration::empty()));
    transport.take("AssetManifest.bin")(Ok(empty_manifest_bytes()));
    finish_pending_streams();
    assert!(cache.status_for_key(&key).pending);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    let final_status = cache.status_for_key(&key);
    assert!(!final_status.live && !final_status.pending && final_status.keep_alive);
    assert_eq!(cache.retained(), (1, BYTES));
    clear_shared_bundle_image_cache();
}

#[test]
fn a_mapped_completion_detaches_its_live_use_from_the_source_stream() {
    let (cache, transport, asset) = live_bundle_fixture();
    let original = cache.resolve_stream(asset.clone());
    let mapped = original.clone().map_image(Ok);
    let _observer = observe_frame_group(&mapped);
    assert!(cache.status_for_key(&asset).live);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(mapped.state(), ImageLoadState::Ready(_)));
    assert!(!cache.status_for_key(&asset).live);
    assert!(cache.status_for_key(&asset).keep_alive);
}

#[test]
fn an_old_worker_preparation_cannot_steal_the_replacement_live_key() {
    use incular_assets::CachingAssetBundle;
    use incular_image::AssetImage;

    clear_shared_bundle_image_cache();
    let cache = shared_bundle_image_cache();
    let transport = Deferred::default();
    let bundle = Arc::new(CachingAssetBundle::new(transport.clone()));
    let provider = AssetImage::new("1").bundle(bundle.clone());
    let asset = key(&ImageAssetBundle::new(bundle), 1);
    let old = provider.resolve(&ImageConfiguration::empty());
    let _old_group = observe_frame_group(&old);
    transport.take("AssetManifest.bin")(Ok(empty_manifest_bytes()));
    assert!(cache.evict(&asset));
    let replacement = cache.resolve_stream(asset.clone());
    finish_pending_streams();
    assert_eq!(cache.resolve_stream(asset.clone()), replacement);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(old.state(), ImageLoadState::Ready(_)));
    assert!(matches!(replacement.state(), ImageLoadState::Loading(_)));
    assert_eq!(cache.retained(), (0, 0));
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(replacement.state(), ImageLoadState::Ready(_)));
    assert!(cache.status_for_key(&asset).keep_alive);
    assert!(!cache.status_for_key(&asset).live);
    clear_shared_bundle_image_cache();
}

#[test]
fn a_synchronous_loader_error_returns_to_the_caller_or_the_supplied_handler() {
    let cache = BundleImageCache::default();
    let bundle = ImageAssetBundle::new(Arc::new(Counting::default()));
    let missing = AssetBundleImageKey::new(bundle, "missing".to_owned(), 1.);
    let expected = cache.resolve_stream_checked(missing.clone()).unwrap_err();
    let mut caught = None;
    assert!(
        cache
            .resolve_stream_on_error(missing.clone(), |error| caught = Some(error))
            .is_none()
    );
    assert_eq!(caught, Some(expected));
    assert_eq!(cache.status_for_key(&missing), Default::default());
}

#[test]
fn a_deferred_loader_error_stays_in_its_stream_rather_than_the_synchronous_handler() {
    let (cache, transport, asset) = live_bundle_fixture();
    let stream = cache
        .resolve_stream_on_error(asset.clone(), |_| panic!("this failure is not inline"))
        .unwrap();
    assert!(cache.status_for_key(&asset).pending);
    transport.take("1")(Err(AssetError::NotFound("1".into())));
    finish_pending_streams();
    assert!(matches!(stream.state(), ImageLoadState::Failed(_)));
    assert_eq!(cache.status_for_key(&asset), Default::default());
}

#[test]
fn decoded_pixels_live_until_the_last_cache_listener_or_image_info_owner_releases_them() {
    use incular_image::ImageStreamListener;
    use std::{cell::RefCell, rc::Rc};

    for listener_during_eviction in [false, true] {
        let (cache, transport, asset) = live_bundle_fixture();
        let received = Rc::new(RefCell::new(None::<incular_image::ImageInfo>));
        let stream = cache.resolve_stream(asset.clone());
        let consumer = stream.listen(ImageStreamListener::new({
            let received = received.clone();
            Rc::new(move |info, _| *received.borrow_mut() = Some(info))
        }));
        transport.take("1")(Ok(Arc::from(IMAGE)));
        finish_pending_streams();
        let frame = received.borrow_mut().take().unwrap();
        let weak = frame.image().downgrade();
        assert_eq!(frame.image().size_bytes(), BYTES);
        assert!(cache.status_for_key(&asset).keep_alive);
        drop(stream);

        let mut consumer = Some(consumer);
        if !listener_during_eviction {
            drop(consumer.take());
        }
        assert!(cache.evict(&asset));
        assert_eq!(cache.retained(), (0, 0));
        assert!(weak.upgrade().is_some());
        drop(consumer);
        assert!(weak.upgrade().is_some());
        drop(frame);
        assert!(
            weak.is_dropped(),
            "no cache or listener pixel owner remains"
        );
    }
}

#[test]
fn a_missing_bundle_asset_is_removed_before_the_observer_receives_the_error() {
    use incular_image::ImageStreamListener;
    use std::{cell::Cell, rc::Rc};

    let (cache, transport, asset) = live_bundle_fixture();
    let first = cache.resolve_stream(asset.clone());
    let seen = Rc::new(Cell::new(None));
    let _observer = first.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("failed asset has no frame"))).on_error({
            let cache = cache.clone();
            let asset = asset.clone();
            let seen = seen.clone();
            Rc::new(move |_| {
                seen.set(Some((
                    cache.status_for_key(&asset).pending,
                    cache.evict(&asset),
                )));
            })
        }),
    );
    transport.take("1")(Err(AssetError::NotFound("missing asset".into())));
    finish_pending_streams();
    assert_eq!(seen.get(), Some((false, false)));
    let retry = cache.resolve_stream(asset.clone());
    assert_ne!(retry, first);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(retry.state(), ImageLoadState::Ready(_)));
}

#[test]
fn clearing_live_tracking_during_decode_keeps_the_original_listener_and_pending_work() {
    use incular_image::ImageStreamListener;
    use std::{cell::Cell, rc::Rc};

    let (cache, source, asset) = live_bundle_fixture();
    let stream = cache.resolve_stream(asset.clone());
    let frames = Rc::new(Cell::new(0));
    let record = frames.clone();
    let _listener = stream.listen(ImageStreamListener::new(Rc::new(move |_, synchronous| {
        assert!(!synchronous);
        record.set(record.get() + 1);
    })));
    let before = cache.status_for_key(&asset);
    assert!(before.pending && before.live && !before.keep_alive);

    cache.clear_live_images();
    let after = cache.status_for_key(&asset);
    assert!(after.pending && !after.live && !after.keep_alive);
    assert_eq!(source.pending(), 1);
    assert_eq!(frames.get(), 0);
    source.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert_eq!(frames.get(), 1);
    assert!(matches!(stream.state(), ImageLoadState::Ready(_)));
    assert!(cache.status_for_key(&asset).keep_alive);
}

#[test]
fn a_deferred_byte_decode_failure_can_be_evicted_by_its_error_listener() {
    use incular_image::ImageStreamListener;
    use std::{cell::Cell, rc::Rc};

    let (cache, transport, asset) = live_bundle_fixture();
    let first = cache.resolve_stream(asset.clone());
    assert!(cache.status_for_key(&asset).pending);
    let seen = Rc::new(Cell::new(None));
    let _listener = first.listen(
        ImageStreamListener::new(Rc::new(|_, _| {
            panic!("invalid bytes must not produce a frame")
        }))
        .on_error({
            let (cache, asset, seen) = (cache.clone(), asset.clone(), seen.clone());
            Rc::new(move |err| {
                assert!(matches!(err, ImageError::Decode(_)));
                seen.set(Some((
                    cache.status_for_key(&asset).pending,
                    cache.evict(&asset),
                )));
            })
        }),
    );
    transport.take("1")(Ok(Arc::from([0_u8; 8])));
    finish_pending_streams();
    assert_eq!(seen.get(), Some((true, true)));
    assert!(!cache.status_for_key(&asset).pending);
    assert_eq!(cache.retained(), (0, 0));

    let retry = cache.resolve_stream(asset);
    assert_ne!(retry, first);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(retry.state(), ImageLoadState::Ready(_)));
}

#[test]
fn a_failed_async_stream_is_replayed_until_eviction_without_retaining_pixels() {
    let (cache, transport, asset) = live_bundle_fixture();
    let first = cache.resolve_stream(asset.clone());
    transport.take("1")(Ok(Arc::from([0_u8; 8])));
    finish_pending_streams();
    assert!(matches!(
        first.state(),
        ImageLoadState::Failed(ImageError::Decode(_))
    ));
    assert!(cache.status_for_key(&asset).pending);
    assert_eq!(cache.pending_image_count(), 1);
    assert_eq!(cache.retained(), (0, 0));
    let same = cache
        .resolve_stream_on_error(asset.clone(), |_| panic!("not a fresh inline loader error"))
        .unwrap();
    assert_eq!(same, first);
    assert_eq!(transport.pending(), 0);
    assert!(cache.evict(&asset));
    assert_eq!(cache.pending_image_count(), 0);
}

#[test]
fn direct_bundle_callback_remains_retryable_after_invalid_bytes() {
    let (cache, transport, asset) = live_bundle_fixture();
    let (send, receive) = mpsc::channel();
    cache.load_async(
        asset.clone(),
        Box::new(move |result| send.send(result).unwrap()),
    );
    transport.take("1")(Ok(Arc::from([0_u8; 8])));
    assert!(matches!(
        receive.try_recv().unwrap(),
        Err(ImageError::Decode(_))
    ));
    assert_eq!(cache.pending_image_count(), 0);
    assert_eq!(cache.retained(), (0, 0));
    assert!(!cache.evict(&asset));
    let retry = cache.resolve_stream(asset);
    transport.take("1")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    assert!(matches!(retry.state(), ImageLoadState::Ready(_)));
}

#[test]
fn provider_cache_status_selects_the_density_key_and_never_loads_pixels() {
    use incular_assets::CachingAssetBundle;
    use incular_core::standard_message_codec::{MessageValue as Msg, StandardMessageCodec};
    use incular_image::{AssetImage, BundleImageCacheStatus, ImageStreamListener};
    use std::{
        future::Future,
        pin::pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    let transport = Deferred::default();
    let bundle = Arc::new(CachingAssetBundle::new(transport.clone()));
    let manifest = Msg::Map(vec![(
        Msg::String("badge.png".into()),
        Msg::List(vec![Msg::Map(vec![
            (
                Msg::String("asset".into()),
                Msg::String("3.0x/badge.png".into()),
            ),
            (Msg::String("dpr".into()), Msg::Float(3.)),
        ])]),
    )]);
    let manifest_bytes = StandardMessageCodec::encode_message(&manifest)
        .unwrap()
        .unwrap();

    let provider = AssetImage::new("badge.png").bundle(bundle.clone());
    let config = ImageConfiguration::new().device_pixel_ratio(3.);
    let exact = ExactAssetImage::new("3.0x/badge.png")
        .bundle(bundle.clone())
        .scale(3.);
    let other_density = ExactAssetImage::new("3.0x/badge.png").bundle(bundle);
    let mut cx = Context::from_waker(Waker::noop());
    let mut cold_query = pin!(provider.obtain_cache_status(&config));
    assert_eq!(cold_query.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(transport.pending(), 1);
    transport.take("AssetManifest.bin")(Ok(manifest_bytes.into()));
    assert_eq!(
        cold_query.as_mut().poll(&mut cx),
        Poll::Ready(Ok(BundleImageCacheStatus::default()))
    );
    assert_eq!(transport.pending(), 0, "status must not open the image");

    let stream = provider.resolve(&config);
    assert!(matches!(stream.state(), ImageLoadState::Loading(_)));
    assert!(exact.obtain_cache_status(&config).pending);
    let mut query = pin!(provider.obtain_cache_status(&config));
    assert_eq!(
        query.as_mut().poll(&mut cx),
        Poll::Ready(Ok(BundleImageCacheStatus {
            pending: true,
            keep_alive: false,
            live: true,
        }))
    );
    let listener = stream.listen(ImageStreamListener::new(Rc::new(|info, synchronous| {
        assert!(!synchronous);
        assert_eq!(info.scale(), 3.);
    })));
    transport.take("3.0x/badge.png")(Ok(Arc::from(IMAGE)));
    finish_pending_streams();
    let mut query = pin!(provider.obtain_cache_status(&config));
    assert_eq!(
        query.as_mut().poll(&mut cx),
        Poll::Ready(Ok(BundleImageCacheStatus {
            pending: false,
            keep_alive: true,
            live: true,
        }))
    );
    assert_eq!(
        other_density.obtain_cache_status(&config),
        BundleImageCacheStatus::default(),
        "the same physical file name with a different scale is a different cache key",
    );
    drop(listener);
    let status = exact.obtain_cache_status(&config);
    assert!(status.keep_alive);
    assert!(!status.live);
    assert_eq!(transport.pending(), 0);
}

#[test]
fn a_key_error_is_delivered_through_both_stream_and_cache_query_without_image_io() {
    use incular_assets::CachingAssetBundle;
    use incular_image::{AssetImage, ImageStreamListener};
    use std::{
        cell::RefCell,
        future::Future,
        pin::pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    let transport = Deferred::default();
    let provider = AssetImage::new("never-load.png")
        .bundle(Arc::new(CachingAssetBundle::new(transport.clone())));
    let config = ImageConfiguration::empty();
    let mut query = pin!(provider.obtain_cache_status(&config));
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(query.as_mut().poll(&mut cx), Poll::Pending);
    let stream = provider.resolve(&config);
    assert!(matches!(stream.state(), ImageLoadState::Loading(_)));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let _listener = stream.listen(
        ImageStreamListener::new(Rc::new(|_, _| panic!("no image key"))).on_error({
            let observed = observed.clone();
            Rc::new(move |error| observed.borrow_mut().push(error.clone()))
        }),
    );
    assert_eq!(
        transport.pending(),
        1,
        "both routes share the manifest request"
    );
    transport.take("AssetManifest.bin")(Ok(Arc::from([0xff, 0xff])));
    let Poll::Ready(Err(error)) = query.as_mut().poll(&mut cx) else {
        panic!("cache query did not return the manifest failure");
    };
    assert!(
        matches!(&error, ImageError::Asset(AssetError::Parse { key, .. }) if key == "AssetManifest.bin")
    );
    finish_pending_streams();
    assert_eq!(stream.state(), ImageLoadState::Failed(error.clone()));
    assert_eq!(observed.borrow().as_slice(), &[error]);
    assert_eq!(transport.pending(), 0);
}
