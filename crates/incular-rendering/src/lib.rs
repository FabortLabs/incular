//! Renderer-neutral scene, canvas, display-list, and compositor primitives.

mod color_space;
mod compositor;
mod display_list;
mod effects;
mod geometry;
mod glyphs;
mod gradients;
mod paint;
mod paths;

pub use color_space::{premultiplied_working_rgba, working_rgba};
pub use compositor::{
    Annotation, CompositorDiagnostics, FlattenedAnnotation, FlattenedPicture, FollowerLayer,
    LayerAnchor, LayerId, LayerKind, LayerLink, LayerTree, PublisherIdentity, normalize_opacity,
    resolve_follower_target,
};
pub use display_list::{Canvas, DisplayList, ImageSampling, PaintCommand, SurfacePartitionId};
pub use effects::{
    BlendMode, ColorFilter, ColorMatrix, DropShadowEffect, Effect, EffectChain, GaussianBlur,
    blend_premultiplied, blur_bounds, blur_margin, drop_shadow_bounds, gaussian_kernel_weights,
    normalize_sigma, premultiplied_shadow_sample,
};
pub use geometry::{CornerRadii, RRect};
pub use glyphs::{GlyphPosition, GlyphRun};
pub use gradients::{
    Brush, FilterQuality, GradientId, GradientStop, GradientStops, LinearGradient, RadialGradient,
    Shader, SweepGradient, TileMode, linear_gradient_t, radial_gradient_t, sample_gradient_stops,
    sweep_gradient_t, two_point_conical_gradient_t,
};
pub use paint::{
    Border, Decoration, FillRule, LineCap, LineJoin, Paint, PaintStyle, Shadow, Stroke,
};
pub use paths::{Path, PathBuilder, PathId};
