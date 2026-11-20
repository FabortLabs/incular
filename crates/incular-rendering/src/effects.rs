use crate::geometry::union_rect;
use incular_core::finite_non_negative;
use incular_core::{Color, Offset, Rect, Size, finite_or_zero};

/// Renderer-neutral separable Gaussian blur description. Sigma is expressed
/// in logical pixels; a backend converts it to physical pixels at render
/// time. Negative and non-finite values are normalized to zero at this
/// boundary so every backend observes the same effect semantics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaussianBlur {
    pub sigma_x: f32,
    pub sigma_y: f32,
}
impl GaussianBlur {
    #[must_use]
    pub fn new(sigma_x: f32, sigma_y: f32) -> Self {
        Self {
            sigma_x: normalize_sigma(sigma_x),
            sigma_y: normalize_sigma(sigma_y),
        }
    }
    #[must_use]
    pub fn uniform(sigma: f32) -> Self {
        Self::new(sigma, sigma)
    }
}

/// Renderer-neutral arbitrary-subtree shadow description. The source is
/// isolated first, then its alpha is blurred and colorized by the compositor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadowEffect {
    pub offset: Offset,
    pub sigma_x: f32,
    pub sigma_y: f32,
    pub color: Color,
}

/// A renderer-independent 4x5 color matrix. Matrix inputs and outputs are
/// straight (unpremultiplied) RGBA in the normalized 0..=1 range. The GPU
/// backend converts its retained premultiplied texture to straight color,
/// applies this matrix, clamps the result, and premultiplies it again.
///
/// A filter may also carry one [`ColorFilterStage`] that runs before the
/// matrix: Flutter's `ColorFilter.linearToSrgbGamma`,
/// `ColorFilter.srgbToLinearGamma`, and the `ColorFilter.mode` blends no
/// matrix can express. Matrix-only filters keep their exact meaning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorFilter {
    matrix: [f32; 20],
    stage: ColorFilterStage,
}

/// A non-affine per-pixel stage of a [`ColorFilter`], applied before the
/// filter's matrix.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorFilterStage {
    /// No stage: the filter is its matrix alone.
    #[default]
    None,
    /// Flutter `ColorFilter.linearToSrgbGamma`: the sRGB transfer function
    /// (encode) applied to each straight RGB channel.
    LinearToSrgbGamma,
    /// Flutter `ColorFilter.srgbToLinearGamma`: the inverse sRGB transfer
    /// function (decode) applied to each straight RGB channel.
    SrgbToLinearGamma,
    /// Flutter `ColorFilter.mode(color, mode)`: `color` is the blend source
    /// and each premultiplied pixel the destination, with the equations of
    /// [`blend_premultiplied`].
    Blend { color: Color, mode: BlendMode },
}

impl ColorFilterStage {
    /// Applies the stage to one straight normalized RGBA value.
    #[must_use]
    pub fn apply(self, rgba: [f32; 4]) -> [f32; 4] {
        match self {
            Self::None => rgba,
            Self::LinearToSrgbGamma => [
                linear_to_srgb(rgba[0]),
                linear_to_srgb(rgba[1]),
                linear_to_srgb(rgba[2]),
                rgba[3],
            ],
            Self::SrgbToLinearGamma => [
                srgb_to_linear(rgba[0]),
                srgb_to_linear(rgba[1]),
                srgb_to_linear(rgba[2]),
                rgba[3],
            ],
            Self::Blend { color, mode } => {
                let alpha = rgba[3];
                let destination = [rgba[0] * alpha, rgba[1] * alpha, rgba[2] * alpha, alpha];
                let source = crate::premultiplied_working_rgba(color);
                let [r, g, b, a] = blend_premultiplied(mode, source, destination);
                if a <= f32::EPSILON {
                    [0., 0., 0., 0.]
                } else {
                    [r / a, g / a, b / a, a]
                }
            }
        }
    }

    /// The shader encoding shared with GPU backends: a stage code (0 none,
    /// 1 linear-to-sRGB, 2 sRGB-to-linear, 3 blend), the blend mode's
    /// [`BlendMode::code`], and the premultiplied blend source color.
    #[must_use]
    pub fn encoded(self) -> ([u32; 2], [f32; 4]) {
        match self {
            Self::None => ([0, 0], [0.; 4]),
            Self::LinearToSrgbGamma => ([1, 0], [0.; 4]),
            Self::SrgbToLinearGamma => ([2, 0], [0.; 4]),
            Self::Blend { color, mode } => {
                ([3, mode.code()], crate::premultiplied_working_rgba(color))
            }
        }
    }
}

/// The sRGB encoding transfer function (IEC 61966-2-1), as Skia's
/// `SkColorFilters::LinearToSRGBGamma` applies it.
#[must_use]
pub fn linear_to_srgb(value: f32) -> f32 {
    let value = finite_unit(value);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    }
}

/// The sRGB decoding transfer function, the inverse of [`linear_to_srgb`].
#[must_use]
pub fn srgb_to_linear(value: f32) -> f32 {
    let value = finite_unit(value);
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
/// Compatibility spelling for callers that model a 4×5 filter as a color
/// matrix.  The alias keeps one canonical representation and one set of
/// straight/premultiplied semantics.
pub type ColorMatrix = ColorFilter;
impl ColorFilter {
    #[must_use]
    pub fn new(matrix: [f32; 20]) -> Self {
        Self::matrix(matrix)
    }
    #[must_use]
    pub fn matrix(matrix: [f32; 20]) -> Self {
        Self {
            matrix: matrix.map(finite_or_zero),
            stage: ColorFilterStage::None,
        }
    }
    /// Builds a filter from Flutter's `ColorFilter.matrix` convention: the
    /// same row-major 4x5 layout, but the translation column (indices 4, 9,
    /// 14 and 19) is unnormalized, in the 0..=255 range (Flutter's
    /// `painting.dart` documents this for `ColorFilter.matrix`). The biases
    /// are divided by 255 into the normalized representation used by
    /// [`Self::matrix`], so Flutter's invert example maps white to black.
    #[must_use]
    pub fn flutter_matrix(mut matrix: [f32; 20]) -> Self {
        for index in [4, 9, 14, 19] {
            matrix[index] = finite_or_zero(matrix[index]) / 255.;
        }
        Self::matrix(matrix)
    }
    /// Flutter's `ColorFilter.mode(color, mode)`: `color` is the blend
    /// source and each filtered pixel the destination. Returns the exact
    /// matrix form for the modes whose straight-alpha result is affine in
    /// the destination: `src`, `srcIn`, `dstIn`, `srcOut`, `dstOut`,
    /// `srcATop` and `modulate`. Every other mode (including
    /// `srcOver`, `dstATop` and the separable and non-separable artistic
    /// modes) divides by or multiplies with the destination alpha after
    /// premultiplication, which a 4x5 straight-RGBA matrix cannot
    /// express; those return `None` rather than an approximation.
    #[must_use]
    pub fn mode(color: Color, mode: BlendMode) -> Option<Self> {
        let [r, g, b, a] = [color.red, color.green, color.blue, color.alpha]
            .map(|component| f32::from(component) / 255.);
        let rows = |rgb: [[f32; 5]; 3], alpha: [f32; 5]| {
            let mut matrix = [0.; 20];
            for (row, values) in rgb.iter().chain(std::iter::once(&alpha)).enumerate() {
                matrix[row * 5..row * 5 + 5].copy_from_slice(values);
            }
            Self::matrix(matrix)
        };
        // Straight RGB of the source color (the destination is replaced).
        let source_rgb = [
            [0., 0., 0., 0., r],
            [0., 0., 0., 0., g],
            [0., 0., 0., 0., b],
        ];
        // Straight RGB of the destination (kept).
        let destination_rgb = [
            [1., 0., 0., 0., 0.],
            [0., 1., 0., 0., 0.],
            [0., 0., 1., 0., 0.],
        ];
        Some(match mode {
            BlendMode::Src => rows(source_rgb, [0., 0., 0., 0., a]),
            BlendMode::SrcIn => rows(source_rgb, [0., 0., 0., a, 0.]),
            BlendMode::DstIn => rows(destination_rgb, [0., 0., 0., a, 0.]),
            BlendMode::SrcOut => rows(source_rgb, [0., 0., 0., -a, a]),
            BlendMode::DstOut => rows(destination_rgb, [0., 0., 0., 1. - a, 0.]),
            // Premultiplied `Cs·as·ad + Cd·ad·(1 - as)` over alpha `ad`.
            BlendMode::SrcAtop => rows(
                [
                    [1. - a, 0., 0., 0., r * a],
                    [0., 1. - a, 0., 0., g * a],
                    [0., 0., 1. - a, 0., b * a],
                ],
                [0., 0., 0., 1., 0.],
            ),
            BlendMode::Modulate => rows(
                [
                    [r, 0., 0., 0., 0.],
                    [0., g, 0., 0., 0.],
                    [0., 0., b, 0., 0.],
                ],
                [0., 0., 0., a, 0.],
            ),
            _ => return None,
        })
    }
    /// Flutter's `ColorFilter.mode(color, mode)` for every mode: the exact
    /// matrix of [`Self::mode`] where one exists, otherwise a
    /// [`ColorFilterStage::Blend`] stage that backends execute per pixel.
    #[must_use]
    pub fn blend(color: Color, mode: BlendMode) -> Self {
        Self::mode(color, mode).unwrap_or(Self {
            stage: ColorFilterStage::Blend { color, mode },
            ..Self::identity()
        })
    }
    /// Flutter's `ColorFilter.linearToSrgbGamma()`.
    #[must_use]
    pub fn linear_to_srgb_gamma() -> Self {
        Self {
            stage: ColorFilterStage::LinearToSrgbGamma,
            ..Self::identity()
        }
    }
    /// Flutter's `ColorFilter.srgbToLinearGamma()`.
    #[must_use]
    pub fn srgb_to_linear_gamma() -> Self {
        Self {
            stage: ColorFilterStage::SrgbToLinearGamma,
            ..Self::identity()
        }
    }
    /// The stage applied before the matrix.
    #[must_use]
    pub const fn stage(self) -> ColorFilterStage {
        self.stage
    }
    #[must_use]
    pub fn identity() -> Self {
        Self::matrix([
            1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0.,
        ])
    }
    #[must_use]
    pub fn to_matrix(self) -> [f32; 20] {
        self.matrix
    }
    #[must_use]
    pub fn is_identity(self) -> bool {
        self == Self::identity()
    }
    /// Applies the stage, then the matrix, to straight normalized RGBA and
    /// clamps every output component. Zero-alpha input has zero straight RGB
    /// to avoid undefined premultiplied-to-straight division at transparent
    /// texels.
    #[must_use]
    pub fn apply(self, rgba: [f32; 4]) -> [f32; 4] {
        let alpha = finite_unit(rgba[3]);
        let rgba = self.stage.apply(if alpha > f32::EPSILON {
            [
                finite_unit(rgba[0]),
                finite_unit(rgba[1]),
                finite_unit(rgba[2]),
                alpha,
            ]
        } else {
            [0., 0., 0., 0.]
        });
        let alpha = finite_unit(rgba[3]);
        let input = [
            if alpha > f32::EPSILON {
                finite_unit(rgba[0])
            } else {
                0.
            },
            if alpha > f32::EPSILON {
                finite_unit(rgba[1])
            } else {
                0.
            },
            if alpha > f32::EPSILON {
                finite_unit(rgba[2])
            } else {
                0.
            },
            alpha,
        ];
        let mut out = [0.; 4];
        for (row, value) in out.iter_mut().enumerate() {
            let base = row * 5;
            *value = self.matrix[base] * input[0]
                + self.matrix[base + 1] * input[1]
                + self.matrix[base + 2] * input[2]
                + self.matrix[base + 3] * input[3]
                + self.matrix[base + 4];
        }
        let out = out.map(finite_unit);
        if out[3] <= f32::EPSILON {
            [0., 0., 0., 0.]
        } else {
            out
        }
    }
    /// `self` followed by `next` as one filter, when one filter expresses
    /// it: `next` has no stage (its matrix multiplies `self`'s, and `self`'s
    /// stage stays first), or `self` is the identity. `None` when `next`'s
    /// stage would have to run between two matrices.
    #[must_use]
    pub fn try_then(self, next: Self) -> Option<Self> {
        if next.stage == ColorFilterStage::None {
            Some(Self {
                stage: self.stage,
                ..self.then_matrix(next)
            })
        } else if self.is_identity() {
            Some(next)
        } else {
            None
        }
    }
    /// Returns a matrix for `self` followed by `next` (`next * self`). The
    /// affine bias column is composed as part of the same multiplication.
    ///
    /// # Panics
    ///
    /// When [`Self::try_then`] returns `None`: `next` carries a
    /// [`ColorFilterStage`] and `self` is not the identity. Matrix-only
    /// filters always compose.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        self.try_then(next)
            .expect("a color filter stage cannot follow another filter's matrix")
    }
    fn then_matrix(self, next: Self) -> Self {
        let a = self.matrix;
        let b = next.matrix;
        let mut out = [0.; 20];
        for row in 0..4 {
            for column in 0..4 {
                out[row * 5 + column] = (0..4)
                    .map(|index| b[row * 5 + index] * a[index * 5 + column])
                    .sum();
            }
            out[row * 5 + 4] = b[row * 5 + 4]
                + (0..4)
                    .map(|index| b[row * 5 + index] * a[index * 5 + 4])
                    .sum::<f32>();
        }
        Self::matrix(out)
    }
    /// Alias for [`Self::then`] that reads naturally at call sites building a
    /// filter pipeline.
    #[must_use]
    pub fn compose(self, next: Self) -> Self {
        self.then(next)
    }
    #[must_use]
    pub fn grayscale(amount: f32) -> Self {
        let amount = finite_unit(amount);
        let inv = 1. - amount;
        let r = 0.2126 * amount;
        let g = 0.7152 * amount;
        let b = 0.0722 * amount;
        Self::matrix([
            r + inv,
            g,
            b,
            0.,
            0.,
            r,
            g + inv,
            b,
            0.,
            0.,
            r,
            g,
            b + inv,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
        ])
    }
    #[must_use]
    pub fn sepia(amount: f32) -> Self {
        let amount = finite_unit(amount);
        let inv = 1. - amount;
        Self::matrix([
            0.393 * amount + inv,
            0.769 * amount,
            0.189 * amount,
            0.,
            0.,
            0.349 * amount,
            0.686 * amount + inv,
            0.168 * amount,
            0.,
            0.,
            0.272 * amount,
            0.534 * amount,
            0.131 * amount + inv,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
        ])
    }
    /// Brightness uses one as identity and zero as black.
    #[must_use]
    pub fn brightness(amount: f32) -> Self {
        let amount = finite_non_negative(amount);
        Self::matrix([
            amount, 0., 0., 0., 0., 0., amount, 0., 0., 0., 0., 0., amount, 0., 0., 0., 0., 0., 1.,
            0.,
        ])
    }
    /// Contrast uses one as identity; the bias keeps middle gray fixed.
    #[must_use]
    pub fn contrast(amount: f32) -> Self {
        let amount = finite_non_negative(amount);
        let bias = 0.5 * (1. - amount);
        Self::matrix([
            amount, 0., 0., 0., bias, 0., amount, 0., 0., bias, 0., 0., amount, 0., bias, 0., 0.,
            0., 1., 0.,
        ])
    }
    /// Saturation uses one as identity and zero as grayscale.
    #[must_use]
    pub fn saturate(amount: f32) -> Self {
        let amount = finite_non_negative(amount);
        let inv = 1. - amount;
        let r = 0.2126 * inv;
        let g = 0.7152 * inv;
        let b = 0.0722 * inv;
        Self::matrix([
            r + amount,
            g,
            b,
            0.,
            0.,
            r,
            g + amount,
            b,
            0.,
            0.,
            r,
            g,
            b + amount,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
        ])
    }
    /// Invert uses one as the fully inverted result and zero as identity.
    #[must_use]
    pub fn invert(amount: f32) -> Self {
        let amount = finite_unit(amount);
        Self::matrix([
            1. - 2. * amount,
            0.,
            0.,
            0.,
            amount,
            0.,
            1. - 2. * amount,
            0.,
            0.,
            amount,
            0.,
            0.,
            1. - 2. * amount,
            0.,
            amount,
            0.,
            0.,
            0.,
            1.,
            0.,
        ])
    }
    /// Component-wise modulation: every straight channel is multiplied by
    /// the color's straight working-space components, including alpha. This
    /// recolors only white masks; it is not a flat tint (see [`Self::tint`]).
    #[must_use]
    pub fn modulate(color: Color) -> Self {
        let [red, green, blue, alpha] = crate::working_rgba(color);
        Self::matrix([
            red, 0., 0., 0., 0., 0., green, 0., 0., 0., 0., 0., blue, 0., 0., 0., 0., 0., alpha, 0.,
        ])
    }
    /// Constant-color tint in straight RGBA: `R' = tint.r`, `G' = tint.g`,
    /// `B' = tint.b`, `A' = source.a * tint.a`.
    ///
    /// A 4x5 matrix expresses this through the RGB bias column and the
    /// alpha row's source-alpha coefficient; the shader and [`Self::apply`]
    /// share that bias convention, so both agree. Transparent sources stay
    /// transparent: zero source alpha yields zero straight RGB, and the
    /// biased RGB is multiplied back by the zero alpha at output.
    #[must_use]
    pub fn tint(color: Color) -> Self {
        let [red, green, blue, alpha] = crate::working_rgba(color);
        Self::matrix([
            0., 0., 0., 0., red, 0., 0., 0., 0., green, 0., 0., 0., 0., blue, 0., 0., 0., alpha, 0.,
        ])
    }
    /// Opacity changes alpha only; RGB remains straight and is premultiplied
    /// by the resulting alpha by the renderer.
    #[must_use]
    pub fn opacity(amount: f32) -> Self {
        Self::matrix([
            1.,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            0.,
            0.,
            0.,
            0.,
            finite_unit(amount),
            0.,
        ])
    }
}
impl Default for ColorFilter {
    fn default() -> Self {
        Self::identity()
    }
}

fn finite_unit(value: f32) -> f32 {
    finite_or_zero(value).clamp(0., 1.)
}

/// A blend operation defined over premultiplied encoded-sRGB RGBA values
/// (see [`crate::working_rgba`]). The GPU
/// compositor uses the same equations as [`blend_premultiplied`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
    #[default]
    SrcOver,
    Src,
    DstOver,
    SrcIn,
    DstIn,
    SrcOut,
    DstOut,
    SrcAtop,
    DstAtop,
    Xor,
    Plus,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    /// Component-wise source/destination modulation. Flutter uses this as
    /// the default blend mode for [`ShaderMask`](https://api.flutter.dev/flutter/widgets/ShaderMask-class.html).
    Modulate,
}
impl BlendMode {
    #[must_use]
    pub const fn requires_destination_read(self) -> bool {
        !matches!(
            self,
            Self::SrcOver
                | Self::Src
                | Self::DstOver
                | Self::SrcIn
                | Self::DstIn
                | Self::SrcOut
                | Self::DstOut
                | Self::SrcAtop
                | Self::DstAtop
                | Self::Xor
                | Self::Plus
        )
    }
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::SrcOver => 0,
            Self::Src => 1,
            Self::DstOver => 2,
            Self::SrcIn => 3,
            Self::DstIn => 4,
            Self::SrcOut => 5,
            Self::DstOut => 6,
            Self::SrcAtop => 7,
            Self::DstAtop => 8,
            Self::Xor => 9,
            Self::Plus => 10,
            Self::Multiply => 11,
            Self::Screen => 12,
            Self::Overlay => 13,
            Self::Darken => 14,
            Self::Lighten => 15,
            Self::ColorDodge => 16,
            Self::ColorBurn => 17,
            Self::HardLight => 18,
            Self::SoftLight => 19,
            Self::Difference => 20,
            Self::Exclusion => 21,
            Self::Modulate => 22,
        }
    }
}

/// Reference blend implementation for premultiplied normalized RGBA. It is
/// public so tests and alternate backends can verify their shader formulas.
#[must_use]
pub fn blend_premultiplied(mode: BlendMode, source: [f32; 4], destination: [f32; 4]) -> [f32; 4] {
    let src = sanitize_premultiplied(source);
    let dst = sanitize_premultiplied(destination);
    let as_ = src[3];
    let ad = dst[3];
    let ao = (as_ + ad - as_ * ad).clamp(0., 1.);
    let result = match mode {
        BlendMode::SrcOver => [
            src[0] + dst[0] * (1. - as_),
            src[1] + dst[1] * (1. - as_),
            src[2] + dst[2] * (1. - as_),
            ao,
        ],
        BlendMode::Src => src,
        BlendMode::DstOver => [
            dst[0] + src[0] * (1. - ad),
            dst[1] + src[1] * (1. - ad),
            dst[2] + src[2] * (1. - ad),
            ao,
        ],
        BlendMode::SrcIn => [src[0] * ad, src[1] * ad, src[2] * ad, as_ * ad],
        BlendMode::DstIn => [dst[0] * as_, dst[1] * as_, dst[2] * as_, ad * as_],
        BlendMode::SrcOut => [
            src[0] * (1. - ad),
            src[1] * (1. - ad),
            src[2] * (1. - ad),
            as_ * (1. - ad),
        ],
        BlendMode::DstOut => [
            dst[0] * (1. - as_),
            dst[1] * (1. - as_),
            dst[2] * (1. - as_),
            ad * (1. - as_),
        ],
        BlendMode::SrcAtop => [
            src[0] * ad + dst[0] * (1. - as_),
            src[1] * ad + dst[1] * (1. - as_),
            src[2] * ad + dst[2] * (1. - as_),
            ad,
        ],
        BlendMode::DstAtop => [
            dst[0] * as_ + src[0] * (1. - ad),
            dst[1] * as_ + src[1] * (1. - ad),
            dst[2] * as_ + src[2] * (1. - ad),
            as_,
        ],
        BlendMode::Xor => [
            src[0] * (1. - ad) + dst[0] * (1. - as_),
            src[1] * (1. - ad) + dst[1] * (1. - as_),
            src[2] * (1. - ad) + dst[2] * (1. - as_),
            (as_ + ad - 2. * as_ * ad).clamp(0., 1.),
        ],
        BlendMode::Plus => [
            (src[0] + dst[0]).min(1.),
            (src[1] + dst[1]).min(1.),
            (src[2] + dst[2]).min(1.),
            (as_ + ad).min(1.),
        ],
        BlendMode::Modulate => [src[0] * dst[0], src[1] * dst[1], src[2] * dst[2], as_ * ad],
        artistic => {
            let cs = straight_rgb(src);
            let cd = straight_rgb(dst);
            let blended = artistic_blend_rgb(artistic, cs, cd);
            [
                (src[0] * (1. - ad) + dst[0] * (1. - as_) + as_ * ad * blended[0]).clamp(0., 1.),
                (src[1] * (1. - ad) + dst[1] * (1. - as_) + as_ * ad * blended[1]).clamp(0., 1.),
                (src[2] * (1. - ad) + dst[2] * (1. - as_) + as_ * ad * blended[2]).clamp(0., 1.),
                ao,
            ]
        }
    };
    sanitize_premultiplied(result)
}

fn sanitize_premultiplied([r, g, b, a]: [f32; 4]) -> [f32; 4] {
    let alpha = finite_unit(a);
    let channel = |value: f32| finite_or_zero(value).clamp(0., alpha);
    [channel(r), channel(g), channel(b), alpha]
}
fn straight_rgb(value: [f32; 4]) -> [f32; 3] {
    if value[3] <= f32::EPSILON {
        [0.; 3]
    } else {
        [
            (value[0] / value[3]).clamp(0., 1.),
            (value[1] / value[3]).clamp(0., 1.),
            (value[2] / value[3]).clamp(0., 1.),
        ]
    }
}
fn artistic_blend_rgb(mode: BlendMode, source: [f32; 3], destination: [f32; 3]) -> [f32; 3] {
    let mut result = [0.; 3];
    for index in 0..3 {
        let s = source[index];
        let d = destination[index];
        result[index] = match mode {
            BlendMode::Multiply => s * d,
            BlendMode::Screen => s + d - s * d,
            BlendMode::Overlay => {
                if d <= 0.5 {
                    2. * s * d
                } else {
                    1. - 2. * (1. - s) * (1. - d)
                }
            }
            BlendMode::Darken => s.min(d),
            BlendMode::Lighten => s.max(d),
            BlendMode::ColorDodge => {
                if s >= 1. {
                    1.
                } else {
                    (d / (1. - s)).min(1.)
                }
            }
            BlendMode::ColorBurn => {
                if s <= 0. {
                    0.
                } else {
                    (1. - (1. - d) / s).max(0.)
                }
            }
            BlendMode::HardLight => {
                if s <= 0.5 {
                    2. * s * d
                } else {
                    1. - 2. * (1. - s) * (1. - d)
                }
            }
            BlendMode::SoftLight => {
                if s <= 0.5 {
                    d - (1. - 2. * s) * d * (1. - d)
                } else {
                    let g = if d <= 0.25 {
                        ((16. * d - 12.) * d + 4.) * d
                    } else {
                        d.sqrt()
                    };
                    d + (2. * s - 1.) * (g - d)
                }
            }
            BlendMode::Difference => (d - s).abs(),
            BlendMode::Exclusion => s + d - 2. * s * d,
            BlendMode::Modulate => s * d,
            _ => s,
        };
    }
    result.map(|value| value.clamp(0., 1.))
}

/// Flutter's `ui.ImageFilter` as a renderer-neutral value: the filter forms
/// `ImageFiltered` and `BackdropFilter` accept. Which forms a stage can
/// execute is decided where the filter is used; an unexecutable form is
/// reported as an [`UnsupportedImageFilter`], never applied as another one.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageFilter {
    /// `ImageFilter.blur`: a separable Gaussian blur.
    Blur(GaussianBlur),
    /// `ImageFilter.matrix`: the input transformed by an affine matrix in
    /// the filtered layer's coordinates.
    Matrix(incular_core::Transform),
    /// `ImageFilter.dilate`: each pixel the channel-wise maximum over the
    /// `2 * radius + 1` rectangle around it, radii in logical pixels.
    Dilate { radius_x: f32, radius_y: f32 },
    /// `ImageFilter.erode`: the channel-wise minimum over that rectangle.
    Erode { radius_x: f32, radius_y: f32 },
    /// A `ColorFilter` used as an image filter (Flutter's `ColorFilter`
    /// implements `ImageFilter`).
    Color(ColorFilter),
    /// `ImageFilter.compose(outer:, inner:)`: `inner` applies first and
    /// `outer` filters its result.
    Compose {
        outer: Box<ImageFilter>,
        inner: Box<ImageFilter>,
    },
}

impl ImageFilter {
    #[must_use]
    pub fn blur(sigma_x: f32, sigma_y: f32) -> Self {
        Self::Blur(GaussianBlur::new(sigma_x, sigma_y))
    }
    #[must_use]
    pub fn matrix(transform: incular_core::Transform) -> Self {
        Self::Matrix(transform)
    }
    /// Negative and non-finite radii are zero.
    #[must_use]
    pub fn dilate(radius_x: f32, radius_y: f32) -> Self {
        Self::Dilate {
            radius_x: finite_non_negative(radius_x),
            radius_y: finite_non_negative(radius_y),
        }
    }
    /// Negative and non-finite radii are zero.
    #[must_use]
    pub fn erode(radius_x: f32, radius_y: f32) -> Self {
        Self::Erode {
            radius_x: finite_non_negative(radius_x),
            radius_y: finite_non_negative(radius_y),
        }
    }
    #[must_use]
    pub fn color(filter: ColorFilter) -> Self {
        Self::Color(filter)
    }
    #[must_use]
    pub fn compose(outer: Self, inner: Self) -> Self {
        Self::Compose {
            outer: Box::new(outer),
            inner: Box::new(inner),
        }
    }
    /// The single filters this one applies, in application order: a
    /// composition flattens to its inner stages, then its outer ones.
    #[must_use]
    pub fn stages(&self) -> Vec<Self> {
        match self {
            Self::Compose { outer, inner } => {
                let mut stages = inner.stages();
                stages.extend(outer.stages());
                stages
            }
            single => vec![single.clone()],
        }
    }
}

impl From<GaussianBlur> for ImageFilter {
    fn from(blur: GaussianBlur) -> Self {
        Self::Blur(blur)
    }
}

impl From<ColorFilter> for ImageFilter {
    fn from(filter: ColorFilter) -> Self {
        Self::Color(filter)
    }
}

/// Where an [`ImageFilter`] was asked to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFilterTarget {
    /// A child subtree (`ImageFiltered`).
    Subtree,
    /// The painted backdrop (`BackdropFilter`).
    Backdrop,
}

/// A filter form a stage cannot execute. Returned instead of silently
/// substituting another filter or painting unfiltered.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsupportedImageFilter {
    /// The first stage, in application order, that cannot run.
    pub filter: ImageFilter,
    pub target: ImageFilterTarget,
}

impl std::fmt::Display for UnsupportedImageFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let form = match self.filter {
            ImageFilter::Blur(_) => "blur",
            ImageFilter::Matrix(_) => "matrix",
            ImageFilter::Dilate { .. } => "dilate",
            ImageFilter::Erode { .. } => "erode",
            ImageFilter::Color(_) => "color",
            ImageFilter::Compose { .. } => "compose",
        };
        let target = match self.target {
            ImageFilterTarget::Subtree => "a subtree",
            ImageFilterTarget::Backdrop => "a backdrop",
        };
        write!(
            formatter,
            "the {form} image filter cannot be executed on {target}"
        )
    }
}

impl std::error::Error for UnsupportedImageFilter {}

/// Ordered single-input effect descriptions. Drop shadows remain a separate
/// layer because they intentionally produce both a shadow and the original
/// source output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    GaussianBlur(GaussianBlur),
    ColorMatrix(ColorFilter),
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectChain {
    effects: Vec<Effect>,
}
impl EffectChain {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }
    #[must_use]
    pub fn from_effects(effects: Vec<Effect>) -> Self {
        Self { effects }
    }
    #[must_use]
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
    pub fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
    #[must_use]
    pub fn with_effect(mut self, effect: Effect) -> Self {
        self.push(effect);
        self
    }
    #[must_use]
    pub fn color_filter(self, filter: ColorFilter) -> Self {
        self.with_effect(Effect::ColorMatrix(filter))
    }
    #[must_use]
    pub fn blur(self, sigma: f32) -> Self {
        self.with_effect(Effect::GaussianBlur(GaussianBlur::uniform(sigma)))
    }
    /// Fuses only adjacent color matrices. Spatial effects remain explicit so
    /// their ordering and cache dependencies cannot be changed accidentally.
    #[must_use]
    pub fn optimized(&self) -> Self {
        let mut out = Self::new();
        for effect in &self.effects {
            if let (Some(Effect::ColorMatrix(previous)), Effect::ColorMatrix(next)) =
                (out.effects.last_mut(), effect)
                && let Some(fused) = previous.try_then(*next)
            {
                *previous = fused;
            } else {
                out.effects.push(*effect);
            }
        }
        out
    }
    #[must_use]
    pub fn fusion_count(&self) -> usize {
        self.effects
            .len()
            .saturating_sub(self.optimized().effects.len())
    }
}
impl DropShadowEffect {
    #[must_use]
    pub fn new(offset: Offset, sigma: f32, color: Color) -> Self {
        Self {
            offset: Offset::finite_or_zero(offset),
            sigma_x: normalize_sigma(sigma),
            sigma_y: normalize_sigma(sigma),
            color,
        }
    }
    #[must_use]
    pub fn asymmetric(offset: Offset, sigma_x: f32, sigma_y: f32, color: Color) -> Self {
        Self {
            offset: Offset::finite_or_zero(offset),
            sigma_x: normalize_sigma(sigma_x),
            sigma_y: normalize_sigma(sigma_y),
            color,
        }
    }
    #[must_use]
    pub fn blur(self) -> GaussianBlur {
        GaussianBlur::new(self.sigma_x, self.sigma_y)
    }
}

/// Canonical sigma normalization used by painting, widgets, and GPU
/// lowering. Non-finite values follow the existing "invalid numeric input is
/// transparent/zero" policy.
#[must_use]
pub fn normalize_sigma(sigma: f32) -> f32 {
    finite_non_negative(sigma)
}

/// Conservative logical blur margin for the finite 3-sigma cutoff.
#[must_use]
pub fn blur_margin(sigma: f32) -> f32 {
    (3. * normalize_sigma(sigma)).max(0.)
}

/// Expands source bounds by the finite Gaussian support in logical pixels.
#[must_use]
pub fn blur_bounds(source: Rect, sigma_x: f32, sigma_y: f32) -> Rect {
    let mx = blur_margin(sigma_x);
    let my = blur_margin(sigma_y);
    Rect::from_origin_size(
        Offset::new(source.origin.x - mx, source.origin.y - my),
        Size::new(source.size.width + 2. * mx, source.size.height + 2. * my),
    )
}

/// Bounds of a shadow plus the original source. The union is deliberately
/// conservative and works for positive, negative, and fractional offsets.
#[must_use]
pub fn drop_shadow_bounds(source: Rect, offset: Offset, sigma_x: f32, sigma_y: f32) -> Rect {
    union_rect(
        source,
        blur_bounds(
            Rect::from_origin_size(source.origin + Offset::finite_or_zero(offset), source.size),
            sigma_x,
            sigma_y,
        ),
    )
}

/// Largest physical sigma [`gaussian_kernel_weights`] builds a kernel for.
///
/// Sigma comes from application input and is only checked for finiteness, so
/// the kernel length has to be bounded or a huge sigma would allocate
/// gigabytes. The wgpu renderer runs blurs at up to 1/16 scale with kernels
/// capped at a radius of 96 pixels (sigma 32), so nothing past 16 x 32 = 512
/// physical pixels is distinguishable on screen. At the cap the kernel is
/// 3073 weights (12 KiB).
const MAX_KERNEL_SIGMA: f32 = 512.;

/// A normalized, symmetric one-dimensional Gaussian kernel represented in
/// order `[-radius, ..., 0, ..., +radius]`. The cutoff is exactly three
/// physical sigma, sigma zero is the identity kernel, and sigma is clamped to
/// 512 physical pixels to bound the kernel length.
#[must_use]
pub fn gaussian_kernel_weights(sigma_physical: f32) -> Vec<f32> {
    let sigma = normalize_sigma(sigma_physical).min(MAX_KERNEL_SIGMA);
    if sigma <= f32::EPSILON {
        return vec![1.];
    }
    let radius = (3. * sigma).ceil().max(1.) as i32;
    let denominator = 2. * sigma * sigma;
    let mut weights = (-radius..=radius)
        .map(|i| {
            let x = i as f32;
            (-(x * x) / denominator).exp()
        })
        .collect::<Vec<_>>();
    let sum: f32 = weights.iter().sum();
    if sum.is_finite() && sum > 0. {
        for weight in &mut weights {
            *weight /= sum;
        }
    } else {
        weights.fill(0.);
        weights[radius as usize] = 1.;
    }
    weights
}

/// Premultiplied shadow color for one blurred alpha sample. This helper is
/// shared by deterministic CPU tests and documents the exact GPU equation.
#[must_use]
pub fn premultiplied_shadow_sample(color: Color, mask: f32) -> [f32; 4] {
    let alpha = (f32::from(color.alpha) / 255.) * mask.clamp(0., 1.);
    let [r, g, b, _] = crate::working_rgba(color);
    [r * alpha, g * alpha, b * alpha, alpha]
}
