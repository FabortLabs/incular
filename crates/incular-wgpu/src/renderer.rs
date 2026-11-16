use super::prelude::*;
use super::*;

mod backdrop;
mod backdrop_cache;
mod capture;
mod effects;
mod frame;
mod lowering;
mod matrix_filter;
mod memory;
mod rendering;
mod resources;
mod state;

pub use state::WgpuRenderer;

/// One isolated effect layer being lowered: its child commands plus the
/// placement every effect stage shares.
#[derive(Clone, Copy)]
struct LayerGroup<'a> {
    commands: &'a [PaintCommand],
    scale: f32,
    layer: incular_painting::LayerId,
    generation: u64,
    bounds: Rect,
    parent_clip: ClipState,
    translation: Offset,
}

/// The mask a shader-mask group draws over its child: Flutter's
/// `ShaderMaskLayer` paints the child into an isolated layer, then draws
/// `mask_size` at `mask_transform` with `shader` and `blend_mode` into that
/// same layer.
struct ShaderMaskStage<'a> {
    shader: &'a Brush,
    blend_mode: BlendMode,
    mask_size: Size,
    mask_transform: Transform,
}

/// A matrix image filter over a subtree (Flutter `ImageFilter.matrix` in an
/// `ImageFilterLayer`, which `Transform.filterQuality` produces): the child
/// renders untransformed and is resampled through `matrix`, applied in the
/// coordinates `stage` maps to the target, with `sampling`.
#[derive(Clone, Copy)]
struct MatrixFilterStage {
    matrix: Transform,
    stage: Transform,
    sampling: incular_painting::ImageSampling,
    /// What the result covers, in target logical coordinates.
    filtered_bounds: Rect,
}
