//! Layout-independent image-filter descriptions and their local sampling bounds.

use std::{
    fmt,
    hash::{Hash, Hasher},
};

use incular_core::Rect;

use crate::{ColorFilterStage, GaussianBlur, ImageFilter, TileMode};

/// The local bounds of the object applying an image filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageFilterContext {
    pub bounds: Rect,
}

/// A Gaussian blur with an explicit edge policy and optional local sampling
/// bounds. Bounds restrict the source; they do not clip the filtered output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledBlur {
    pub blur: GaussianBlur,
    pub tile_mode: TileMode,
    pub bounds: Option<Rect>,
}

/// A filter description that can resolve against the painted object's bounds.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageFilterConfig {
    Direct(ImageFilter),
    Blur {
        sigma_x: f32,
        sigma_y: f32,
        tile_mode: TileMode,
        bounded: bool,
    },
    Compose {
        outer: Box<Self>,
        inner: Box<Self>,
    },
}

impl ImageFilterConfig {
    #[must_use]
    pub const fn new(filter: ImageFilter) -> Self {
        Self::Direct(filter)
    }

    #[must_use]
    pub const fn blur(sigma_x: f32, sigma_y: f32) -> Self {
        Self::blur_with_sampling(sigma_x, sigma_y, TileMode::Clamp, false)
    }

    #[must_use]
    pub const fn blur_with_sampling(
        sigma_x: f32,
        sigma_y: f32,
        tile_mode: TileMode,
        bounded: bool,
    ) -> Self {
        Self::Blur {
            sigma_x,
            sigma_y,
            tile_mode,
            bounded,
        }
    }

    /// Applies `inner` to the source before applying `outer`.
    #[must_use]
    pub fn compose(outer: Self, inner: Self) -> Self {
        Self::Compose {
            outer: Box::new(outer),
            inner: Box::new(inner),
        }
    }

    /// The original filter, if this configuration wraps a direct filter.
    #[must_use]
    pub const fn filter(&self) -> Option<&ImageFilter> {
        match self {
            Self::Direct(filter) => Some(filter),
            _ => None,
        }
    }

    #[must_use]
    pub fn resolve(&self, context: ImageFilterContext) -> ImageFilter {
        match self {
            Self::Direct(filter) => filter.clone(),
            Self::Blur {
                sigma_x,
                sigma_y,
                tile_mode,
                bounded,
            } => ImageFilter::SampledBlur(SampledBlur {
                blur: GaussianBlur::new(*sigma_x, *sigma_y),
                tile_mode: *tile_mode,
                bounds: bounded.then_some(context.bounds),
            }),
            Self::Compose { outer, inner } => {
                ImageFilter::compose(outer.resolve(context), inner.resolve(context))
            }
        }
    }

    #[must_use]
    pub fn debug_short_description(&self) -> String {
        match self {
            Self::Direct(filter) => filter.debug_short_description(),
            Self::Blur {
                sigma_x,
                sigma_y,
                tile_mode,
                bounded,
            } => {
                let mode = tile_name(*tile_mode);
                let range = if *bounded { "bounded" } else { "unbounded" };
                format!("blur({sigma_x:?}, {sigma_y:?}, {mode}, {range})")
            }
            Self::Compose { outer, inner } => format!(
                "{} -> {}",
                inner.debug_short_description(),
                outer.debug_short_description(),
            ),
        }
    }
}

impl fmt::Display for ImageFilterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let short = self.debug_short_description();
        match self {
            Self::Direct(_) => write!(f, "ImageFilterConfig({short})"),
            Self::Blur { .. } => write!(f, "ImageFilterConfig.{short}"),
            Self::Compose { .. } => {
                write!(f, "ImageFilterConfig.compose(source -> {short} -> result)")
            }
        }
    }
}

fn tile_name(mode: TileMode) -> &'static str {
    match mode {
        TileMode::Clamp => "clamp",
        TileMode::Repeated => "repeated",
        TileMode::Mirror => "mirror",
        TileMode::Decal => "decal",
    }
}

fn number(value: f32, state: &mut impl Hasher) {
    // PartialEq equates positive and negative zero.
    if value == 0. { 0 } else { value.to_bits() }.hash(state);
}

impl Hash for ImageFilterConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Direct(filter) => filter.hash(state),
            Self::Blur {
                sigma_x,
                sigma_y,
                tile_mode,
                bounded,
            } => {
                number(*sigma_x, state);
                number(*sigma_y, state);
                tile_mode.hash(state);
                bounded.hash(state);
            }
            Self::Compose { outer, inner } => {
                outer.hash(state);
                inner.hash(state);
            }
        }
    }
}

impl ImageFilter {
    #[must_use]
    pub fn debug_short_description(&self) -> String {
        match self {
            Self::Blur(blur) => {
                format!("blur({:?}, {:?}, unspecified)", blur.sigma_x, blur.sigma_y)
            }
            Self::SampledBlur(sampled) => {
                let SampledBlur {
                    blur,
                    tile_mode,
                    bounds,
                } = sampled;
                let mode = tile_name(*tile_mode);
                match bounds {
                    None => format!("blur({:?}, {:?}, {mode})", blur.sigma_x, blur.sigma_y),
                    Some(bounds) => {
                        let left = bounds.origin.x;
                        let top = bounds.origin.y;
                        let right = left + bounds.size.width;
                        let bottom = top + bounds.size.height;
                        format!(
                            "blur({:?}, {:?}, {mode}, bounds: Rect.fromLTRB({left:?}, {top:?}, {right:?}, {bottom:?}))",
                            blur.sigma_x, blur.sigma_y
                        )
                    }
                }
            }
            Self::Compose { outer, inner } => format!(
                "{} -> {}",
                inner.debug_short_description(),
                outer.debug_short_description()
            ),
            Self::Matrix(transform) => format!("matrix({:?})", transform.coefficients()),
            Self::SampledMatrix { transform, quality } => {
                format!("matrix({:?}, {quality:?})", transform.coefficients())
            }
            Self::Dilate { radius_x, radius_y } => format!("dilate({radius_x:?}, {radius_y:?})"),
            Self::Erode { radius_x, radius_y } => format!("erode({radius_x:?}, {radius_y:?})"),
            Self::Color(filter) => format!("color({filter:?})"),
        }
    }
}

impl fmt::Display for ImageFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImageFilter.{}", self.debug_short_description())
    }
}

impl Hash for ImageFilter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Blur(blur) => {
                number(blur.sigma_x, state);
                number(blur.sigma_y, state);
            }
            Self::SampledBlur(sampled) => {
                number(sampled.blur.sigma_x, state);
                number(sampled.blur.sigma_y, state);
                sampled.tile_mode.hash(state);
                sampled.bounds.is_some().hash(state);
                if let Some(bounds) = sampled.bounds {
                    for n in [
                        bounds.origin.x,
                        bounds.origin.y,
                        bounds.size.width,
                        bounds.size.height,
                    ] {
                        number(n, state);
                    }
                }
            }
            Self::Matrix(transform) | Self::SampledMatrix { transform, .. } => {
                if let Self::SampledMatrix { quality, .. } = self {
                    quality.hash(state);
                }
                for coefficient in transform.coefficients() {
                    if coefficient == 0. {
                        0
                    } else {
                        coefficient.to_bits()
                    }
                    .hash(state);
                }
            }
            Self::Dilate { radius_x, radius_y } | Self::Erode { radius_x, radius_y } => {
                number(*radius_x, state);
                number(*radius_y, state);
            }
            Self::Color(filter) => {
                for n in filter.to_matrix() {
                    number(n, state);
                }
                let stage = filter.stage();
                std::mem::discriminant(&stage).hash(state);
                if let ColorFilterStage::Blend { color, mode } = stage {
                    color.hash(state);
                    mode.hash(state);
                }
            }
            Self::Compose { outer, inner } => {
                outer.hash(state);
                inner.hash(state);
            }
        }
    }
}
