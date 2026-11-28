//! Typed asset handles and non-blocking typed loading.
//!
//! The byte and string bundle APIs stay as they are; this module layers
//! typed identities and loading states on top. A [`TypedAsset`] decodes
//! bundle bytes into a concrete type, an [`AssetHandle`] names one typed
//! asset in a bundle, and loads report clear [`AssetLoadState`]s through a
//! callback or a [`AssetFuture`]. Raster images stay in `incular-image`;
//! the built-in typed assets here are fonts ([`FontAsset`]) and shader
//! sources ([`ShaderAsset`]).

use std::{
    fmt,
    marker::PhantomData,
    sync::{Arc, Mutex, PoisonError},
    task::{Context, Poll, Waker},
};

use crate::{AssetBundle, AssetError, FontBytes, FontHandle, FontId, FontLocation};

/// Stable identity for an asset, independent of its logical key and of
/// which bundle currently resolves it.
///
/// Identities are only compared for equality. [`AssetHandle::new`] hands
/// out process-unique ids; [`AssetHandle::with_id`] uses a caller-owned id,
/// like [`FontId`] does for fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(pub u64);

/// A typed asset decoded from bundle bytes.
///
/// Implement this for application asset types to load them with
/// [`dyn AssetBundle::load_typed`], [`dyn AssetBundle::load_typed_async`],
/// [`dyn AssetBundle::load_typed_future`] or a typed [`AssetHandle`]. Structured
/// text formats can instead reuse [`dyn AssetBundle::load_structured_data`].
pub trait TypedAsset: Send + Sync + 'static {
    /// Decodes the asset's bytes.
    ///
    /// `key` is the logical (or overridden physical) key the bytes were
    /// loaded from; it feeds error context and format detection.
    ///
    /// # Errors
    /// A structured [`AssetError`], typically [`AssetError::InvalidUtf8`]
    /// or [`AssetError::Parse`].
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError>
    where
        Self: Sized;
}

/// Raw bytes are already a typed asset.
impl TypedAsset for Vec<u8> {
    fn decode_asset(_key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        Ok(bytes.to_vec())
    }
}

/// Shared raw bytes are already a typed asset.
impl TypedAsset for Arc<[u8]> {
    fn decode_asset(_key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        Ok(Arc::from(bytes))
    }
}

/// A UTF-8 string is a typed asset.
impl TypedAsset for String {
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        Ok(utf8(key, bytes)?.to_owned())
    }
}

/// A shared UTF-8 string is a typed asset.
impl TypedAsset for Arc<str> {
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        Ok(Arc::from(utf8(key, bytes)?))
    }
}

fn utf8<'a>(key: &str, bytes: &'a [u8]) -> Result<&'a str, AssetError> {
    std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidUtf8(key.to_owned()))
}

/// Lifecycle of a typed asset load.
///
/// The same states drive callbacks, [`AssetFuture`] polling and state
/// queries, matching the crate's `AssetError` result style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetLoadState<T> {
    /// The load has not finished; no value or error is available yet.
    Loading,
    /// The asset loaded and decoded.
    Ready(T),
    /// Loading or decoding failed.
    Failed(AssetError),
}

impl<T> AssetLoadState<T> {
    /// Builds a terminal state from a load result.
    #[must_use]
    pub fn from_result(result: Result<T, AssetError>) -> Self {
        match result {
            Ok(value) => Self::Ready(value),
            Err(error) => Self::Failed(error),
        }
    }

    /// Whether the load is still in flight.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Whether a value is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Whether loading or decoding failed.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Whether loading or decoding failed; alias of [`Self::is_failed`].
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.is_failed()
    }

    /// Builds a [`Self::Failed`] state.
    #[must_use]
    pub fn failed(error: AssetError) -> Self {
        Self::Failed(error)
    }

    /// The ready value, if any.
    #[must_use]
    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Loading | Self::Failed(_) => None,
        }
    }

    /// Consumes the state, returning the ready value if any.
    #[must_use]
    pub fn into_ready(self) -> Option<T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Loading | Self::Failed(_) => None,
        }
    }

    /// The error, if failed.
    #[must_use]
    pub fn error(&self) -> Option<&AssetError> {
        match self {
            Self::Failed(error) => Some(error),
            Self::Loading | Self::Ready(_) => None,
        }
    }

    /// Maps a ready value, preserving `Loading` and `Failed`.
    #[must_use]
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> AssetLoadState<U> {
        match self {
            Self::Loading => AssetLoadState::Loading,
            Self::Ready(value) => AssetLoadState::Ready(map(value)),
            Self::Failed(error) => AssetLoadState::Failed(error),
        }
    }
}

/// Receives the outcome of one typed load.
pub type TypedAssetLoadCallback<T> = Box<dyn FnOnce(Result<T, AssetError>) + Send>;

/// A typed handle to one named asset.
///
/// Cheap to clone. `T` is the decoded type; the key stays a `/`-separated
/// logical path, and any [path overrides](crate::PathOverrides) of the
/// bundle still apply at load time.
pub struct AssetHandle<T: TypedAsset> {
    id: AssetId,
    key: Arc<str>,
    _asset: PhantomData<fn() -> T>,
}

static NEXT_ASSET_ID: incular_core::IdGenerator = incular_core::IdGenerator::new();

impl<T: TypedAsset> AssetHandle<T> {
    /// A handle with a fresh process-unique [`AssetId`] for `key`.
    #[must_use]
    pub fn new(key: impl Into<Arc<str>>) -> Self {
        Self::with_id(AssetId(NEXT_ASSET_ID.next()), key)
    }

    /// A handle with a caller-owned identity for `key`.
    #[must_use]
    pub fn with_id(id: AssetId, key: impl Into<Arc<str>>) -> Self {
        Self {
            id,
            key: key.into(),
            _asset: PhantomData,
        }
    }

    /// Stable identity of this handle.
    #[must_use]
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// The logical asset key loaded from a bundle.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Loads and decodes this asset synchronously.
    ///
    /// # Errors
    /// As [`dyn AssetBundle::load_typed`].
    pub fn load(&self, bundle: &dyn AssetBundle) -> Result<T, AssetError> {
        bundle.load_typed::<T>(self.key())
    }

    /// Starts a non-blocking load; `done` runs exactly once.
    pub fn load_async(&self, bundle: &dyn AssetBundle, done: TypedAssetLoadCallback<T>) {
        bundle.load_typed_async::<T>(self.key(), done);
    }

    /// Starts a non-blocking load as a future.
    ///
    /// The future is `Send`; a slow bundle completes it from another
    /// thread. A bundle that completes synchronously yields an
    /// already-ready future. State while waiting is
    /// [`AssetLoadState::Loading`]; [`AssetFuture::state`] reports the
    /// terminal state once ready.
    #[must_use]
    pub fn load_future(&self, bundle: &dyn AssetBundle) -> AssetFuture<T> {
        bundle.load_typed_future::<T>(self.key())
    }
}

impl<T: TypedAsset> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            key: self.key.clone(),
            _asset: PhantomData,
        }
    }
}

impl<T: TypedAsset> fmt::Debug for AssetHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetHandle")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("asset", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T: TypedAsset> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.key == other.key
    }
}

impl<T: TypedAsset> Eq for AssetHandle<T> {}

impl<T: TypedAsset> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.key.hash(state);
    }
}

/// A [`std::future::Future`] over one non-blocking typed asset load.
///
/// Resolves to `Result<T, AssetError>`. Cancellation-safe: dropping the
/// future leaves the underlying load running and simply stops observing
/// it. The future is `Send` and does not require any async runtime.
pub struct AssetFuture<T> {
    slot: Arc<Mutex<AssetFutureSlot<T>>>,
}

struct AssetFutureSlot<T> {
    result: Option<Result<T, AssetError>>,
    waker: Option<Waker>,
}

impl<T> AssetFuture<T> {
    pub(crate) fn start(begin: impl FnOnce(TypedAssetLoadCallback<T>)) -> Self
    where
        T: Send + 'static,
    {
        let slot = Arc::new(Mutex::new(AssetFutureSlot {
            result: None,
            waker: None,
        }));
        let completion = Arc::clone(&slot);
        begin(Box::new(move |result| {
            let mut slot = completion.lock().unwrap_or_else(PoisonError::into_inner);
            slot.result = Some(result);
            let waker = slot.waker.take();
            drop(slot);
            if let Some(waker) = waker {
                waker.wake();
            }
        }));
        Self { slot }
    }

    /// Current lifecycle state: [`AssetLoadState::Loading`] until the
    /// load completes, then `Ready` or `Failed`. Does not consume the
    /// future and does not affect polling.
    #[must_use]
    pub fn state(&self) -> AssetLoadState<T>
    where
        T: Clone,
    {
        let slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        match &slot.result {
            None => AssetLoadState::Loading,
            Some(Ok(value)) => AssetLoadState::Ready(value.clone()),
            Some(Err(error)) => AssetLoadState::Failed(error.clone()),
        }
    }

    /// Whether the load has not finished yet.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .result
            .is_none()
    }

    /// Whether the load has finished, ready or failed. Does not consume
    /// the future; polling still yields the result.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.is_loading()
    }
}

impl<T> std::future::Future for AssetFuture<T> {
    type Output = Result<T, AssetError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// A typed font asset: font source bytes plus identity metadata.
///
/// The bytes are handed to `incular-text` unchanged; registering them
/// (choosing a family name, shaping) stays in `incular-text`. Raster image
/// types are deliberately not here; they live in `incular-image`.
#[derive(Clone, PartialEq, Eq)]
pub struct FontAsset {
    key: Arc<str>,
    bytes: FontBytes,
    face_index: u32,
    location: FontLocation,
    family: Option<Arc<str>>,
}

impl FontAsset {
    /// A font asset for `key` wrapping `bytes`.
    #[must_use]
    pub fn new(key: impl Into<Arc<str>>, bytes: impl Into<FontBytes>) -> Self {
        Self {
            key: key.into(),
            bytes: bytes.into(),
            face_index: 0,
            location: FontLocation::default(),
            family: None,
        }
    }

    /// The logical asset key the font was loaded from.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Shared, cheaply cloned font source bytes.
    #[must_use]
    pub fn bytes(&self) -> &FontBytes {
        &self.bytes
    }

    /// Face index for OpenType collections.
    #[must_use]
    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Selects a face in an OpenType collection.
    #[must_use]
    pub fn with_face_index(mut self, face_index: u32) -> Self {
        self.face_index = face_index;
        self
    }

    /// OpenType variation coordinates.
    #[must_use]
    pub fn location(&self) -> &FontLocation {
        &self.location
    }

    /// Selects the shaped instance.
    #[must_use]
    pub fn with_location(mut self, location: FontLocation) -> Self {
        self.location = location;
        self
    }

    /// An application-suggested family name, used by `incular-text`
    /// registration when the caller has no explicit choice.
    #[must_use]
    pub fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    /// Sets an application-suggested family name.
    #[must_use]
    pub fn with_family(mut self, family: impl Into<Arc<str>>) -> Self {
        self.family = Some(family.into());
        self
    }

    /// The `incular-text` font handle for `id`, retaining face index and
    /// variation location. The caller owns `id`'s identity.
    #[must_use]
    pub fn to_font_handle(&self, id: FontId) -> FontHandle {
        FontHandle::with_face_index(id, self.bytes.clone(), self.face_index)
            .with_location(self.location.clone())
    }

    /// Consumes the asset, returning owned font source bytes for APIs that
    /// take `Vec<u8>`, such as `incular-text`'s `FontLoader`.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes.as_ref().to_vec()
    }
}

impl fmt::Debug for FontAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontAsset")
            .field("key", &self.key)
            .field("face_index", &self.face_index)
            .field("location", &self.location)
            .field("family", &self.family)
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

/// `incular-text` `FontLoader::add_font` accepts `B: Into<Vec<u8>>`.
impl From<FontAsset> for Vec<u8> {
    fn from(asset: FontAsset) -> Self {
        asset.into_bytes()
    }
}

impl TypedAsset for FontAsset {
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        Ok(Self::new(key, FontBytes::from(bytes.to_vec())))
    }
}

/// The shader source language, inferred from the asset key's extension.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShaderLanguage {
    /// WebGPU shading language (`.wgsl`); the default.
    #[default]
    Wgsl,
    /// OpenGL shading language (`.glsl`, `.frag`, `.vert`, `.comp`, ...).
    Glsl,
    /// High-level shading language (`.hlsl`, `.fx`).
    Hlsl,
    /// Metal shading language (`.metal`, `.msl`).
    Metal,
    /// Binary SPIR-V (`.spv`, `.spirv`); not UTF-8 text.
    SpirV,
}

impl ShaderLanguage {
    /// Infers the language from `key`'s file extension; unknown or missing
    /// extensions are [`ShaderLanguage::Wgsl`].
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        let extension = key
            .rsplit('/')
            .next()
            .unwrap_or(key)
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase());
        match extension.as_deref() {
            Some(
                "glsl" | "frag" | "vert" | "comp" | "geom" | "tesc" | "tese" | "rgen" | "rchit"
                | "rmiss" | "rahit" | "rint" | "rcall",
            ) => Self::Glsl,
            Some("hlsl" | "fx" | "fxh" | "cg") => Self::Hlsl,
            Some("metal" | "msl") => Self::Metal,
            Some("spv" | "spirv") => Self::SpirV,
            _ => Self::Wgsl,
        }
    }

    /// Whether source in this language is UTF-8 text.
    #[must_use]
    pub fn is_text(self) -> bool {
        !matches!(self, Self::SpirV)
    }
}

/// A typed shader asset: shader source bytes plus identity and language.
///
/// Compiling or uploading shaders stays with the renderer (`incular-wgpu`);
/// this handle only carries the source. `incular-assets` has no GPU
/// dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderAsset {
    key: Arc<str>,
    source: Arc<[u8]>,
    language: ShaderLanguage,
}

impl ShaderAsset {
    /// A shader asset for `key` with explicit `language`.
    #[must_use]
    pub fn new(
        key: impl Into<Arc<str>>,
        source: impl Into<Arc<[u8]>>,
        language: ShaderLanguage,
    ) -> Self {
        Self {
            key: key.into(),
            source: source.into(),
            language,
        }
    }

    /// The logical asset key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The raw source bytes.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }

    /// The inferred or explicit source language.
    #[must_use]
    pub fn language(&self) -> ShaderLanguage {
        self.language
    }

    /// The source as UTF-8 text.
    ///
    /// # Errors
    /// [`AssetError::InvalidUtf8`] for binary shaders such as SPIR-V, and
    /// for invalid UTF-8 in a text shader.
    pub fn source_text(&self) -> Result<&str, AssetError> {
        if !self.language.is_text() {
            return Err(AssetError::InvalidUtf8(self.key.to_string()));
        }
        std::str::from_utf8(&self.source).map_err(|_| AssetError::InvalidUtf8(self.key.to_string()))
    }

    /// Selects a different source language.
    #[must_use]
    pub fn with_language(mut self, language: ShaderLanguage) -> Self {
        self.language = language;
        self
    }
}

impl TypedAsset for ShaderAsset {
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        let language = ShaderLanguage::from_key(key);
        if language.is_text() {
            // Text shaders must be valid UTF-8 to be usable source.
            std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidUtf8(key.to_owned()))?;
        }
        Ok(Self::new(key, Arc::<[u8]>::from(bytes), language))
    }
}
