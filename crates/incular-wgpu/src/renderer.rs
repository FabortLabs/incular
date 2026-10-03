use super::prelude::*;
use super::*;

mod capture;
mod effects;
mod frame;
mod lowering;
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
