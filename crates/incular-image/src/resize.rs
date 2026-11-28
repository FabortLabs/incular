//! Flutter's exact and aspect-preserving decoded image sizing.

use std::sync::Arc;

use crate::{
    AnyImageProvider, DecodedImage, ImageConfiguration, ImageError, ImageHandle, ImageProvider,
    ImageStream,
};

/// How the cache dimensions constrain the decoded image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResizeImagePolicy {
    /// Clamp dimensions independently unless upscaling is allowed. When
    /// only one is supplied, the other preserves the source aspect ratio.
    #[default]
    Exact,
    /// Preserve the source aspect ratio inside the specified bounds.
    Fit,
}

/// Options shared by a `ResizeImage` and the widget cache-size builders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageResizeOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub policy: ResizeImagePolicy,
    pub allow_upscaling: bool,
    pub use_logical_size: bool,
}

impl ImageResizeOptions {
    /// Effective physical dimensions, before comparing them with the source.
    /// The pinned contract uses ceiling, independently for each axis.
    #[must_use]
    pub fn physical_size(self, configuration: &ImageConfiguration) -> (Option<u32>, Option<u32>) {
        let scale = if self.use_logical_size {
            configuration.target_device_pixel_ratio().unwrap_or(1.0)
        } else {
            1.0
        };
        (
            self.width.map(|w| (f64::from(w) * scale).ceil() as u32),
            self.height.map(|h| (f64::from(h) * scale).ceil() as u32),
        )
    }

    /// Canonical cache dimensions. Two requests that resolve to the same
    /// physical bounds have the same settings for cache identity.
    #[must_use]
    pub fn physical(self, configuration: &ImageConfiguration) -> Self {
        let (width, height) = self.physical_size(configuration);
        Self {
            width,
            height,
            use_logical_size: false,
            ..self
        }
    }

    /// The target raster size for the requested bounds and source aspect.
    ///
    /// # Errors
    ///
    /// Zero bounds and zero-sized output are invalid. Output also follows
    /// the same per-axis and byte allocation ceilings as regular decoding.
    pub fn target_size(
        self,
        source: (u32, u32),
        configuration: &ImageConfiguration,
    ) -> Result<(u32, u32), ImageError> {
        if source.0 == 0 || source.1 == 0 {
            return Err(ImageError::InvalidDimensions);
        }
        let (width, height) = self.physical_size(configuration);
        if width == Some(0) || height == Some(0) {
            return Err(ImageError::InvalidDimensions);
        }
        let (source_w, source_h) = source;
        let ratio = f64::from(source_w) / f64::from(source_h);
        let (target_w, target_h) = match self.policy {
            ResizeImagePolicy::Exact => {
                let width = width.map(|w| {
                    if self.allow_upscaling {
                        w
                    } else {
                        w.min(source_w)
                    }
                });
                let height = height.map(|h| {
                    if self.allow_upscaling {
                        h
                    } else {
                        h.min(source_h)
                    }
                });
                match (width, height) {
                    (Some(w), Some(h)) => (w, h),
                    (Some(w), None) => (w, (f64::from(w) / ratio).floor() as u32),
                    // The single-height engine path rounds to nearest.
                    (None, Some(h)) => ((f64::from(h) * ratio).round() as u32, h),
                    (None, None) => source,
                }
            }
            ResizeImagePolicy::Fit if self.allow_upscaling => match (width, height) {
                (Some(w), Some(h)) => (
                    w.min((f64::from(h) * ratio).floor() as u32),
                    h.min((f64::from(w) / ratio).floor() as u32),
                ),
                (Some(w), None) => (w, (f64::from(w) / ratio).floor() as u32),
                (None, Some(h)) => ((f64::from(h) * ratio).floor() as u32, h),
                (None, None) => source,
            },
            ResizeImagePolicy::Fit => {
                let mut target = source;
                if let Some(w) = width
                    && target.0 > w
                {
                    target = (w, (f64::from(w) / ratio).floor() as u32);
                }
                if let Some(h) = height
                    && target.1 > h
                {
                    target = ((f64::from(h) * ratio).floor() as u32, h);
                }
                target
            }
        };
        if target_w == 0 || target_h == 0 {
            return Err(ImageError::InvalidDimensions);
        }
        ImageHandle::check_decode_policy(target_w, target_h)?;
        Ok((target_w, target_h))
    }

    /// Applies these bounds to an existing frame without mutating it, and
    /// preserves density metadata. An unchanged target returns the original
    /// handle, without a copy or a new GPU resource identity.
    pub fn resize(
        self,
        image: ImageHandle,
        configuration: &ImageConfiguration,
    ) -> Result<ImageHandle, ImageError> {
        let source = (image.decoded().width(), image.decoded().height());
        let (width, height) = self.target_size(source, configuration)?;
        if (width, height) == source {
            return Ok(image);
        }
        let pixel_bytes = (width as usize) * (height as usize) * 4;
        let mut rgba: Arc<[u8]> = std::iter::repeat_n(0, pixel_bytes).collect();
        let output = Arc::get_mut(&mut rgba).expect("resize output has one owner");
        resample(image.decoded().pixels(), source, output, (width, height));
        let mut resized = ImageHandle::ready(
            image.source().clone(),
            DecodedImage::from_rgba8(width, height, rgba)?,
        )?;
        resized.set_decoded_scale(image.scale());
        Ok(resized)
    }
}

/// Identifies a resized variant by the resolved source key, effective
/// physical dimensions, policy and upscaling permission.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResizeImageKey<K> {
    provider_key: K,
    options: ImageResizeOptions,
}

impl<K> ResizeImageKey<K> {
    #[must_use]
    pub fn provider_key(&self) -> &K {
        &self.provider_key
    }

    #[must_use]
    pub fn options(&self) -> ImageResizeOptions {
        self.options
    }
}

/// Flutter's resizing provider. It wraps any Incular provider, including
/// ones whose first image is delivered asynchronously.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResizeImage {
    pub image_provider: AnyImageProvider,
    pub options: ImageResizeOptions,
}

impl ResizeImage {
    #[must_use]
    pub fn new(
        provider: impl ImageProvider + 'static,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Self {
        debug_assert!(
            width.is_some() || height.is_some(),
            "ResizeImage needs at least one dimension"
        );
        Self {
            image_provider: AnyImageProvider::new(provider),
            options: ImageResizeOptions {
                width,
                height,
                ..Default::default()
            },
        }
    }

    /// Requested width before density scaling, when supplied.
    #[must_use]
    pub const fn width(&self) -> Option<u32> {
        self.options.width
    }

    /// Requested height before density scaling, when supplied.
    #[must_use]
    pub const fn height(&self) -> Option<u32> {
        self.options.height
    }

    #[must_use]
    pub fn allow_upscaling(mut self, value: bool) -> Self {
        self.options.allow_upscaling = value;
        self
    }

    #[must_use]
    pub fn use_logical_size(mut self, value: bool) -> Self {
        self.options.use_logical_size = value;
        self
    }

    #[must_use]
    pub fn policy(mut self, policy: ResizeImagePolicy) -> Self {
        self.options.policy = policy;
        self
    }

    /// Combines a source provider's resolved key with this request's
    /// physical dimensions. For an AssetImage, pass its resolved key,
    /// not the logical name, so resolution variants stay independent.
    #[must_use]
    pub fn key_for<K>(
        &self,
        provider_key: K,
        configuration: &ImageConfiguration,
    ) -> ResizeImageKey<K> {
        ResizeImageKey {
            provider_key,
            options: self.options.physical(configuration),
        }
    }

    /// Decorates a typed source-key lookup with this resize policy.
    ///
    /// The type-erased provider does not prescribe the source key type:
    /// pass the concrete provider's own lookup, for example
    /// `|complete| asset.obtain_key(configuration, complete)`. Inline
    /// callbacks stay inline and deferred callbacks stay deferred. The
    /// physical settings are captured before starting the lookup.
    pub fn obtain_key_for<K: 'static, E: 'static>(
        &self,
        configuration: &ImageConfiguration,
        obtain_source_key: impl FnOnce(Box<dyn FnOnce(Result<K, E>) + Send>),
        complete: impl FnOnce(Result<ResizeImageKey<K>, E>) + Send + 'static,
    ) {
        let options = self.options.physical(configuration);
        obtain_source_key(Box::new(move |result| {
            complete(result.map(|provider_key| ResizeImageKey { provider_key, options }));
        }));
    }

    /// Returns the original provider when neither cache dimension is set.
    #[must_use]
    pub fn resize_if_needed(
        provider: impl ImageProvider + 'static,
        width: Option<u32>,
        height: Option<u32>,
        use_logical_size: bool,
    ) -> AnyImageProvider {
        if width.is_none() && height.is_none() {
            AnyImageProvider::new(provider)
        } else {
            AnyImageProvider::new(
                Self::new(provider, width, height).use_logical_size(use_logical_size),
            )
        }
    }
}

fn label_resize(stream: ImageStream, options: ImageResizeOptions) -> ImageStream {
    let Some(source) = stream.debug_label() else {
        return stream;
    };
    let dimension = |value: Option<u32>| value.map_or_else(|| "null".into(), |v| v.to_string());
    stream.with_debug_label(format!(
        "{source} - Resized({}×{})",
        dimension(options.width),
        dimension(options.height)
    ))
}

impl ImageProvider for ResizeImage {
    fn resolve_cached(
        &self,
        configuration: &ImageConfiguration,
        resize: Option<ImageResizeOptions>,
    ) -> Option<ImageStream> {
        if resize.is_some() {
            // Nested resizing composes two transforms, rather than reusing
            // a cache item with only one of the effective bounds.
            return None;
        }
        let options = self.options.physical(configuration);
        self.image_provider
            .resolve_cached_with_bounds(configuration, Some(options))
            .map(|stream| label_resize(stream, options))
    }

    fn load(&self) -> Result<ImageHandle, ImageError> {
        self.load_with(&ImageConfiguration::empty())
    }

    fn load_with(&self, configuration: &ImageConfiguration) -> Result<ImageHandle, ImageError> {
        self.image_provider
            .load_resized(configuration, self.options.physical(configuration))
    }

    fn resolve(&self, configuration: &ImageConfiguration) -> ImageStream {
        let options = self.options.physical(configuration);
        label_resize(
            self.image_provider.resolve_resized(configuration, options),
            options,
        )
    }
}

#[derive(Clone, Copy)]
struct Sample {
    low: usize,
    high: usize,
    high_weight: f32,
}

fn sample(target_index: u32, source: u32, target: u32) -> Sample {
    let position = ((f64::from(target_index) + 0.5) * f64::from(source) / f64::from(target) - 0.5)
        .clamp(0.0, f64::from(source - 1));
    let low = position as usize;
    Sample {
        low,
        high: (low + 1).min(source as usize - 1),
        high_weight: (position - low as f64) as f32,
    }
}

fn resample(input: &[u8], source: (u32, u32), output: &mut [u8], target: (u32, u32)) {
    let x: Vec<_> = (0..target.0)
        .map(|i| sample(i, source.0, target.0))
        .collect();
    let stride = source.0 as usize * 4;
    for (row, output) in output.chunks_exact_mut(target.0 as usize * 4).enumerate() {
        let y = sample(row as u32, source.1, target.1);
        let (y0, y1) = (y.low * stride, y.high * stride);
        for (pixel, x) in output.chunks_exact_mut(4).zip(&x) {
            let x0 = x.low * 4;
            let x1 = x.high * 4;
            let inputs = [
                (
                    &input[y0 + x0..][..4],
                    (1.0 - x.high_weight) * (1.0 - y.high_weight),
                ),
                (
                    &input[y0 + x1..][..4],
                    x.high_weight * (1.0 - y.high_weight),
                ),
                (
                    &input[y1 + x0..][..4],
                    (1.0 - x.high_weight) * y.high_weight,
                ),
                (&input[y1 + x1..][..4], x.high_weight * y.high_weight),
            ];
            // Linear filtering in associated-alpha space avoids pulling
            // RGB from an invisible pixel into the edge of painted content.
            let mut sum = [0.0; 4];
            for (src, w) in inputs {
                let alpha = f32::from(src[3]) * w;
                for channel in 0..3 {
                    sum[channel] += f32::from(src[channel]) * alpha;
                }
                sum[3] += alpha;
            }
            pixel[3] = sum[3].round() as u8;
            if pixel[3] != 0 {
                for channel in 0..3 {
                    pixel[channel] = (sum[channel] / sum[3]).round() as u8;
                }
            }
        }
    }
}
