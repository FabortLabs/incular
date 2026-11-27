//! Renderer-neutral scene, canvas, display-list, and compositor primitives.

mod backdrop;
mod color_space;
mod composition_callbacks;
mod compositor;
mod display_list;
mod effects;
mod geometry;
mod glyphs;
mod gradients;
mod image_filter_config;
mod paint;
mod paths;
mod rsuperellipse;
mod selection;

pub use backdrop::{
    BackdropCaptures, BackdropSample, backdrop_filter_is_active, backdrop_image_filter_executable,
    backdrop_matrix_sampling, backdrop_stage_is_active,
};
pub use color_space::{premultiplied_working_rgba, working_rgba};
pub use composition_callbacks::CompositionCallbackSubscription;
pub use compositor::{
    Annotation, AnnotationEntry, AnnotationSearchOptions, CompositorDiagnostics, FlattenedAnnotation, FlattenedPicture, FollowerLayer,
    LayerAnchor, LayerId, LayerKind, LayerLink, LayerTree, PublisherIdentity, normalize_opacity,
    flutter_layer_opacity, opacity_is_transparent, resolve_follower_target, resolve_linked_follower_world_transform,
};
pub use display_list::{
    BackdropKey, Canvas, ClipCoverage, ClipIsolation, DisplayList, ImageSampling, PaintCommand,
    SurfacePartitionId, UnbalancedCanvas,
};
pub use effects::{
    BlendMode, ColorFilter, ColorFilterStage, ColorMatrix, DropShadowEffect, Effect, EffectChain,
    GaussianBlur, ImageFilter, ImageFilterTarget, UnsupportedImageFilter, blend_premultiplied,
    blur_bounds, blur_margin, drop_shadow_bounds, gaussian_kernel_weights, linear_to_srgb,
    normalize_sigma, premultiplied_shadow_sample, srgb_to_linear,
};
pub use geometry::{CornerRadii, RRect};
pub use glyphs::{GlyphPosition, GlyphRun};
pub use gradients::{
    Brush, FilterQuality, GradientId, GradientStop, GradientStops, GradientStopsError, LinearGradient,
    RadialGradient, Shader, SweepGradient, TileMode, linear_gradient_t, radial_gradient_t,
    sample_gradient_stops, sweep_gradient_t, two_point_conical_gradient_t,
};
pub use image_filter_config::{ImageFilterConfig, ImageFilterContext, SampledBlur};
pub use paint::{
    Border, Decoration, FillRule, LineCap, LineJoin, Paint, PaintStyle, Shadow, Stroke,
};
pub use paths::{Path, PathBuilder, PathId};
pub use rsuperellipse::rounded_superellipse_contains;
pub use selection::{SelectionResult, SelectionUtils};

// Double-precision geometry utilities; retained rendering still uses affine transforms.
pub use incular_core::{CylindricalOrientation, Matrix4, Point64, Rect64};
