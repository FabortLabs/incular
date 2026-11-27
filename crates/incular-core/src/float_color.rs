//! Floating color-space conversions before the renderer's sRGBA8 boundary.

use crate::Color;

/// The primaries and transfer convention used by a floating RGB color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    #[default]
    Srgb,
    /// sRGB primaries with signed encoded channels outside the unit interval.
    ExtendedSrgb,
    DisplayP3,
}

/// An unquantized, straight-alpha color in an explicitly tagged space.
///
/// Conversion is CPU-side. The ordinary paint color remains compact sRGBA8;
/// use to_srgba8 to convert and clip for that renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloatColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
    pub color_space: ColorSpace,
}

impl FloatColor {
    #[must_use]
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64, color_space: ColorSpace) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
            color_space,
        }
    }

    /// Converts primaries in linear light and preserves straight alpha.
    /// Conversion into a bounded destination clips RGB; extended sRGB
    /// keeps negative channels and values above one.
    #[must_use]
    pub fn with_color_space(self, destination: ColorSpace) -> Self {
        use ColorSpace::{DisplayP3, ExtendedSrgb, Srgb};

        if self.color_space == destination {
            return self;
        }

        let source = [self.red, self.green, self.blue];
        let converted = match (self.color_space, destination) {
            (DisplayP3, Srgb | ExtendedSrgb) => convert(source, P3_TO_SRGB),
            (Srgb | ExtendedSrgb, DisplayP3) => convert(source, SRGB_TO_P3),
            _ => source,
        };
        let [red, green, blue] = if destination == ExtendedSrgb {
            converted
        } else {
            converted.map(clip)
        };
        Self::new(red, green, blue, self.alpha, destination)
    }

    /// Converts to bounded sRGB primaries, then clamps and rounds the four
    /// encoded components at the renderer's eight-bit boundary.
    #[must_use]
    pub fn to_srgba8(self) -> Color {
        let srgb = self.with_color_space(ColorSpace::Srgb);
        let byte = |channel| (clip(channel) * 255.0).round() as u8;
        Color::rgba(
            byte(srgb.red),
            byte(srgb.green),
            byte(srgb.blue),
            byte(srgb.alpha),
        )
    }
}

impl From<Color> for FloatColor {
    fn from(color: Color) -> Self {
        let unit = |byte| f64::from(byte) / 255.0;
        Self::new(
            unit(color.red),
            unit(color.green),
            unit(color.blue),
            unit(color.alpha),
            ColorSpace::Srgb,
        )
    }
}

// D65 RGB primaries composed via XYZ, in the pinned Flutter row order.
// These coefficients operate on linear light, never on encoded bytes.
const P3_TO_SRGB: [[f64; 3]; 3] = [
    [1.2249401, -0.2249402, 0.0],
    [-0.0420569, 1.0420571, 0.0],
    [-0.0196376, -0.0786507, 1.0982884],
];
const SRGB_TO_P3: [[f64; 3]; 3] = [
    [0.8224622, 0.1775380, 0.0],
    [0.0331942, 0.9668058, 0.0],
    [0.0170806, 0.0723974, 0.9105220],
];

fn convert(encoded: [f64; 3], matrix: [[f64; 3]; 3]) -> [f64; 3] {
    let [r, g, b] = encoded.map(decode);
    matrix.map(|[a, c, d]| encode((a * r + c * g) + d * b))
}

fn decode(encoded: f64) -> f64 {
    let v = encoded.abs();
    let linear = if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    };
    linear.copysign(encoded)
}

fn encode(linear: f64) -> f64 {
    let v = linear.abs();
    let encoded = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    encoded.copysign(linear)
}

fn clip(channel: f64) -> f64 {
    if channel.is_nan() {
        0.0
    } else {
        channel.clamp(0.0, 1.0)
    }
}
