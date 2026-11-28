//! Equality-compatible fingerprints of decoration paint values.

use std::hash::{Hash, Hasher};

use incular_core::{Offset, Rect};
use incular_rendering::{Brush, ColorFilter, ColorFilterStage, GradientStops};

use super::{
    BoxDecoration, BoxLinearGradient, BoxRadialGradient, BoxSweepGradient, DecorationImage,
    DecorationImageSource, decoration::DecorationImageBlend, hash_paint_float,
};

fn optional<T, H: Hasher>(value: Option<&T>, state: &mut H, hash: impl FnOnce(&T, &mut H)) {
    value.is_some().hash(state);
    if let Some(value) = value {
        hash(value, state);
    }
}

fn number(value: Option<f32>, state: &mut impl Hasher) {
    optional(value.as_ref(), state, |value, state| {
        hash_paint_float(*value, state)
    });
}

fn point(point: &Offset, state: &mut impl Hasher) {
    hash_paint_float(point.x, state);
    hash_paint_float(point.y, state);
}

fn rect(rect: &Rect, state: &mut impl Hasher) {
    point(&rect.origin, state);
    hash_paint_float(rect.size.width, state);
    hash_paint_float(rect.size.height, state);
}

fn stops(stops: &GradientStops, state: &mut impl Hasher) {
    // The renderer compares normalized settings, not IDs or whether the
    // positions were explicit in the descriptor.
    stops.as_slice().len().hash(state);
    for stop in stops.as_slice() {
        hash_paint_float(stop.offset, state);
        stop.color.hash(state);
    }
}

fn filter(filter: &ColorFilter, state: &mut impl Hasher) {
    for value in filter.to_matrix() {
        hash_paint_float(value, state);
    }
    let stage = filter.stage();
    std::mem::discriminant(&stage).hash(state);
    if let ColorFilterStage::Blend { color, mode } = stage {
        color.hash(state);
        mode.hash(state);
    }
}

pub(super) fn brush_option(value: Option<&Brush>, state: &mut impl Hasher) {
    optional(value, state, |brush, state| {
        std::mem::discriminant(brush).hash(state);
        match brush {
            Brush::Solid(color) => color.hash(state),
            Brush::LinearGradient(value) => {
                point(&value.start, state);
                point(&value.end, state);
                stops(&value.stops, state);
                value.tile_mode.hash(state);
            }
            Brush::RadialGradient(value) => {
                point(&value.center, state);
                hash_paint_float(value.radius, state);
                stops(&value.stops, state);
                value.tile_mode.hash(state);
                optional(value.focal.as_ref(), state, point);
                hash_paint_float(value.focal_radius, state);
            }
            Brush::SweepGradient(value) => {
                point(&value.center, state);
                hash_paint_float(value.start_angle, state);
                number(value.end_angle, state);
                hash_paint_float(value.rotation, state);
                stops(&value.stops, state);
                value.tile_mode.hash(state);
            }
        }
    });
}

impl Hash for BoxLinearGradient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.begin.hash(state);
        self.end.hash(state);
        stops(&self.stops, state);
        self.tile_mode.hash(state);
        number(self.rotation, state);
    }
}

impl Hash for BoxRadialGradient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.center.hash(state);
        hash_paint_float(self.radius, state);
        self.focal.hash(state);
        hash_paint_float(self.focal_radius, state);
        stops(&self.stops, state);
        self.tile_mode.hash(state);
        number(self.rotation, state);
    }
}

impl Hash for BoxSweepGradient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.center.hash(state);
        hash_paint_float(self.start_angle, state);
        hash_paint_float(self.end_angle, state);
        stops(&self.stops, state);
        self.tile_mode.hash(state);
        number(self.rotation, state);
    }
}

impl Hash for DecorationImageSource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Decoded(image) => {
                image.id().hash(state);
                let scale = image.scale();
                (if scale == 0. { 0 } else { scale.to_bits() }).hash(state);
            }
            Self::Provider(value) => value.hash(state),
        }
    }
}

impl Hash for DecorationImageBlend {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.from.hash(state);
        self.to.hash(state);
        hash_paint_float(self.t, state);
        hash_paint_float(self.opacity, state);
    }
}

impl Hash for DecorationImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.image.hash(state);
        self.placeholder.hash(state);
        self.fit.hash(state);
        self.alignment.hash(state);
        self.alignment_geometry.hash(state);
        optional(self.center_slice.as_ref(), state, rect);
        self.repeat.hash(state);
        self.match_text_direction.hash(state);
        hash_paint_float(self.scale, state);
        hash_paint_float(self.opacity, state);
        optional(self.color_filter.as_ref(), state, filter);
        self.invert_colors.hash(state);
        self.filter_quality.hash(state);
        self.blend.hash(state);
        // on_error is observational; value comparison ignores it too.
    }
}

impl Hash for BoxDecoration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.color.hash(state);
        self.image.hash(state);
        self.border.hash(state);
        self.directional_border.hash(state);
        // A custom border can supply opaque equality without a hash.
        self.custom_border.is_some().hash(state);
        self.border_radius.hash(state);
        self.directional_border_radius.hash(state);
        self.box_shadow.hash(state);
        brush_option(self.gradient.as_ref(), state);
        self.aligned_gradient.hash(state);
        self.aligned_radial_gradient.hash(state);
        self.aligned_sweep_gradient.hash(state);
        self.background_blend_mode.hash(state);
        self.shape.hash(state);
    }
}
