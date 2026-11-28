//! UI listener ownership for the decoded bundle cache.

use super::*;
use incular_assets::AssetBundle;

#[derive(Hash, PartialEq, Eq)]
struct Key {
    owner: usize,
    bundle: usize,
    name: String,
    scale: u64,
    resize: Option<ImageResizeOptions>,
}

impl Key {
    fn new(cache: &BundleImageCache, key: &BundleDecodeKey) -> Self {
        Self {
            owner: Arc::as_ptr(&cache.0) as usize,
            bundle: Arc::as_ptr(key.asset.bundle.shared()) as *const () as usize,
            name: key.asset.name.clone(),
            scale: crate::asset_image::comparable_scale(key.asset.scale),
            resize: key.resize,
        }
    }
}

struct Entry {
    owner: Weak<Mutex<State>>,
    bundle: Weak<dyn AssetBundle>,
    stream: crate::stream::WeakImageStream,
    pending: Option<Weak<PendingDecode>>,
}

struct Memo {
    entries: HashMap<Key, Entry>,
    next_sweep: usize,
}

thread_local! {
    static LIVE: RefCell<Memo> = RefCell::new(Memo {
        entries: HashMap::new(),
        next_sweep: 32,
    });
}

pub(super) fn remember(
    cache: &BundleImageCache,
    key: &BundleDecodeKey,
    stream: &ImageStream,
    pending: Option<&Pending>,
) {
    LIVE.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.entries.len() >= memo.next_sweep {
            memo.entries.retain(|_, entry| {
                entry.owner.strong_count() != 0
                    && entry.bundle.strong_count() != 0
                    && (entry.stream.is_live()
                        || entry
                            .pending
                            .as_ref()
                            .is_some_and(|job| job.strong_count() != 0))
            });
            memo.next_sweep = memo.entries.len().saturating_mul(2).max(32);
        }
        memo.entries.insert(
            Key::new(cache, key),
            Entry {
                owner: Arc::downgrade(&cache.0),
                bundle: Arc::downgrade(key.asset.bundle.shared()),
                stream: stream.downgrade(),
                pending: pending.map(Arc::downgrade),
            },
        );
    });
}

pub(super) fn stream(cache: &BundleImageCache, key: &BundleDecodeKey) -> Option<ImageStream> {
    let stream = LIVE.with(|memo| {
        memo.borrow()
            .entries
            .get(&Key::new(cache, key))?
            .stream
            .upgrade()
    })?;
    (stream.has_live_listeners()
        && !matches!(stream.untracked_state(), crate::ImageLoadState::Failed(_)))
    .then_some(stream)
}

pub(super) fn is_live(
    cache: &BundleImageCache,
    key: &BundleDecodeKey,
    pending: Option<&Pending>,
) -> bool {
    LIVE.with(|memo| {
        let memo = memo.borrow();
        let Some(entry) = memo.entries.get(&Key::new(cache, key)) else {
            return false;
        };
        let pending = pending.is_some_and(|job| {
            entry
                .pending
                .as_ref()
                .is_some_and(|old| old.as_ptr() == Arc::as_ptr(job))
        });
        pending
            || entry
                .stream
                .upgrade()
                .is_some_and(|stream| stream.has_live_listeners())
    })
}

pub(super) fn count(cache: &BundleImageCache, pending: &HashSet<usize>) -> usize {
    let owner = Arc::as_ptr(&cache.0) as usize;
    LIVE.with(|memo| {
        memo.borrow()
            .entries
            .iter()
            .filter(|(key, entry)| {
                key.owner == owner
                    && (entry
                        .pending
                        .as_ref()
                        .is_some_and(|job| pending.contains(&(job.as_ptr() as usize)))
                        || entry
                            .stream
                            .upgrade()
                            .is_some_and(|stream| stream.has_live_listeners()))
            })
            .count()
    })
}

pub(super) fn remove(cache: &BundleImageCache, key: &BundleDecodeKey) {
    let _ = LIVE.try_with(|memo| memo.borrow_mut().entries.remove(&Key::new(cache, key)));
}

pub(super) fn clear(cache: &BundleImageCache) {
    let owner = Arc::as_ptr(&cache.0) as usize;
    let _ = LIVE.try_with(|memo| {
        memo.borrow_mut()
            .entries
            .retain(|key, _| key.owner != owner)
    });
}
