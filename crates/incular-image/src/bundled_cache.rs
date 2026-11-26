//! Bounded decoded-resource cache keyed by a Flutter bundle image key.

use std::{
    cell::OnceCell,
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, Mutex, PoisonError},
};

use lru::LruCache;

use crate::{
    AssetBundleImageKey, ImageCacheLimits, ImageConfiguration, ImageError, ImageHandle,
    ImageLoadCallback, ImageResizeOptions, ImageStream,
};

type Pending = Arc<Mutex<PendingImage>>;

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

/// Cached decoded images, kept by exact bundle identity, physical key and
/// scale. Counts retained decoded pixel bytes, not temporary codec buffers.
/// A cache hit never rereads or copies encoded bytes.
#[derive(Clone)]
pub struct BundleImageCache(Arc<Mutex<State>>);

struct State {
    limits: ImageCacheLimits,
    pending: HashMap<BundleDecodeKey, Pending>,
    ready: LruCache<BundleDecodeKey, ImageHandle>,
    bytes: usize,
    generation: u64,
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
            ready: LruCache::unbounded(),
            bytes: 0,
            generation: 0,
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
                state.generation = state.generation.wrapping_add(1);
                std::mem::take(&mut state.pending)
            } else {
                HashMap::new()
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
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pending
            .len()
    }

    /// Whether a key is pending or has a retained decoded image.
    #[must_use]
    pub fn contains_key(&self, key: &AssetBundleImageKey) -> bool {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let key = BundleDecodeKey::plain(key.clone());
        state.ready.contains(&key) || state.pending.contains_key(&key)
    }

    /// Detaches an individual pending decode and/or releases its retained
    /// image. Existing waiters still receive the original outcome.
    pub fn evict(&self, key: &AssetBundleImageKey) -> bool {
        let key = BundleDecodeKey::plain(key.clone());
        let (removed, detached) = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let removed = state.ready.pop_entry(&key);
            let detached = state.pending.remove_entry(&key);
            state.generation = state.generation.wrapping_add(1);
            if let Some((_, image)) = &removed {
                state.bytes -= image.size_bytes();
            }
            (removed, detached)
        };
        removed.is_some() || detached.is_some()
    }

    /// Detaches every pending decode and releases all retained images.
    pub fn clear(&self) {
        let (removed, detached) = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            state.generation = state.generation.wrapping_add(1);
            state.bytes = 0;
            (
                std::mem::replace(&mut state.ready, LruCache::unbounded()),
                std::mem::take(&mut state.pending),
            )
        };
        drop((removed, detached));
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
        let generation = {
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
            state.generation
        };
        let image = key.read_uncached()?;
        let mut removed = Vec::new();
        let image = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if state.generation == generation {
                state.admit(key, &image, &mut removed)
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
        let key = BundleDecodeKey { asset, resize };
        let pending = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(image) = state.ready.get(&key) {
                return Some(ImageStream::ready(image.clone()));
            }
            state.pending.get(&key).cloned()?
        };
        Some(ImageStream::from_load_callback(|done| {
            attach(&pending, done)
        }))
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
        let key = BundleDecodeKey { asset, resize };
        let pending = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(image) = state.ready.get(&key).cloned() {
                drop(state);
                done(Ok(image));
                return;
            }
            if !state.fits(0) {
                drop(state);
                key.read_uncached_async(done);
                return;
            }
            match state.pending.entry(key.clone()) {
                Entry::Occupied(entry) => {
                    let pending = entry.get().clone();
                    drop(state);
                    attach(&pending, done);
                    return;
                }
                Entry::Vacant(entry) => {
                    let pending = Arc::new(Mutex::new(PendingImage::Loading(vec![done])));
                    entry.insert(pending.clone());
                    pending
                }
            }
        };
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
    }
}
