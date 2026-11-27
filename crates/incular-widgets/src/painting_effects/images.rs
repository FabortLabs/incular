//! Image-backed painting effect descriptors.

use incular_config::Alignment;
use incular_core::Color;
use incular_image::{ImageHandle, ImageInfo};
use incular_rendering::{
    BlendMode, ColorFilter, FilterQuality, ImageFilter, ImageFilterTarget, UnsupportedImageFilter,
};
use typed_builder::TypedBuilder;

use crate::tree::{ImageFit, ImageOptions};
use crate::{AlignmentGeometry, Blur, BlurController, ColorFiltered, Image, ImageRepeat, Widget};

/// A source filter and group opacity that are ready for the retained image.
pub(crate) struct RasterPaintSources {
    pub(crate) blend: Option<BlendMode>,
    pub(crate) filter: Option<ColorFilter>,
    pub(crate) opacity: Option<f32>,
}

impl crate::tree::RasterSourceEffects {
    pub(crate) fn raster_paint_sources(self, mode: BlendMode) -> RasterPaintSources {
        let blend = (mode != BlendMode::SrcOver).then_some(mode);
        let alpha = if self.opacity.is_nan() {
            1.0
        } else {
            self.opacity
        };
        let preserve_coverage = matches!(
            mode,
            BlendMode::Clear
                | BlendMode::Src
                | BlendMode::SrcIn
                | BlendMode::DstIn
                | BlendMode::SrcOut
                | BlendMode::DstAtop
                | BlendMode::Modulate
        ) && incular_rendering::opacity_is_transparent(alpha);
        if preserve_coverage && alpha == 0.0 {
            // A zero-alpha opacity group is culled, whereas Flutter draws
            // transparent source pixels when the mode erases the destination.
            return RasterPaintSources {
                blend,
                filter: Some(ColorFilter::opacity(0.0)),
                opacity: None,
            };
        }
        let mut filter = self.color.map(|color| {
            ColorFilter::blend(color, self.color_blend_mode.unwrap_or(BlendMode::SrcIn))
        });
        // Inversion follows tint, without building a second isolation.
        if self.invert_colors {
            filter = Some(
                filter
                    .unwrap_or_else(ColorFilter::identity)
                    .then(ColorFilter::invert(1.0)),
            );
        }
        if preserve_coverage {
            filter = Some(
                filter
                    .unwrap_or_else(ColorFilter::identity)
                    .then(ColorFilter::opacity(alpha)),
            );
        }
        RasterPaintSources {
            blend,
            filter,
            opacity: (alpha < 1.0 && !preserve_coverage).then_some(alpha),
        }
    }
}

/// Renders a raw raster image buffer directly without asset caching.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct RawImage {
    #[builder(default, setter(strip_option))]
    image: Option<ImageHandle>,
    #[builder(
        default,
        setter(transform = |width: f32| Some(width.max(0.0)))
    )]
    pub(super) width: Option<f32>,
    #[builder(
        default,
        setter(transform = |height: f32| Some(height.max(0.0)))
    )]
    pub(super) height: Option<f32>,
    #[builder(
        default = 1.0,
        setter(transform = |scale: f32| scale.max(0.001))
    )]
    pub(super) scale: f32,
    #[builder(default, setter(strip_option))]
    color: Option<Color>,
    /// When omitted, paints with scale-down, or fill for a nine-slice.
    #[builder(default, setter(strip_option))]
    pub(super) fit: Option<ImageFit>,
    #[builder(default = ImageRepeat::NoRepeat)]
    pub(super) repeat: ImageRepeat,
    /// Flutter `alignment`: where a fitted image sits inside its box.
    #[builder(default = AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    pub(super) alignment: AlignmentGeometry,
    /// Flutter `filterQuality`; `None` keeps the default sampling.
    #[builder(default, setter(strip_option))]
    pub(super) filter_quality: Option<FilterQuality>,
    /// Flutter `opacity`, clamped to `0..=1`.
    #[builder(
        default = 1.0,
        setter(transform = |opacity: f32| opacity.clamp(0.0, 1.0))
    )]
    pub(super) opacity: f32,
    /// Flutter `invertColors`.
    #[builder(default = false)]
    pub(super) invert_colors: bool,
    /// Flutter `colorBlendMode`: how `color` combines with the image
    /// (`ColorFilter.mode(color, mode)`); `None` is Flutter's `srcIn`.
    #[builder(default, setter(strip_option))]
    pub(super) color_blend_mode: Option<BlendMode>,
    /// Composites the finished image against the destination (Flutter
    /// `blendMode`). Independent of the optional tint's `colorBlendMode`.
    #[builder(default = BlendMode::SrcOver)]
    blend_mode: BlendMode,
    /// Flutter `centerSlice`, in logical image pixels.
    #[builder(default, setter(strip_option))]
    pub(super) center_slice: Option<incular_core::Rect>,
    /// Flutter `matchTextDirection`.
    #[builder(default = false)]
    pub(super) match_text_direction: bool,
}

impl Default for RawImage {
    fn default() -> Self {
        Self::new()
    }
}

impl RawImage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            image: None,
            width: None,
            height: None,
            scale: 1.0,
            color: None,
            fit: None,
            repeat: ImageRepeat::NoRepeat,
            alignment: Alignment::CENTER.into(),
            filter_quality: None,
            opacity: 1.0,
            invert_colors: false,
            color_blend_mode: None,
            blend_mode: BlendMode::SrcOver,
            center_slice: None,
            match_text_direction: false,
        }
    }

    /// Uses a decoded image and its intended logical scale. The diagnostic
    /// label does not affect layout, painting, or retained image identity.
    #[must_use]
    pub fn from_image_info(info: ImageInfo) -> Self {
        let scale = info.scale().min(f64::from(f32::MAX)) as f32;
        Self::new().image(info.into_image()).scale(scale)
    }

    #[must_use]
    pub fn image(mut self, image: ImageHandle) -> Self {
        self.image = Some(image);
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height.max(0.0));
        self
    }

    #[must_use]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale.max(0.001);
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = Some(fit);
        self
    }

    #[must_use]
    pub fn repeat(mut self, repeat: ImageRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    #[must_use]
    pub fn filter_quality(mut self, quality: FilterQuality) -> Self {
        self.filter_quality = Some(quality);
        self
    }

    /// Flutter `opacity`, clamped to `0..=1`.
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn invert_colors(mut self, invert: bool) -> Self {
        self.invert_colors = invert;
        self
    }

    /// Flutter `colorBlendMode`: blends `color` with the image through
    /// [`ColorFilter::blend`] instead of the default `srcIn` tint.
    #[must_use]
    pub fn color_blend_mode(mut self, mode: BlendMode) -> Self {
        self.color_blend_mode = Some(mode);
        self
    }

    /// Composites the image against what's already painted. The default
    /// is source-over; this does not change how an optional tint is applied.
    #[must_use]
    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Flutter `centerSlice`: paints the image as a nine-slice.
    #[must_use]
    pub fn center_slice(mut self, slice: incular_core::Rect) -> Self {
        self.center_slice = Some(slice);
        self
    }

    /// Flutter `matchTextDirection`: mirrors under a right-to-left
    /// ambient direction.
    #[must_use]
    pub fn match_text_direction(mut self, matches: bool) -> Self {
        self.match_text_direction = matches;
        self
    }
}

impl From<RawImage> for Widget {
    fn from(value: RawImage) -> Self {
        Widget::image(
            value.image,
            ImageOptions {
                width: value.width,
                height: value.height,
                scale: if value.scale.is_finite() {
                    value.scale
                } else {
                    1.0
                },
                fit: value.fit,
                repeat: value.repeat,
                alignment: value.alignment,
                sampling: value.filter_quality.map_or(
                    incular_rendering::ImageSampling::Linear,
                    FilterQuality::sampling,
                ),
                center_slice: value.center_slice,
                match_text_direction: value.match_text_direction,
                destination_blend_mode: value.blend_mode,
                color: value.color,
                color_blend_mode: value.color_blend_mode,
                invert_colors: value.invert_colors,
                opacity: value.opacity,
            },
        )
    }
}

/// An icon that comes from an [`ImageHandle`].
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ImageIcon {
    image: ImageHandle,
    #[builder(
        default,
        setter(transform = |size: f32| Some(size.max(0.0)))
    )]
    pub(super) size: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(super) color: Option<Color>,
}

impl ImageIcon {
    #[must_use]
    pub fn new(image: ImageHandle) -> Self {
        Self {
            image,
            size: None,
            color: None,
        }
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size.max(0.0));
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl From<ImageIcon> for Widget {
    fn from(value: ImageIcon) -> Self {
        let size = value.size.unwrap_or(24.0);
        let image: Widget = Image::new(value.image)
            .width(size)
            .height(size)
            .fit(ImageFit::ScaleDown)
            .into();
        // Icon recolor contract: a flat constant-color tint matching the
        // `srcIn` blend behavior, sharing RawImage's color-matrix layer;
        // the source handle is untouched.
        match value.color {
            Some(color) => {
                crate::ColorFiltered::new(incular_rendering::ColorFilter::tint(color), image).into()
            }
            None => image,
        }
    }
}

/// An image filter applied directly to its child subtree.
///
/// Flutter's `ImageFiltered`. [`Self::new`] accepts a general
/// [`ImageFilter`]: blur, matrix and color filters, and compositions of
/// them, each run as its own retained stage in application order. A matrix
/// filter transforms the painted child about its top-left corner without
/// moving hit testing, as Flutter's `ImageFilterLayer` does. Dilate and erode
/// have no subtree stage yet and are refused with
/// [`UnsupportedImageFilter`] instead of painting unfiltered.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ImageFiltered {
    sigma: f32,
    #[builder(default, setter(skip))]
    pub(super) controller: Option<BlurController>,
    /// A general filter from [`Self::new`]; replaces `sigma` when set.
    #[builder(default, setter(skip))]
    filter: Option<ImageFilter>,
    /// Flutter `ImageFiltered.enabled`: a disabled filter passes its child
    /// through unchanged, keeping the configured filter.
    #[builder(default = true)]
    enabled: bool,
    #[builder(setter(into))]
    child: Widget,
}

impl ImageFiltered {
    /// Flutter `ImageFiltered(imageFilter: filter)`. Refuses a filter with a
    /// dilate or erode stage.
    pub fn new(
        filter: ImageFilter,
        child: impl Into<Widget>,
    ) -> Result<Self, UnsupportedImageFilter> {
        if let Some(stage) = filter.stages().into_iter().find(|stage| {
            matches!(
                stage,
                ImageFilter::Dilate { .. }
                    | ImageFilter::Erode { .. }
                    | ImageFilter::SampledBlur(_)
            )
        }) {
            return Err(UnsupportedImageFilter {
                filter: stage,
                target: ImageFilterTarget::Subtree,
            });
        }
        Ok(Self {
            sigma: 0.0,
            controller: None,
            filter: Some(filter),
            enabled: true,
            child: child.into(),
        })
    }

    #[must_use]
    pub fn blur(sigma: f32, child: impl Into<Widget>) -> Self {
        Self {
            sigma,
            controller: None,
            filter: None,
            enabled: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn controlled(controller: BlurController, child: impl Into<Widget>) -> Self {
        Self {
            sigma: controller.sigma(),
            controller: Some(controller),
            filter: None,
            enabled: true,
            child: child.into(),
        }
    }

    /// Flutter `ImageFiltered.enabled`; disabling keeps the filter configured.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl From<ImageFiltered> for Widget {
    fn from(value: ImageFiltered) -> Self {
        if !value.enabled {
            return value.child;
        }
        if let Some(filter) = value.filter {
            return filter
                .stages()
                .into_iter()
                .fold(value.child, |child, stage| match stage {
                    ImageFilter::Blur(blur) => {
                        Widget::asymmetric_blur(blur.sigma_x, blur.sigma_y, child)
                    }
                    ImageFilter::Color(filter) => ColorFiltered::new(filter, child).into(),
                    ImageFilter::Matrix(transform) => {
                        Widget::from_kind(crate::tree::WidgetKind::Transform {
                            transform,
                            origin: Some(incular_core::Offset::ZERO),
                            alignment: None,
                            fraction: None,
                            transform_hit_tests: false,
                            slot: None,
                            // Flutter's ImageFilter.matrix defaults to
                            // Medium. Even affine scaling samples an
                            // isolated child image, not transformed draws.
                            filter_quality: Some(incular_rendering::FilterQuality::Medium),
                            child,
                        })
                    }
                    // `new` refuses these and flattens compositions.
                    ImageFilter::Dilate { .. }
                    | ImageFilter::Erode { .. }
                    | ImageFilter::SampledBlur(_)
                    | ImageFilter::Compose { .. } => child,
                });
        }
        match value.controller {
            Some(c) => Blur::controlled(c, value.child).into(),
            None => Blur::new(value.sigma, value.child).into(),
        }
    }
}
