//! The color space Incular composites in.
//!
//! Incular blends, filters and interpolates gamma-encoded sRGB components
//! directly, as Flutter (Skia and Impeller), browsers and CSS do. An
//! `rgba(20, 20, 24, 220)` panel over a light page therefore looks like the
//! near-black its components describe instead of the mid-grey that blending
//! in linear light produces. Renderers must keep every source color, sampled
//! texel, gradient stop and framebuffer value in this one space: they upload
//! [`working_rgba`] values and render into non-sRGB targets so the GPU
//! neither decodes nor encodes at the boundary.

use incular_core::Color;

/// Straight encoded-sRGB components of `color` normalized to `0.0..=1.0`:
/// the value a renderer's shaders and blend units operate on.
#[must_use]
pub fn working_rgba(color: Color) -> [f32; 4] {
    [color.red, color.green, color.blue, color.alpha].map(|channel| f32::from(channel) / 255.)
}

/// [`working_rgba`] with RGB multiplied by alpha.
#[must_use]
pub fn premultiplied_working_rgba(color: Color) -> [f32; 4] {
    let [red, green, blue, alpha] = working_rgba(color);
    [red * alpha, green * alpha, blue * alpha, alpha]
}
