//! Ordered registration of the faces that make up an application font family.

use std::{
    convert::Infallible,
    error::Error,
    fmt,
    future::{Future, ready},
    pin::Pin,
    sync::Arc,
};

type Cause = Box<dyn Error + Send + Sync>;
type FontAsset = Pin<Box<dyn Future<Output = Result<Vec<u8>, Cause>>>>;

/// A font family operation failed. An attempted load consumes the loader's
/// queue; create a new loader to retry after a failure.
#[derive(Debug)]
pub enum FontLoaderError {
    AlreadyLoaded,
    Asset(Cause),
    Registration(Cause),
}

impl fmt::Display for FontLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyLoaded => f.write_str("font loader is already loaded"),
            Self::Asset(cause) => write!(f, "could not read a font asset: {cause}"),
            Self::Registration(cause) => write!(f, "could not register a font asset: {cause}"),
        }
    }
}

impl Error for FontLoaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AlreadyLoaded => None,
            Self::Asset(cause) | Self::Registration(cause) => Some(cause.as_ref()),
        }
    }
}

/// Loads the fonts for a family in insertion order. Each face finishes
/// registration before the next is delivered, even if later data is ready
/// sooner. The default registrar updates the current UI thread's text engines.
pub struct FontLoader {
    family: Arc<str>,
    assets: Vec<FontAsset>,
    loaded: bool,
}

impl FontLoader {
    #[must_use]
    pub fn new(family: impl Into<Arc<str>>) -> Self {
        Self {
            family: family.into(),
            assets: Vec::new(),
            loaded: false,
        }
    }

    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Queues a future yielding the bytes for one face or collection.
    /// The loader polls asset futures in insertion order.
    ///
    /// # Errors
    /// Returns `AlreadyLoaded` after a previous load attempt.
    pub fn add_font<B, E>(
        &mut self,
        asset: impl Future<Output = Result<B, E>> + 'static,
    ) -> Result<(), FontLoaderError>
    where
        B: Into<Vec<u8>> + 'static,
        E: Error + Send + Sync + 'static,
    {
        if self.loaded {
            return Err(FontLoaderError::AlreadyLoaded);
        }
        self.assets.push(Box::pin(async move {
            asset
                .await
                .map(Into::into)
                .map_err(|cause| Box::new(cause) as Cause)
        }));
        Ok(())
    }

    /// Queues already available font bytes under the same family.
    ///
    /// # Errors
    /// Returns `AlreadyLoaded` after a previous load attempt.
    pub fn add_bytes(&mut self, bytes: impl Into<Vec<u8>>) -> Result<(), FontLoaderError> {
        if self.loaded {
            return Err(FontLoaderError::AlreadyLoaded);
        }
        self.add_font(ready(Ok::<_, Infallible>(bytes.into())))
    }

    /// Submits the declared faces to the current thread's font registry.
    /// Existing text engines see the new family on subsequent layout.
    ///
    /// # Errors
    /// Returns `AlreadyLoaded` or the first asset error.
    pub async fn load(&mut self) -> Result<(), FontLoaderError> {
        self.load_with(|family, bytes| {
            crate::register_application_font_as(family, bytes);
            ready(Ok::<(), Infallible>(()))
        })
        .await
    }

    /// Uses a caller-provided registry. Previous successful registrations
    /// remain installed if a later asset or registry call fails.
    ///
    /// # Errors
    /// Returns `AlreadyLoaded`, or the first asset or registration error.
    pub async fn load_with<F, E>(
        &mut self,
        mut register: impl FnMut(Arc<str>, Vec<u8>) -> F,
    ) -> Result<(), FontLoaderError>
    where
        F: Future<Output = Result<(), E>>,
        E: Error + Send + Sync + 'static,
    {
        if std::mem::replace(&mut self.loaded, true) {
            return Err(FontLoaderError::AlreadyLoaded);
        }
        for asset in std::mem::take(&mut self.assets) {
            let bytes = asset.await.map_err(FontLoaderError::Asset)?;
            register(self.family.clone(), bytes)
                .await
                .map_err(|cause| FontLoaderError::Registration(Box::new(cause)))?;
        }
        Ok(())
    }
}
