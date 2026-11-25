//! Image-backed painting effect descriptors.

use incular_config::Alignment;
use incular_core::Color;
use incular_image::ImageHandle;
use incular_rendering::{
    BlendMode, ColorFilter, FilterQuality, ImageFilter, ImageFilterTarget, UnsupportedImageFilter,
};
use typed_builder::TypedBuilder;

use crate::tree::ImageFit;
use crate::{Blur, BlurController, ColorFiltered, Image, ImageRepeat, Opacity, Widget};

/// Flutter `RawImage`'s `invertColors` matrix: each channel becomes `255 - c`,
/// alpha unchanged. Written in Flutter's 0..255 bias units.
const INVERT_COLORS: [f32; 20] = [
    -1., 0., 0., 0., 255., 0., -1., 0., 0., 255., 0., 0., -1., 0., 255., 0., 0., 0., 1., 0.,
];

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
    #[builder(default = ImageFit::ScaleDown)]
    pub(super) fit: ImageFit,
    #[builder(default = ImageRepeat::NoRepeat)]
    pub(super) repeat: ImageRepeat,
    /// Flutter `alignment`: where a fitted image sits inside its box.
    #[builder(default = Alignment::CENTER)]
    pub(super) alignment: Alignment,
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
            fit: ImageFit::ScaleDown,
            repeat: ImageRepeat::NoRepeat,
            alignment: Alignment::CENTER,
            filter_quality: None,
            opacity: 1.0,
            invert_colors: false,
            color_blend_mode: None,
            center_slice: None,
            match_text_direction: false,
        }
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
        self.fit = fit;
        self
    }

    #[must_use]
    pub fn repeat(mut self, repeat: ImageRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
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
        match value.image {
            Some(handle) => {
                let mut img = Image::new(handle)
                    .scale(value.scale)
                    .fit(value.fit)
                    .repeat(value.repeat)
                    .alignment(value.alignment)
                    .match_text_direction(value.match_text_direction);
                if let Some(slice) = value.center_slice {
                    img = img.center_slice(slice);
                }
                if let Some(w) = value.width {
                    img = img.width(w);
                }
                if let Some(h) = value.height {
                    img = img.height(h);
                }
                if let Some(quality) = value.filter_quality {
                    img = img.filter_quality(quality);
                }
                let mut image: Widget = img.into();
                // Flutter applies `invertColors` to the decoded pixels before
                // the `color` tint, and `opacity` to the finished image.
                if value.invert_colors {
                    image = ColorFiltered::new(ColorFilter::flutter_matrix(INVERT_COLORS), image)
                        .into();
                }
                // Tint contract: a flat constant-color recolor of the decoded
                // source (opaque white, black, or colored art all flatten to
                // the tint; coverage follows source alpha times tint alpha).
                // It reuses the retained color-matrix layer, so the decoded
                // handle is unchanged and no second image is built.
                let image = match value.color {
                    Some(color) => crate::ColorFiltered::new(
                        value.color_blend_mode.map_or_else(
                            || ColorFilter::tint(color),
                            |mode| ColorFilter::blend(color, mode),
                        ),
                        image,
                    )
                    .into(),
                    None => image,
                };
                if value.opacity < 1.0 {
                    Opacity::new(value.opacity, image).into()
                } else {
                    image
                }
            }
            None => crate::layout::SizedBox::new()
                .width(value.width.unwrap_or(0.0))
                .height(value.height.unwrap_or(0.0))
                .into(),
        }
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
                ImageFilter::Dilate { .. } | ImageFilter::Erode { .. }
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
                            filter_quality: None,
                            child,
                        })
                    }
                    // `new` refuses these and flattens compositions.
                    ImageFilter::Dilate { .. }
                    | ImageFilter::Erode { .. }
                    | ImageFilter::Compose { .. } => child,
                });
        }
        match value.controller {
            Some(c) => Blur::controlled(c, value.child).into(),
            None => Blur::new(value.sigma, value.child).into(),
        }
    }
}
