//! Font handles and non-raster resource foundations for Incular.
//!
//! Raster image loading deliberately lives in `incular-image`, which keeps
//! renderer and widget users from depending on unrelated asset categories.

use std::{fmt, sync::Arc};

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

/// Font bytes and their stable identity. Paths are deliberately not part of
/// the public identity: system discovery and bundled assets both produce this
/// same handle.
#[derive(Clone, PartialEq, Eq)]
pub struct FontHandle {
    id: FontId,
    bytes: FontBytes,
    face_index: u32,
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
        }
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
            .finish_non_exhaustive()
    }
}
