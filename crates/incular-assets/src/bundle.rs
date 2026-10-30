//! Flutter's `AssetBundle` (`services/asset_bundle.dart`): named byte
//! resources an application ships with.

use std::{
    any::Any,
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

/// Why an asset could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    /// Flutter's `Unable to load asset`: the bundle has no such key.
    NotFound(String),
    /// The asset is not UTF-8, so it has no string form.
    InvalidUtf8(String),
    /// The bundle's storage failed while reading the key.
    Io { key: String, message: String },
    /// A structured-data parser rejected the asset.
    Parse { key: String, message: String },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(key) => write!(f, "Unable to load asset: \"{key}\"."),
            Self::InvalidUtf8(key) => write!(f, "Asset \"{key}\" is not UTF-8."),
            Self::Io { key, message } => write!(f, "Unable to load asset \"{key}\": {message}"),
            Self::Parse { key, message } => write!(f, "Unable to parse asset \"{key}\": {message}"),
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

type StructuredCache = HashMap<String, Arc<dyn Any + Send + Sync>>;

/// Flutter's `CachingAssetBundle`: keeps strings, and parsed structured
/// data, until they are evicted. Bytes are not cached, as in Flutter.
pub struct CachingAssetBundle<B> {
    inner: B,
    strings: Mutex<HashMap<String, Arc<str>>>,
    structured: Mutex<StructuredCache>,
}

impl<B: AssetBundle> CachingAssetBundle<B> {
    #[must_use]
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            strings: Mutex::default(),
            structured: Mutex::default(),
        }
    }

    /// `CachingAssetBundle.loadStructuredData`: the parsed value, cached by
    /// key until evicted. A failed parse caches nothing.
    ///
    /// # Errors
    /// As [`AssetBundle::load_string`], or the parser's message.
    ///
    /// # Panics
    /// Panics when a value of another type is cached for `key`, as Flutter's
    /// cast throws.
    pub fn load_structured_data<T: Clone + Send + Sync + 'static, E: fmt::Display>(
        &self,
        key: &str,
        parser: impl FnOnce(&str) -> Result<T, E>,
    ) -> Result<T, AssetError> {
        if let Some(value) = self
            .structured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
        {
            return Ok(value
                .downcast_ref::<T>()
                .unwrap_or_else(|| panic!("structured asset {key:?} has another type"))
                .clone());
        }
        let text = self.load_string(key, false)?;
        let value = parser(&text).map_err(|error| AssetError::Parse {
            key: key.to_owned(),
            message: error.to_string(),
        })?;
        self.structured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.to_owned(), Arc::new(value.clone()));
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

    fn load_string(&self, key: &str, cache: bool) -> Result<Arc<str>, AssetError> {
        if cache
            && let Some(text) = self
                .strings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(key)
        {
            return Ok(text.clone());
        }
        let text = decode(key, &self.inner.load(key)?)?;
        if cache {
            self.strings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(key.to_owned(), text.clone());
        }
        Ok(text)
    }

    fn evict(&self, key: &str) {
        self.strings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
        self.structured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
        self.inner.evict(key);
    }

    fn clear(&self) {
        self.strings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.structured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.inner.clear();
    }
}
