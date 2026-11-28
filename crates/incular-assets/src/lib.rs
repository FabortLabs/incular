//! Font handles, asset bundles and non-raster resource foundations for
//! Incular.
//!
//! Raster image loading deliberately lives in `incular-image`, which keeps
//! renderer and widget users from depending on unrelated asset categories.

use std::{fmt, sync::Arc};

mod bundle;
mod consolidate_response;
/// Hot-reload asset invalidation hooks; only with `hot-reload`.
#[cfg(feature = "hot-reload")]
pub mod hot_reload;
mod manifest;
mod network;
pub mod notices;
mod path_overrides;
mod root_bundle;
mod typed;
pub use bundle::{
    AssetBundle, AssetError, CachingAssetBundle, DirectoryAssetBundle, MemoryAssetBundle,
};
pub use consolidate_response::{
    ConsolidateResponseError, HttpResponseCompressionState, HttpResponseOptions,
    consolidate_http_client_response_bytes, consolidate_http_client_response_bytes_with_progress,
};
pub use manifest::{AssetManifest, AssetManifestFormat, AssetManifestLoadCallback, AssetMetadata};
pub use network::{
    AssetFetchCallback, AssetFetcher, AssetLoadCallback, AssetResponse, NetworkAssetBundle,
    resolve_url,
};
pub use notices::{LicenseNoticeFormat, load_license_notices, register_license_notices};
pub use path_overrides::{PathOverrideBundle, PathOverrides};

pub use root_bundle::{configured_root_asset_bundle, replace_root_asset_bundle, root_asset_bundle};
pub use typed::{
    AssetFuture, AssetHandle, AssetId, AssetLoadState, FontAsset, ShaderAsset, ShaderLanguage,
    TypedAsset, TypedAssetLoadCallback,
};

/// Stable renderer-independent identity for a loaded font asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub u64);

/// Immutable, cheaply cloned font source bytes. The source may be a heap
/// allocation or a read-only file mapping owned by system font discovery, so
/// sharing a handle never copies the font file.
#[derive(Clone)]
pub struct FontBytes(Arc<dyn AsRef<[u8]> + Send + Sync>);

impl FontBytes {
    /// Shares an existing byte source without copying it.
    #[must_use]
    pub fn from_shared(source: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Self {
        Self(source)
    }
}
impl std::ops::Deref for FontBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}
impl AsRef<[u8]> for FontBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
impl PartialEq for FontBytes {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || **self == **other
    }
}
impl Eq for FontBytes {}
impl From<Arc<[u8]>> for FontBytes {
    fn from(bytes: Arc<[u8]>) -> Self {
        Self(Arc::new(bytes))
    }
}
impl From<Vec<u8>> for FontBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }
}
impl From<&'static [u8]> for FontBytes {
    fn from(bytes: &'static [u8]) -> Self {
        Self(Arc::new(bytes))
    }
}
impl<const N: usize> From<[u8; N]> for FontBytes {
    fn from(bytes: [u8; N]) -> Self {
        Self(Arc::new(bytes))
    }
}

/// OpenType variation coordinates in the face's axis order, after the face's
/// axis normalization and mapping. Values store the exact signed 2.14 bits.
///
/// Trailing zero coordinates are implicit. An empty location is the default
/// instance and needs no allocation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontLocation(Option<Arc<[i16]>>);

impl FontLocation {
    #[must_use]
    pub fn from_normalized_coordinates(coordinates: &[i16]) -> Self {
        let end = coordinates.iter().rposition(|&value| value != 0);
        Self(end.map(|end| Arc::from(&coordinates[..=end])))
    }

    #[must_use]
    pub fn coordinates(&self) -> &[i16] {
        self.0.as_deref().unwrap_or_default()
    }

    #[must_use]
    pub const fn is_default(&self) -> bool {
        self.0.is_none()
    }
}

/// Font bytes and their stable identity. Paths are deliberately not part of
/// the public identity: system discovery and bundled assets both produce this
/// same handle.
#[derive(Clone, PartialEq, Eq)]
pub struct FontHandle {
    id: FontId,
    bytes: FontBytes,
    face_index: u32,
    location: FontLocation,
}
impl FontHandle {
    #[must_use]
    pub fn new(id: FontId, bytes: impl Into<FontBytes>) -> Self {
        Self::with_face_index(id, bytes, 0)
    }
    /// Creates a handle for a face in an OpenType collection. The caller owns
    /// the identity: collection index must be included in `id`.
    #[must_use]
    pub fn with_face_index(id: FontId, bytes: impl Into<FontBytes>, face_index: u32) -> Self {
        Self {
            id,
            bytes: bytes.into(),
            face_index,
            location: FontLocation::default(),
        }
    }
    /// Selects the shaped instance without changing the face identity used
    /// to retain and share parsed font tables.
    #[must_use]
    pub fn with_location(mut self, location: FontLocation) -> Self {
        self.location = location;
        self
    }
    #[must_use]
    pub const fn location(&self) -> &FontLocation {
        &self.location
    }
    #[must_use]
    pub const fn id(&self) -> FontId {
        self.id
    }
    #[must_use]
    pub fn bytes(&self) -> &FontBytes {
        &self.bytes
    }
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
}
impl fmt::Debug for FontHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontHandle")
            .field("id", &self.id)
            .field("face_index", &self.face_index)
            .field("location", &self.location)
            .finish_non_exhaustive()
    }
}
