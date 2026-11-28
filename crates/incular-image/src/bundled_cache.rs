//! Bounded decoded-resource cache keyed by a Flutter bundle image key.

use std::{
    cell::{OnceCell, RefCell},
    collections::{HashMap, HashSet, hash_map::Entry},
    sync::{Arc, Mutex, PoisonError, Weak},
};

use lru::LruCache;

use crate::{
    AssetBundleImageKey, ImageCacheLimits, ImageConfiguration, ImageError, ImageHandle,
    ImageLoadCallback, ImageResizeOptions, ImageStream,
};

mod live_streams;

type Pending = Arc<Mutex<PendingImage>>;

/// Decode jobs are Send, but their UI stream cells are not. Intern weak
/// views in the current UI thread without retaining finished jobs, decoded
/// frames, or consumers. A geometric sweep bounds stale entries without
/// forgetting a view that still has a live pending job and a caller.
struct PendingStreamEntry {
    job: Weak<Mutex<PendingImage>>,
    stream: crate::stream::WeakImageStream,
}

struct PendingStreamMemo {
    entries: HashMap<usize, PendingStreamEntry>,
    next_sweep: usize,
}

impl PendingStreamMemo {
    fn get(&self, job: &Pending) -> Option<ImageStream> {
        self.entries.get(&(Arc::as_ptr(job) as usize)).and_then(|entry| {
            // The weak job protects its allocation identity even after the
            // caller evicts it; a replacement cannot alias this view.
            debug_assert_eq!(entry.job.as_ptr(), Arc::as_ptr(job));
            entry.stream.upgrade()
        })
    }

    fn remember(&mut self, job: &Pending, stream: &ImageStream) {
        if self.entries.len() >= self.next_sweep {
            self.entries.retain(|_, entry| {
                entry.job.strong_count() != 0 && entry.stream.is_live()
            });
            self.next_sweep = self.entries.len().saturating_mul(2).max(32);
        }
        self.entries.insert(
            Arc::as_ptr(job) as usize,
            PendingStreamEntry {
                job: Arc::downgrade(job),
                stream: stream.downgrade(),
            },
        );
    }
}

fn stream_for_pending(job: &Pending) -> ImageStream {
    if let Some(stream) = PENDING_STREAMS.with(|memo| memo.borrow().get(job)) {
        return stream;
    }
    let stream = ImageStream::from_load_callback(|done| attach(job, done));
    PENDING_STREAMS.with(|memo| memo.borrow_mut().remember(job, &stream));
    stream
}

enum PendingImage {
    Loading(Vec<ImageLoadCallback>),
    Completed(Result<ImageHandle, ImageError>),
}

fn attach(pending: &Pending, done: ImageLoadCallback) {
    let completed = {
        let mut pending = pending.lock().unwrap_or_else(PoisonError::into_inner);
        match &mut *pending {
            PendingImage::Loading(waiters) => {
                waiters.push(done);
                return;
            }
            PendingImage::Completed(result) => result.clone(),
        }
    };
    done(completed);
}

/// A worker may start the decode immediately after obtaining the key.
/// The UI later joins the right pending stream without starting I/O again.
pub(crate) struct PreparedBundleStream {
    key: BundleDecodeKey,
    pending: Option<Pending>,
    result: futures_channel::oneshot::Receiver<Result<ImageHandle, ImageError>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BundleDecodeKey {
    asset: AssetBundleImageKey,
    resize: Option<ImageResizeOptions>,
}

impl BundleDecodeKey {
    fn plain(asset: AssetBundleImageKey) -> Self {
        Self {
            asset,
            resize: None,
        }
    }

    fn read_uncached(&self) -> Result<ImageHandle, ImageError> {
        let image = self.asset.read_uncached()?;
        match self.resize {
            Some(size) => size.resize(image, &ImageConfiguration::empty()),
            None => Ok(image),
        }
    }

    fn read_uncached_async(self, done: ImageLoadCallback) {
        let Self { asset, resize } = self;
        match resize {
            None => asset.read_uncached_async(done),
            Some(size) => asset.read_uncached_async(Box::new(move |outcome| {
                done(outcome.and_then(|image| size.resize(image, &ImageConfiguration::empty())));
            })),
        }
    }
}

thread_local! {
    static SHARED: OnceCell<BundleImageCache> = const { OnceCell::new() };
    static PENDING_STREAMS: RefCell<PendingStreamMemo> = RefCell::new(PendingStreamMemo {
        entries: HashMap::new(),
        next_sweep: 32,
    });
}

/// The current UI isolate's decoded bundle images. A worker uses the
/// handle captured by the initiating UI thread rather than its own cache.
#[must_use]
pub fn shared_bundle_image_cache() -> BundleImageCache {
    SHARED.with(|cache| cache.get_or_init(BundleImageCache::default).clone())
}

/// Drops the owning thread's decoded bundle cache and detaches pending
/// work without invalidating existing image handles.
pub fn clear_shared_bundle_image_cache() {
    SHARED.with(|value| {
        if let Some(cache) = value.get() {
            cache.clear();
        }
    });
}

/// Stops tracking this UI thread's live bundle streams without canceling
/// image consumers. An unused cache is not initialized to clear it.
pub fn clear_shared_bundle_live_images() {
    SHARED.with(|slot| {
        if let Some(cache) = slot.get() {
            cache.clear_live_images();
        }
    });
}

/// Cached decoded images, kept by exact bundle identity, physical key and
/// scale. Counts retained decoded pixel bytes, not temporary codec buffers.
/// A cache hit never rereads or copies encoded bytes.
#[derive(Clone)]
pub struct BundleImageCache(Arc<Mutex<State>>);

/// Whether an exact bundle key is waiting, retained, or used by an
/// active UI listener. The three states are independent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BundleImageCacheStatus {
    pub pending: bool,
    pub keep_alive: bool,
    pub live: bool,
}

struct State {
    limits: ImageCacheLimits,
    pending: HashMap<BundleDecodeKey, Pending>,
    blocking: HashMap<BundleDecodeKey, BlockingReads>,
    ready: LruCache<BundleDecodeKey, ImageHandle>,
    bytes: usize,
}

struct BlockingReads {
    ticket: Arc<()>,
    active: usize,
}

struct BlockingRead<'a> {
    state: &'a Mutex<State>,
    key: &'a BundleDecodeKey,
    ticket: Option<Arc<()>>,
}

impl BlockingRead<'_> {
    fn finish(&mut self, state: &mut State) -> bool {
        let Some(ticket) = self.ticket.take() else {
            return false;
        };
        let Entry::Occupied(mut entry) = state.blocking.entry(self.key.clone()) else {
            return false;
        };
        if !Arc::ptr_eq(&entry.get().ticket, &ticket) {
            return false;
        }
        entry.get_mut().active -= 1;
        if entry.get().active == 0 {
            entry.remove();
        }
        true
    }
}

impl Drop for BlockingRead<'_> {
    fn drop(&mut self) {
        if self.ticket.is_some() {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            self.finish(&mut state);
        }
    }
}

impl State {
    fn fits(&self, bytes: usize) -> bool {
        self.limits.max_entries != 0 && self.limits.max_bytes != 0 && bytes <= self.limits.max_bytes
    }

    fn evict_excess(&mut self, removed: &mut Vec<(BundleDecodeKey, ImageHandle)>) {
        while self.ready.len() > self.limits.max_entries || self.bytes > self.limits.max_bytes {
            let Some((key, image)) = self.ready.pop_lru() else {
                break;
            };
            self.bytes -= image.size_bytes();
            removed.push((key, image));
        }
    }

    fn admit(
        &mut self,
        key: BundleDecodeKey,
        image: &ImageHandle,
        removed: &mut Vec<(BundleDecodeKey, ImageHandle)>,
    ) -> ImageHandle {
        if let Some(warm) = self.ready.get(&key) {
            // A separate synchronous read might have finished while the
            // callback path was decoding; both should publish one identity.
            return warm.clone();
        }
        let bytes = image.size_bytes();
        if self.fits(bytes) {
            self.bytes += bytes;
            self.ready.put(key, image.clone());
            self.evict_excess(removed);
        }
        image.clone()
    }
}

impl Default for BundleImageCache {
    fn default() -> Self {
        Self::with_limits(ImageCacheLimits::default())
    }
}

impl BundleImageCache {
    #[must_use]
    pub fn with_limits(limits: ImageCacheLimits) -> Self {
        Self(Arc::new(Mutex::new(State {
            limits,
            pending: HashMap::new(),
            blocking: HashMap::new(),
            ready: LruCache::unbounded(),
            bytes: 0,
        })))
    }

    #[must_use]
    pub fn limits(&self) -> ImageCacheLimits {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).limits
    }

    /// Decreasing the limits immediately evicts the least recent images.
    /// Setting either limit to zero also detaches pending cache entries.
    pub fn set_limits(&self, limits: ImageCacheLimits) {
        let (removed, detached) = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if state.limits == limits {
                return;
            }
            state.limits = limits;
            let mut removed = Vec::new();
            state.evict_excess(&mut removed);
            let detached = if limits.max_entries == 0 || limits.max_bytes == 0 {
                (
                    std::mem::take(&mut state.pending),
                    std::mem::take(&mut state.blocking),
                )
            } else {
                Default::default()
            };
            (removed, detached)
        };
        drop((removed, detached));
    }

    /// Number and bytes of decoded images currently retained.
    #[must_use]
    pub fn retained(&self) -> (usize, usize) {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        (state.ready.len(), state.bytes)
    }

    /// Number of cached decode jobs still waiting for an outcome. Joining
    /// an existing key does not add another job.
    #[must_use]
    pub fn pending_image_count(&self) -> usize {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.pending.len()
            + state
                .blocking
                .keys()
                .filter(|key| !state.pending.contains_key(*key))
                .count()
    }

    /// Whether a key is pending or has a retained decoded image.
    #[must_use]
    pub fn contains_key(&self, key: &AssetBundleImageKey) -> bool {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let key = BundleDecodeKey::plain(key.clone());
        state.ready.contains(&key)
            || state.pending.contains_key(&key)
            || state.blocking.contains_key(&key)
    }

    /// Reports the storage and active-listener state of this exact key
    /// from the owning UI thread, without changing its LRU order.
    #[must_use]
    pub fn status_for_key(&self, key: &AssetBundleImageKey) -> BundleImageCacheStatus {
        let key = BundleDecodeKey::plain(key.clone());
        let (pending, blocking, keep_alive) = {
            let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            (
                state.pending.get(&key).cloned(),
                state.blocking.contains_key(&key),
                state.ready.contains(&key),
            )
        };
        BundleImageCacheStatus {
            pending: pending.is_some() || blocking,
            keep_alive,
            live: live_streams::is_live(self, &key, pending.as_ref()),
        }
    }

    /// Number of exact keys with a live stream on this UI thread.
    #[must_use]
    pub fn live_image_count(&self) -> usize {
        let active: HashSet<usize> = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pending
            .values()
            .map(|job| Arc::as_ptr(job) as usize)
            .collect();
        live_streams::count(self, &active)
    }

    /// Stops tracking live stream views without canceling consumers or
    /// evicting the retained or pending decoded entries.
    pub fn clear_live_images(&self) {
        live_streams::clear(self);
    }

    /// Detaches an individual pending decode and/or releases its retained
    /// image. Existing waiters still receive the original outcome.
    pub fn evict(&self, key: &AssetBundleImageKey) -> bool {
        self.evict_with_live(key, true)
    }

    /// Evicts the pending or retained decode. If `include_live` is false,
    /// an externally observed stream can still satisfy a later resolve.
    pub fn evict_with_live(&self, key: &AssetBundleImageKey, include_live: bool) -> bool {
        let key = BundleDecodeKey::plain(key.clone());
        if include_live {
            live_streams::remove(self, &key);
        }
        let (removed, detached, blocking) = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let removed = state.ready.pop_entry(&key);
            let detached = state.pending.remove_entry(&key);
            let blocking = state.blocking.remove_entry(&key);
            if let Some((_, image)) = &removed {
                state.bytes -= image.size_bytes();
            }
            (removed, detached, blocking)
        };
        removed.is_some() || detached.is_some() || blocking.is_some()
    }

    /// Detaches every pending decode and releases all retained images.
    pub fn clear(&self) {
        let detached = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            state.bytes = 0;
            (
                std::mem::replace(&mut state.ready, LruCache::unbounded()),
                std::mem::take(&mut state.pending),
                std::mem::take(&mut state.blocking),
            )
        };
        drop(detached);
    }

    /// Loads the key synchronously when the bundle can block.
    pub fn load(&self, key: &AssetBundleImageKey) -> Result<ImageHandle, ImageError> {
        self.load_with_size(key.clone(), None)
    }

    pub(crate) fn load_with_size(
        &self,
        asset: AssetBundleImageKey,
        resize: Option<ImageResizeOptions>,
    ) -> Result<ImageHandle, ImageError> {
        let key = BundleDecodeKey { asset, resize };
        let ticket = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(image) = state.ready.get(&key) {
                return Ok(image.clone());
            }
            // Like the async path, latch whether retention is enabled
            // before invoking the loader. A budget change during a cold
            // read must not admit work that started outside the cache.
            if !state.fits(0) {
                drop(state);
                return key.read_uncached();
            }
            let reads = state
                .blocking
                .entry(key.clone())
                .or_insert_with(|| BlockingReads {
                    ticket: Arc::new(()),
                    active: 0,
                });
            reads.active += 1;
            reads.ticket.clone()
        };
        let mut read = BlockingRead {
            state: &self.0,
            key: &key,
            ticket: Some(ticket),
        };
        let image = key.read_uncached()?;
        let mut removed = Vec::new();
        let image = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if read.finish(&mut state) {
                state.admit(key.clone(), &image, &mut removed)
            } else {
                image
            }
        };
        drop(removed);
        Ok(image)
    }

    /// Reuses retained pixels or joins the already-started load for a
    /// selected key. This never starts I/O on a cache miss.
    pub(crate) fn resolve_cached(
        &self,
        asset: AssetBundleImageKey,
        resize: Option<ImageResizeOptions>,
    ) -> Option<ImageStream> {
        self.resolve_cached_key(&BundleDecodeKey { asset, resize })
    }

    fn resolve_cached_key(&self, key: &BundleDecodeKey) -> Option<ImageStream> {
        let (ready, pending) = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let ready = state.ready.get(key).cloned();
            let pending = state.pending.get(key).cloned();
            (ready, pending)
        };
        if let Some(pending) = pending {
            let stream = stream_for_pending(&pending);
            live_streams::remember(self, key, &stream, Some(&pending));
            return Some(stream);
        }
        if let Some(stream) = live_streams::stream(self, key) {
            if ready.is_none()
                && let crate::ImageLoadState::Ready(image) = stream.untracked_state()
            {
                let mut removed = Vec::new();
                {
                    let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
                    let _ = state.admit(key.clone(), &image, &mut removed);
                }
                drop(removed);
            }
            return Some(stream);
        }
        let stream = ImageStream::ready(ready?);
        live_streams::remember(self, key, &stream, None);
        Some(stream)
    }

    /// Resolves an exact asset key on the current UI thread. Concurrent
    /// callers on this thread share its existing pending stream; a ready
    /// cache hit does not start any I/O.
    #[must_use]
    pub fn resolve_stream(&self, key: AssetBundleImageKey) -> ImageStream {
        self.resolve_stream_with_size(key, None)
    }

    /// Resolves the key and returns a synchronous loader failure directly.
    /// A result that is still pending keeps its normal typed stream error
    /// path; this does not block for the worker.
    pub fn resolve_stream_checked(
        &self,
        key: AssetBundleImageKey,
    ) -> Result<ImageStream, ImageError> {
        let stream = self.resolve_stream(key);
        if let crate::ImageLoadState::Failed(error) = stream.untracked_state() {
            return Err(error);
        }
        Ok(stream)
    }

    /// Resolves a key and delivers a synchronous loader error to one caller
    /// supplied handler, returning no stream in that case.
    pub fn resolve_stream_on_error(
        &self,
        key: AssetBundleImageKey,
        on_error: impl FnOnce(ImageError),
    ) -> Option<ImageStream> {
        match self.resolve_stream_checked(key) {
            Ok(stream) => Some(stream),
            Err(error) => {
                on_error(error);
                None
            }
        }
    }

    pub(crate) fn resolve_stream_with_size(
        &self,
        asset: AssetBundleImageKey,
        resize: Option<ImageResizeOptions>,
    ) -> ImageStream {
        let key = BundleDecodeKey { asset, resize };
        if let Some(stream) = self.resolve_cached_key(&key) {
            return stream;
        }
        let mut pending = None;
        let mut stream = ImageStream::from_load_callback(|done| {
            pending = self.start_async_with_key(key.clone(), done);
        });
        if let Some(pending) = &pending {
            // A reentrant provider can resolve the same key while it
            // starts. Use the already-published view rather than replacing
            // the view another caller has received.
            let existing = PENDING_STREAMS.with(|memo| memo.borrow().get(pending));
            if let Some(existing) = existing {
                stream = existing;
            } else {
                PENDING_STREAMS.with(|memo| memo.borrow_mut().remember(pending, &stream));
            }
        }
        live_streams::remember(self, &key, &stream, pending.as_ref());
        stream
    }

    pub(crate) fn prepare_stream(
        &self,
        asset: AssetBundleImageKey,
        resize: Option<ImageResizeOptions>,
    ) -> PreparedBundleStream {
        let key = BundleDecodeKey { asset, resize };
        let (send, result) = futures_channel::oneshot::channel();
        let pending = self.start_async_with_key(
            key.clone(),
            Box::new(move |result| { let _ = send.send(result); }),
        );
        PreparedBundleStream { key, pending, result }
    }

    pub(crate) fn install_prepared_stream(&self, prepared: PreparedBundleStream) -> ImageStream {
        let PreparedBundleStream { key, pending, result } = prepared;
        let stream = if let Some(existing) = pending.as_ref().and_then(|pending| {
            PENDING_STREAMS.with(|memo| memo.borrow().get(pending))
        }) {
            existing
        } else {
            let stream = ImageStream::from_load_receiver(result);
            if let Some(pending) = &pending {
                PENDING_STREAMS.with(|memo| memo.borrow_mut().remember(pending, &stream));
            }
            stream
        };
        // A worker may complete a detached job while its UI handoff
        // waits. Never let that old view replace the new job's tracking.
        let resolved = stream.untracked_state();
        let current = {
            let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            match state.pending.get(&key) {
                Some(active) => pending.as_ref().is_some_and(|work| Arc::ptr_eq(work, active)),
                None => match (state.ready.peek(&key), resolved) {
                    (Some(current), crate::ImageLoadState::Ready(result)) => *current == result,
                    (None, _) => pending.is_none(),
                    _ => false,
                },
            }
        };
        if current {
            live_streams::remember(self, &key, &stream, pending.as_ref());
        }
        stream
    }

    /// Joins an already pending load for the exact key, or starts one. A
    /// warm result is returned immediately. No bundle code or user callback
    /// is called while the cache lock is held.
    pub fn load_async(&self, key: AssetBundleImageKey, done: ImageLoadCallback) {
        self.load_async_with_size(key, None, done);
    }

    pub(crate) fn load_async_with_size(
        &self,
        asset: AssetBundleImageKey,
        resize: Option<ImageResizeOptions>,
        done: ImageLoadCallback,
    ) {
        let _ = self.start_async_with_key(BundleDecodeKey { asset, resize }, done);
    }

    fn start_async_with_key(
        &self,
        key: BundleDecodeKey,
        done: ImageLoadCallback,
    ) -> Option<Pending> {
        let pending = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(image) = state.ready.get(&key).cloned() {
                drop(state);
                done(Ok(image));
                return None;
            }
            if !state.fits(0) {
                drop(state);
                key.read_uncached_async(done);
                return None;
            }
            match state.pending.entry(key.clone()) {
                Entry::Occupied(entry) => {
                    let pending = entry.get().clone();
                    drop(state);
                    attach(&pending, done);
                    return Some(pending);
                }
                Entry::Vacant(entry) => {
                    let pending = Arc::new(Mutex::new(PendingImage::Loading(vec![done])));
                    entry.insert(pending.clone());
                    pending
                }
            }
        };
        let work = pending.clone();
        let cache = Arc::downgrade(&self.0);
        let completed = key.clone();
        key.read_uncached_async(Box::new(move |mut result| {
            let mut removed = Vec::new();
            let mut detached = None;
            if let Some(cache) = cache.upgrade() {
                let mut state = cache.lock().unwrap_or_else(PoisonError::into_inner);
                if state
                    .pending
                    .get(&completed)
                    .is_some_and(|p| Arc::ptr_eq(p, &pending))
                {
                    detached = state.pending.remove_entry(&completed);
                    if let Ok(image) = &result {
                        result = Ok(state.admit(completed, image, &mut removed));
                    }
                }
            }
            drop((detached, removed));
            let previous = {
                let mut pending = pending.lock().unwrap_or_else(PoisonError::into_inner);
                std::mem::replace(&mut *pending, PendingImage::Completed(result.clone()))
            };
            if let PendingImage::Loading(callbacks) = previous {
                for callback in callbacks {
                    callback(result.clone());
                }
            }
        }));
        Some(work)
    }
}
