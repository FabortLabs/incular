//! Packing for the analytic anti-aliased ellipse in framebuffer coordinates.

use incular_core::{Offset, Rect, Transform};

pub(crate) struct AffineOvalClip {
    /// Conservative world-space scissor before the one-pixel edge guard.
    pub(crate) bounds: Rect,
    center: Offset,
    /// Rows of the framebuffer-pixel-to-unit-ellipse matrix.
    pub(crate) normalized_inverse: [f32; 4],
}

impl AffineOvalClip {
    pub(crate) fn new(rect: Rect, transform: Transform, scale: f32) -> Option<Self> {
        if !(rect.size.width > 0. && rect.size.height > 0. && scale.is_finite() && scale > 0.) {
            return None;
        }
        let [a, b, c, d, _, _] = transform.inverse()?.coefficients();
        let x = 2. / (f64::from(rect.size.width) * f64::from(scale));
        let y = 2. / (f64::from(rect.size.height) * f64::from(scale));
        // The full inverse preserves nonuniform scales, rotations, and shear.
        let normalized_inverse = [a * x, c * x, b * y, d * y].map(|value| value as f32);
        if !normalized_inverse.iter().all(|value| value.is_finite()) {
            return None;
        }
        let center = transform.transform_point(
            rect.origin + Offset::new(rect.size.width * 0.5, rect.size.height * 0.5),
        );
        Some(Self {
            bounds: transform.transform_rect_bbox(rect),
            center,
            normalized_inverse,
        })
    }

    /// Same storage layout as the existing soft clip shapes. Affine ovals
    /// use the first two values for their center, not the bounding origin.
    pub(crate) fn shader_rect(&self, scale: f32) -> [f32; 4] {
        [
            self.center.x * scale,
            self.center.y * scale,
            self.bounds.size.width * scale,
            self.bounds.size.height * scale,
        ]
    }
}
