//! Logical-to-physical asset path overrides.
//!
//! [`PathOverrides`] maps a logical asset key to a different physical key
//! before a bundle resolves it; [`PathOverrideBundle`] applies a live set
//! of overrides to any wrapped [`AssetBundle`]. Existing byte, string,
//! typed and manifest APIs keep working; only key resolution changes.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{Arc, Mutex, PoisonError},
};

use crate::{
    AssetBundle, AssetError, AssetManifest, AssetManifestFormat, AssetManifestLoadCallback,
};

/// An ordered map from logical asset keys to physical keys.
///
/// Overrides apply once: an override target is not itself re-resolved, so
/// override chains and cycles cannot occur. Unknown keys pass through
/// unchanged. Logical keys stay `/`-separated, as in a Flutter pubspec.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathOverrides {
    overrides: BTreeMap<Arc<str>, Arc<str>>,
}

impl PathOverrides {
    /// An empty override set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Maps `logical` to `physical`, returning any previous mapping.
    pub fn insert(
        &mut self,
        logical: impl Into<Arc<str>>,
        physical: impl Into<Arc<str>>,
    ) -> Option<Arc<str>> {
        self.overrides.insert(logical.into(), physical.into())
    }

    /// Removes the override for `logical`, returning it if any.
    pub fn remove(&mut self, logical: &str) -> Option<Arc<str>> {
        self.overrides.remove(logical)
    }

    /// The physical key `logical` maps to, if overridden.
    #[must_use]
    pub fn get(&self, logical: &str) -> Option<&str> {
        self.overrides.get(logical).map(Arc::as_ref)
    }

    /// Whether `logical` has an override.
    #[must_use]
    pub fn contains(&self, logical: &str) -> bool {
        self.overrides.contains_key(logical)
    }

    /// Number of overrides.
    #[must_use]
    pub fn len(&self) -> usize {
        self.overrides.len()
    }

    /// Whether there are no overrides.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }

    /// Removes every override.
    pub fn clear(&mut self) {
        self.overrides.clear();
    }

    /// Iterates logical/physical pairs in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.overrides
            .iter()
            .map(|(logical, physical)| (logical.as_ref(), physical.as_ref()))
    }

    /// Resolves `key` to its physical key. An unoverridden key borrows;
    /// an overridden key returns the mapped target.
    #[must_use]
    pub fn resolve<'a>(&self, key: &'a str) -> Cow<'a, str> {
        match self.overrides.get(key) {
            Some(physical) => Cow::Owned(physical.to_string()),
            None => Cow::Borrowed(key),
        }
    }
}

impl FromIterator<(Arc<str>, Arc<str>)> for PathOverrides {
    fn from_iter<I: IntoIterator<Item = (Arc<str>, Arc<str>)>>(iter: I) -> Self {
        Self {
            overrides: iter.into_iter().collect(),
        }
    }
}

impl FromIterator<(String, String)> for PathOverrides {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self {
            overrides: iter
                .into_iter()
                .map(|(logical, physical)| (Arc::from(logical), Arc::from(physical)))
                .collect(),
        }
    }
}

/// A bundle applying live [`PathOverrides`] to logical keys before
/// delegating to `inner`.
///
/// Resolution happens once per call. Errors are re-keyed to the logical
/// key. Caches inside `inner` (e.g. [`crate::CachingAssetBundle`]) key on
/// the physical key; eviction resolves overrides so `evict`/`clear`
/// invalidate the same entries.
pub struct PathOverrideBundle<B> {
    inner: B,
    overrides: Mutex<PathOverrides>,
}

impl<B> PathOverrideBundle<B> {
    /// Wraps `inner` with an empty override set.
    #[must_use]
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            overrides: Mutex::new(PathOverrides::new()),
        }
    }

    /// Wraps `inner` with `overrides`.
    #[must_use]
    pub fn with_overrides(inner: B, overrides: PathOverrides) -> Self {
        Self {
            inner,
            overrides: Mutex::new(overrides),
        }
    }

    /// Builder form of [`PathOverrides::insert`].
    #[must_use]
    pub fn with_path_override(
        self,
        logical: impl Into<Arc<str>>,
        physical: impl Into<Arc<str>>,
    ) -> Self {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(logical, physical);
        self
    }

    /// Adds or replaces an override, returning any previous mapping.
    pub fn add_path_override(
        &self,
        logical: impl Into<Arc<str>>,
        physical: impl Into<Arc<str>>,
    ) -> Option<Arc<str>> {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(logical, physical)
    }

    /// Removes an override, returning it if any.
    pub fn remove_path_override(&self, logical: &str) -> Option<Arc<str>> {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(logical)
    }

    /// Replaces the whole override set.
    pub fn set_path_overrides(&self, overrides: PathOverrides) {
        *self
            .overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = overrides;
    }

    /// Removes every override.
    pub fn clear_path_overrides(&self) {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// A snapshot of the current overrides.
    #[must_use]
    pub fn path_overrides(&self) -> PathOverrides {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Resolves a logical key to its physical key under current overrides.
    #[must_use]
    pub fn resolve_key(&self, key: &str) -> String {
        self.overrides
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .resolve(key)
            .into_owned()
    }

    /// The wrapped bundle.
    pub fn inner(&self) -> &B {
        &self.inner
    }

    /// The wrapped bundle, mutably.
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    /// Unwraps, returning the inner bundle.
    #[must_use]
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: std::fmt::Debug> std::fmt::Debug for PathOverrideBundle<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathOverrideBundle")
            .field("inner", &self.inner)
            .field("overrides", &self.path_overrides())
            .finish()
    }
}

impl<B: AssetBundle> AssetBundle for PathOverrideBundle<B> {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        let physical = self.resolve_key(key);
        self.inner
            .load(&physical)
            .map_err(|error| rekey(error, key))
    }

    fn load_string(&self, key: &str, cache: bool) -> Result<Arc<str>, AssetError> {
        let physical = self.resolve_key(key);
        self.inner
            .load_string(&physical, cache)
            .map_err(|error| rekey(error, key))
    }

    fn load_async(&self, key: &str, done: crate::AssetLoadCallback) {
        let physical = self.resolve_key(key);
        let logical = key.to_owned();
        self.inner.load_async(
            &physical,
            Box::new(move |result| done(result.map_err(|error| rekey(error, &logical)))),
        );
    }

    fn cached_asset_manifest(&self, _format: AssetManifestFormat) -> Option<AssetManifest> {
        // `inner` caches under physical keys; nothing is exposed here.
        None
    }

    fn read_asset_manifest(
        &self,
        format: AssetManifestFormat,
    ) -> Result<AssetManifest, AssetError> {
        // Read through `load`, which resolves overrides.
        let key = format.asset_key();
        AssetManifest::from_bytes(&self.load(key)?, format)
    }

    fn load_asset_manifest(
        self: Arc<Self>,
        format: AssetManifestFormat,
        done: AssetManifestLoadCallback,
    ) {
        // Uses `load_async`, which resolves overrides.
        self.load_async(
            format.asset_key(),
            Box::new(move |bytes| {
                done(bytes.and_then(|bytes| AssetManifest::from_bytes(&bytes, format)))
            }),
        );
    }

    fn evict(&self, key: &str) {
        let physical = self.resolve_key(key);
        self.inner.evict(key);
        if physical != key {
            self.inner.evict(&physical);
        }
    }

    fn clear(&self) {
        self.inner.clear();
    }
}

/// Rewrites an error's key back to the logical key.
fn rekey(error: AssetError, logical: &str) -> AssetError {
    match error {
        AssetError::NotFound(_) => AssetError::NotFound(logical.to_owned()),
        AssetError::InvalidUtf8(_) => AssetError::InvalidUtf8(logical.to_owned()),
        AssetError::Io { message, .. } => AssetError::Io {
            key: logical.to_owned(),
            message,
        },
        AssetError::Parse { message, .. } => AssetError::Parse {
            key: logical.to_owned(),
            message,
        },
        AssetError::TypeMismatch { .. } => AssetError::TypeMismatch {
            key: logical.to_owned(),
        },
    }
}
