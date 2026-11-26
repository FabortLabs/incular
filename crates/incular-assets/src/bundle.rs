//! Flutter's `AssetBundle` (`services/asset_bundle.dart`): named byte
//! resources an application ships with.

use std::{
    any::Any,
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use crate::{AssetManifest, AssetManifestFormat, AssetManifestLoadCallback};

/// Why an asset could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetError {
    /// Flutter's `Unable to load asset`: the bundle has no such key.
    NotFound(String),
    /// The asset is not UTF-8, so it has no string form.
    InvalidUtf8(String),
    /// The bundle's storage failed while reading the key.
    Io { key: String, message: String },
    /// A structured-data parser rejected the asset.
    Parse { key: String, message: String },
    /// The key's cached structured value has a different type than the one
    /// requested.
    TypeMismatch { key: String },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(key) => write!(
                f,
                "Unable to load asset: \"{key}\".\nThe asset does not exist or has empty data."
            ),
            Self::InvalidUtf8(key) => write!(f, "Asset \"{key}\" is not UTF-8."),
            Self::Io { key, message } => write!(f, "Unable to load asset \"{key}\": {message}"),
            Self::Parse { key, message } => write!(f, "Unable to parse asset \"{key}\": {message}"),
            Self::TypeMismatch { key } => {
                write!(f, "Asset \"{key}\" is cached as a different type.")
            }
        }
    }
}

impl std::error::Error for AssetError {}

/// A collection of named resources. Keys are `/`-separated relative paths,
/// as in a Flutter pubspec.
pub trait AssetBundle: Send + Sync + 'static {
    /// `AssetBundle.load`: the asset's bytes.
    ///
    /// # Errors
    /// [`AssetError::NotFound`] for an unknown key, or the storage error.
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError>;

    /// `AssetBundle.loadString`: the asset decoded as UTF-8. A caching
    /// bundle keeps the string when `cache` is true.
    ///
    /// # Errors
    /// As [`Self::load`], or [`AssetError::InvalidUtf8`].
    fn load_string(&self, key: &str, cache: bool) -> Result<Arc<str>, AssetError> {
        let _ = cache;
        decode(key, &self.load(key)?)
    }

    /// `AssetBundle.load` as Flutter's future: calls `done` once with the
    /// asset's bytes. The default loads synchronously through [`Self::load`]
    /// and calls `done` before returning. A bundle over a slow source, such
    /// as [`NetworkAssetBundle`](crate::NetworkAssetBundle), calls it later,
    /// possibly on another thread, and never blocks.
    fn load_async(&self, key: &str, done: crate::AssetLoadCallback) {
        done(self.load(key));
    }

    /// Returns only an already cached manifest, without fetching or parsing.
    fn cached_asset_manifest(&self, _format: AssetManifestFormat) -> Option<AssetManifest> {
        None
    }

    /// Reads a manifest immediately from a synchronous bundle. Applications
    /// with a callback-only transport use [Self::load_asset_manifest].
    fn read_asset_manifest(
        &self,
        format: AssetManifestFormat,
    ) -> Result<AssetManifest, AssetError> {
        AssetManifest::from_bytes(&self.load(format.asset_key())?, format)
    }

    /// Starts a manifest load. The default uses the bundle's asynchronous
    /// byte transport, without caching the decoded result.
    fn load_asset_manifest(
        self: Arc<Self>,
        format: AssetManifestFormat,
        done: AssetManifestLoadCallback,
    ) {
        self.load_async(
            format.asset_key(),
            Box::new(move |bytes| {
                done(bytes.and_then(|bytes| AssetManifest::from_bytes(&bytes, format)))
            }),
        );
    }

    /// `AssetBundle.evict`: forgets anything cached for `key`.
    fn evict(&self, key: &str) {
        let _ = key;
    }

    /// `AssetBundle.clear`: forgets everything cached.
    fn clear(&self) {}
}

fn decode(key: &str, bytes: &[u8]) -> Result<Arc<str>, AssetError> {
    std::str::from_utf8(bytes)
        .map(Arc::from)
        .map_err(|_| AssetError::InvalidUtf8(key.to_owned()))
}

impl dyn AssetBundle {
    /// `AssetBundle.loadStructuredData`: the asset's string parsed by
    /// `parser`. Bundles do not cache parsed values; a
    /// [`CachingAssetBundle`] does through its own method.
    ///
    /// # Errors
    /// As [`AssetBundle::load_string`], or the parser's message as
    /// [`AssetError::Parse`].
    pub fn load_structured_data<T, E: fmt::Display>(
        &self,
        key: &str,
        parser: impl FnOnce(&str) -> Result<T, E>,
    ) -> Result<T, AssetError> {
        let text = self.load_string(key, false)?;
        parser(&text).map_err(|error| AssetError::Parse {
            key: key.to_owned(),
            message: error.to_string(),
        })
    }

    /// Parses a named asset from its raw bytes, without caching the value.
    ///
    /// # Errors
    ///
    /// As [AssetBundle::load], or the binary parser's error.
    pub fn load_structured_binary_data<T, E: fmt::Display>(
        &self,
        key: &str,
        parser: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<T, AssetError> {
        let data = self.load(key)?;
        parser(&data).map_err(|error| parse_error(key, error))
    }
}

/// A bundle held in memory, for bundled `include_bytes!` data and tests.
#[derive(Clone, Default)]
pub struct MemoryAssetBundle {
    assets: HashMap<String, Arc<[u8]>>,
}

impl MemoryAssetBundle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces the asset at `key`.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        self.insert(key, bytes);
        self
    }

    /// Adds or replaces the asset at `key`.
    pub fn insert(&mut self, key: impl Into<String>, bytes: impl Into<Arc<[u8]>>) {
        self.assets.insert(key.into(), bytes.into());
    }
}

impl fmt::Debug for MemoryAssetBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<_> = self.assets.keys().collect();
        keys.sort();
        f.debug_struct("MemoryAssetBundle")
            .field("keys", &keys)
            .finish()
    }
}

impl AssetBundle for MemoryAssetBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.assets
            .get(key)
            .cloned()
            .ok_or_else(|| AssetError::NotFound(key.to_owned()))
    }
}

/// A bundle read from a directory, as Flutter's platform bundle reads the
/// application's packaged assets. Keys may not leave the directory.
#[derive(Clone, Debug)]
pub struct DirectoryAssetBundle {
    root: PathBuf,
}

impl DirectoryAssetBundle {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl AssetBundle for DirectoryAssetBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        let mut path = self.root.clone();
        for part in key.split('/') {
            // Empty, current and parent segments would reach outside the root.
            if matches!(part, "" | "." | "..") || part.contains('\\') {
                return Err(AssetError::NotFound(key.to_owned()));
            }
            path.push(part);
        }
        std::fs::read(&path)
            .map(Arc::from)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => AssetError::NotFound(key.to_owned()),
                _ => AssetError::Io {
                    key: key.to_owned(),
                    message: error.to_string(),
                },
            })
    }
}

fn parse_error(key: &str, error: impl fmt::Display) -> AssetError {
    AssetError::Parse {
        key: key.to_owned(),
        message: error.to_string(),
    }
}

type PendingManifest = Arc<Mutex<Vec<AssetManifestLoadCallback>>>;

type StructuredCache = HashMap<String, Arc<dyn Any + Send + Sync>>;

#[derive(Clone, Copy)]
enum StructuredKind {
    Text,
    Binary,
}

#[derive(Default)]
struct CachedAssets {
    strings: HashMap<String, Arc<str>>,
    structured: StructuredCache,
    binary: StructuredCache,
    pending_manifests: HashMap<&'static str, PendingManifest>,
    generation: u64,
}

impl CachedAssets {
    fn structured_cache(&self, kind: StructuredKind) -> &StructuredCache {
        match kind {
            StructuredKind::Text => &self.structured,
            StructuredKind::Binary => &self.binary,
        }
    }

    fn structured_cache_mut(&mut self, kind: StructuredKind) -> &mut StructuredCache {
        match kind {
            StructuredKind::Text => &mut self.structured,
            StructuredKind::Binary => &mut self.binary,
        }
    }
}

/// Flutter's `CachingAssetBundle`: keeps strings, and parsed structured
/// data, until they are evicted. Bytes are not cached, as in Flutter.
pub struct CachingAssetBundle<B> {
    inner: B,
    cache: Mutex<CachedAssets>,
}

impl<B: AssetBundle> CachingAssetBundle<B> {
    #[must_use]
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            cache: Mutex::default(),
        }
    }

    /// `CachingAssetBundle.loadStructuredData`: the parsed value, cached by
    /// key until evicted. A failed parse caches nothing.
    ///
    /// # Errors
    /// As [`AssetBundle::load_string`], the parser's message as
    /// [`AssetError::Parse`], or [`AssetError::TypeMismatch`] when a value of
    /// another type is cached for `key` (where Flutter's cast throws).
    pub fn load_structured_data<T: Clone + Send + Sync + 'static, E: fmt::Display>(
        &self,
        key: &str,
        parser: impl FnOnce(&str) -> Result<T, E>,
    ) -> Result<T, AssetError> {
        self.load_cached_structured(key, StructuredKind::Text, || {
            let text = self.load_string(key, false)?;
            parser(&text).map_err(|error| parse_error(key, error))
        })
    }

    /// Parses a binary asset once per cache generation. Text, parsed text
    /// and parsed binary assets each have a separate cache namespace.
    ///
    /// # Errors
    ///
    /// As [AssetBundle::load], a parser error, or a cached type mismatch.
    pub fn load_structured_binary_data<T: Clone + Send + Sync + 'static, E: fmt::Display>(
        &self,
        key: &str,
        parser: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<T, AssetError> {
        self.load_cached_structured(key, StructuredKind::Binary, || {
            let bytes = self.load(key)?;
            parser(&bytes).map_err(|error| parse_error(key, error))
        })
    }

    fn load_cached_structured<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        kind: StructuredKind,
        fetch_and_parse: impl FnOnce() -> Result<T, AssetError>,
    ) -> Result<T, AssetError> {
        let (cached, generation) = {
            let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            (
                cache.structured_cache(kind).get(key).cloned(),
                cache.generation,
            )
        };
        if let Some(value) = cached {
            return value
                .downcast_ref::<T>()
                .cloned()
                .ok_or_else(|| AssetError::TypeMismatch {
                    key: key.to_owned(),
                });
        }
        let value = fetch_and_parse()?;
        let shared = Arc::new(value.clone());
        let previous = {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            if cache.generation == generation {
                cache
                    .structured_cache_mut(kind)
                    .insert(key.to_owned(), shared)
            } else {
                None
            }
        };
        // A custom cached value may run user code in its destructor.
        drop(previous);
        Ok(value)
    }
}

impl<B: fmt::Debug> fmt::Debug for CachingAssetBundle<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachingAssetBundle")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<B: AssetBundle> AssetBundle for CachingAssetBundle<B> {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.inner.load(key)
    }

    fn load_async(&self, key: &str, done: crate::AssetLoadCallback) {
        self.inner.load_async(key, done);
    }

    fn cached_asset_manifest(&self, format: AssetManifestFormat) -> Option<AssetManifest> {
        let key = format.asset_key();
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .structured_cache(manifest_kind(format))
            .get(key)
            .and_then(|value| value.downcast_ref::<AssetManifest>())
            .cloned()
    }

    fn read_asset_manifest(
        &self,
        format: AssetManifestFormat,
    ) -> Result<AssetManifest, AssetError> {
        let key = format.asset_key();
        self.load_cached_structured(key, manifest_kind(format), || {
            AssetManifest::from_bytes(&self.load(key)?, format)
        })
    }

    fn load_asset_manifest(
        self: Arc<Self>,
        format: AssetManifestFormat,
        done: AssetManifestLoadCallback,
    ) {
        let key = format.asset_key();
        let kind = manifest_kind(format);
        let (pending, generation) = {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(value) = cache.structured_cache(kind).get(key).cloned() {
                drop(cache);
                done(cached_manifest(&value, key));
                return;
            }
            if let Some(pending) = cache.pending_manifests.get(key) {
                pending
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(done);
                return;
            }
            let pending = Arc::new(Mutex::new(vec![done]));
            cache.pending_manifests.insert(key, pending.clone());
            (pending, cache.generation)
        };
        let bundle = Arc::downgrade(&self);
        self.inner.load_async(
            key,
            Box::new(move |bytes| {
                let mut result = bytes.and_then(|bytes| AssetManifest::from_bytes(&bytes, format));
                if let Some(bundle) = bundle.upgrade() {
                    let mut cache = bundle.cache.lock().unwrap_or_else(PoisonError::into_inner);
                    if cache
                        .pending_manifests
                        .get(key)
                        .is_some_and(|current| Arc::ptr_eq(current, &pending))
                    {
                        cache.pending_manifests.remove(key);
                        if cache.generation == generation
                            && let Ok(value) = &result
                        {
                            // A synchronous typed reader might have populated
                            // the same key while the asynchronous read ran.
                            let value = cache
                                .structured_cache_mut(kind)
                                .entry(key.to_owned())
                                .or_insert_with(|| Arc::new(value.clone()));
                            result = cached_manifest(value, key);
                        }
                    }
                }
                let callbacks =
                    std::mem::take(&mut *pending.lock().unwrap_or_else(PoisonError::into_inner));
                for callback in callbacks {
                    callback(result.clone());
                }
            }),
        );
    }

    fn load_string(&self, key: &str, cache: bool) -> Result<Arc<str>, AssetError> {
        if !cache {
            return decode(key, &self.inner.load(key)?);
        }
        let generation = {
            let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(text) = cache.strings.get(key) {
                return Ok(text.clone());
            }
            cache.generation
        };
        let text = decode(key, &self.inner.load(key)?)?;
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.generation == generation {
            cache.strings.insert(key.to_owned(), text.clone());
        }
        Ok(text)
    }

    fn evict(&self, key: &str) {
        let removed = {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            cache.generation = cache.generation.wrapping_add(1);
            (
                cache.strings.remove(key),
                cache.structured.remove(key),
                cache.binary.remove(key),
                cache.pending_manifests.remove(key),
            )
        };
        drop(removed);
        self.inner.evict(key);
    }

    fn clear(&self) {
        let removed = {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            let generation = cache.generation.wrapping_add(1);
            std::mem::replace(
                &mut *cache,
                CachedAssets {
                    generation,
                    ..CachedAssets::default()
                },
            )
        };
        drop(removed);
        self.inner.clear();
    }
}

fn cached_manifest(
    value: &Arc<dyn Any + Send + Sync>,
    key: &str,
) -> Result<AssetManifest, AssetError> {
    value
        .downcast_ref::<AssetManifest>()
        .cloned()
        .ok_or_else(|| AssetError::TypeMismatch {
            key: key.to_owned(),
        })
}

const fn manifest_kind(format: AssetManifestFormat) -> StructuredKind {
    match format {
        AssetManifestFormat::Binary => StructuredKind::Binary,
        AssetManifestFormat::WebJson => StructuredKind::Text,
    }
}
