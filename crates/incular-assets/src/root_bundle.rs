//! The fallback application bundle used by unscoped asset consumers.

use std::{
    cell::RefCell,
    sync::{Arc, OnceLock},
};

use crate::{AssetBundle, CachingAssetBundle, MemoryAssetBundle};

thread_local! {
    static OVERRIDE: RefCell<Option<Arc<dyn AssetBundle>>> = const { RefCell::new(None) };
}

/// The root component bundle for this UI thread. A host may supply its
/// packaged assets explicitly. The default is an empty, cached bundle.
#[must_use]
pub fn root_asset_bundle() -> Arc<dyn AssetBundle> {
    static EMPTY_ROOT: OnceLock<Arc<dyn AssetBundle>> = OnceLock::new();
    configured_root_asset_bundle().unwrap_or_else(|| {
        EMPTY_ROOT
            .get_or_init(|| Arc::new(CachingAssetBundle::new(MemoryAssetBundle::new())))
            .clone()
    })
}

/// The host's configured root bundle, excluding the empty fallback. File
/// adapters can use this to preserve their behavior without packaged assets.
#[must_use]
pub fn configured_root_asset_bundle() -> Option<Arc<dyn AssetBundle>> {
    OVERRIDE.with(|slot| slot.borrow().clone())
}

/// Sets or clears the root bundle override, returning the previous override.
/// Configure it before mounting the application. Already-retained consumers
/// see the new root on their next build, not during a layout pass.
pub fn replace_root_asset_bundle(
    replacement: Option<Arc<dyn AssetBundle>>,
) -> Option<Arc<dyn AssetBundle>> {
    OVERRIDE.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), replacement))
}
