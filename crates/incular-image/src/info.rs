use std::{fmt, sync::Arc};

use crate::{ImageHandle, finite_positive_or_one};

/// A decoded image reference together with the scale and label for this use.
///
/// Cloning shares the decoded pixels; it does not decode or allocate a
/// second pixel buffer. Dropping the info only releases its own reference.
pub struct ImageInfo {
    image: ImageHandle,
    scale: f64,
    debug_label: Option<Arc<str>>,
    #[cfg(debug_assertions)]
    _lifetime: incular_core::memory::allocations::ObjectLifetime,
}

impl ImageInfo {
    #[must_use]
    pub fn new(image: ImageHandle) -> Self {
        Self {
            image,
            scale: 1.0,
            debug_label: None,
            #[cfg(debug_assertions)]
            _lifetime: incular_core::memory::allocations::ObjectLifetime::new(
                "painting",
                "ImageInfo",
            ),
        }
    }

    /// Sets the number of encoded pixels per logical pixel. Invalid scales
    /// use one, like the image-provider scale policy.
    #[must_use]
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = finite_positive_or_one(scale);
        self
    }

    #[must_use]
    pub fn with_debug_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.debug_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn image(&self) -> &ImageHandle {
        &self.image
    }

    #[must_use]
    pub fn into_image(self) -> ImageHandle {
        self.image
    }

    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }

    #[must_use]
    pub fn debug_label(&self) -> Option<&str> {
        self.debug_label.as_deref()
    }

    /// Decoded RGBA8 bytes, independent of the display scale and label.
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        self.image.size_bytes()
    }

    #[must_use]
    pub fn logical_size(&self) -> (f64, f64) {
        let image = self.image.decoded();
        (
            f64::from(image.width()) / self.scale,
            f64::from(image.height()) / self.scale,
        )
    }

    /// Checks shared resource identity as well as scale and diagnostic label.
    /// A separately decoded image with identical bytes is not a clone.
    #[must_use]
    pub fn is_clone_of(&self, other: &Self) -> bool {
        self.image == other.image
            && self.scale == other.scale
            && self.debug_label == other.debug_label
    }
}

impl Clone for ImageInfo {
    fn clone(&self) -> Self {
        let mut result = Self::new(self.image.clone());
        result.scale = self.scale;
        result.debug_label.clone_from(&self.debug_label);
        result
    }
}

impl fmt::Debug for ImageInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageInfo")
            .field("image", &self.image)
            .field("scale", &self.scale)
            .field("debug_label", &self.debug_label)
            .finish()
    }
}
