//! Hot-reload asset invalidation hooks.
//!
//! Compiled only with the `hot-reload` feature; builds without it are
//! unaffected. Cached assets ([`crate::CachingAssetBundle`], typed loads
//! that route through it, ...) can be invalidated and reloaded cleanly:
//!
//! - [`AssetBundle::hot_reload_invalidate`] /
//!   [`AssetBundle::hot_reload_invalidate_all`] drop cached state for one
//!   bundle, as [`AssetBundle::evict`] / [`AssetBundle::clear`] do;
//! - [`crate::hot_reload::invalidate`], [`crate::hot_reload::invalidate_key`], [`crate::hot_reload::invalidate_path`] and
//!   [`crate::hot_reload::invalidate_all`] broadcast an [`crate::hot_reload::AssetInvalidation`] to every
//!   [`crate::hot_reload::subscribe`]r;
//! - [`crate::hot_reload::watch`] wires a shared bundle so broadcasts invalidate its caches
//!   automatically.
//!
//! Reloading after invalidation is ordinary loading: call `load*` again
//! and the bundle re-reads its source. Logical keys resolved through
//! [`crate::PathOverrideBundle`] evict their physical targets too.

use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, PoisonError, Weak},
};

use crate::AssetBundle;

/// What changed during a hot-reload invalidation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetInvalidation {
    /// One logical asset key changed; cached entries for it are dropped.
    /// Path-override targets are resolved when evicting.
    Key(String),
    /// One physical file changed. Bundles that cannot map it back to keys
    /// conservatively invalidate everything.
    Path(PathBuf),
    /// Every cached asset changed.
    All,
}

impl AssetInvalidation {
    /// A key-change invalidation.
    #[must_use]
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key(key.into())
    }

    /// A physical-file-change invalidation.
    #[must_use]
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }

    /// An invalidate-everything invalidation.
    #[must_use]
    pub fn all() -> Self {
        Self::All
    }

    /// Whether this invalidation should drop cached `key`. `Key` matches
    /// exactly; `Path` and `All` match everything (the cache cannot prove
    /// a key is unaffected).
    #[must_use]
    pub fn affects_key(&self, key: &str) -> bool {
        match self {
            Self::Key(changed) => changed == key,
            Self::Path(_) | Self::All => true,
        }
    }
}

/// A registered invalidation hook.
pub type AssetInvalidationHook = Arc<dyn Fn(&AssetInvalidation) + Send + Sync + 'static>;

#[derive(Default)]
struct Registry {
    next_id: u64,
    hooks: Vec<(u64, AssetInvalidationHook)>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Mutex::default);

fn registry() -> &'static Mutex<Registry> {
    &REGISTRY
}

/// RAII subscription; dropping it unregisters the hook.
///
/// ```
/// # #[cfg(feature = "hot-reload")]
/// # fn main() {
/// let _sub = incular_assets::hot_reload::subscribe(|_| {});
/// incular_assets::hot_reload::invalidate_all();
/// // `_sub` dropped here; no further calls.
/// # }
/// # #[cfg(not(feature = "hot-reload"))] fn main() {}
/// ```
pub struct AssetInvalidationSubscription {
    id: Option<u64>,
}

impl AssetInvalidationSubscription {
    /// Unregisters explicitly; dropping does the same.
    pub fn unsubscribe(mut self) {
        self.remove();
    }

    fn remove(&mut self) {
        if let Some(id) = self.id.take() {
            registry()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .hooks
                .retain(|(hook_id, _)| *hook_id != id);
        }
    }
}

impl Drop for AssetInvalidationSubscription {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Registers `hook`, called with every [`crate::hot_reload::invalidate`] until the returned
/// subscription is dropped. Hooks run synchronously in registration
/// order; hooks must not block. A hook may itself invalidate.
#[must_use]
pub fn subscribe(
    hook: impl Fn(&AssetInvalidation) + Send + Sync + 'static,
) -> AssetInvalidationSubscription {
    let mut registry = registry().lock().unwrap_or_else(PoisonError::into_inner);
    let id = registry.next_id;
    registry.next_id = registry.next_id.wrapping_add(1);
    registry.hooks.push((id, Arc::new(hook)));
    AssetInvalidationSubscription { id: Some(id) }
}

/// Broadcasts `invalidation` to all subscribers. Does not itself touch
/// any bundle's caches; pair with [`crate::hot_reload::watch`], or call
/// [`AssetBundle::hot_reload_invalidate`] directly.
pub fn invalidate(invalidation: AssetInvalidation) {
    // Clone hooks out of the lock so reentrant invalidation cannot deadlock.
    let hooks: Vec<AssetInvalidationHook> = registry()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .hooks
        .iter()
        .map(|(_, hook)| hook.clone())
        .collect();
    for hook in hooks {
        hook(&invalidation);
    }
}

/// Broadcasts [`AssetInvalidation::Key`].
pub fn invalidate_key(key: impl Into<String>) {
    invalidate(AssetInvalidation::key(key));
}

/// Broadcasts [`AssetInvalidation::Path`] for a changed physical file.
pub fn invalidate_path(path: impl Into<PathBuf>) {
    invalidate(AssetInvalidation::path(path));
}

/// Broadcasts [`AssetInvalidation::All`].
pub fn invalidate_all() {
    invalidate(AssetInvalidation::All);
}

/// Wires a shared bundle so broadcasts invalidate its caches: a `Key`
/// evicts that key, a `Path` or `All` clears. The bundle is held weakly;
/// dropping it stops delivery. The subscription can also be dropped to
/// stop delivery explicitly.
#[must_use]
pub fn watch<B: AssetBundle>(bundle: &Arc<B>) -> AssetInvalidationSubscription {
    let weak: Weak<B> = Arc::downgrade(bundle);
    subscribe(move |invalidation| {
        let Some(bundle) = weak.upgrade() else { return };
        match invalidation {
            AssetInvalidation::Key(key) => bundle.hot_reload_invalidate(key),
            AssetInvalidation::Path(_) | AssetInvalidation::All => {
                bundle.hot_reload_invalidate_all()
            }
        }
    })
}
