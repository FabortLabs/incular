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

impl ShaderMaskStage<'_> {
    fn placement_in_target(&self, translation: Offset) -> Transform {
        Transform::translation(translation).then(self.mask_transform)
    }
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

#[cfg(test)]
mod shader_mask_placement_tests {
    use super::*;

    #[test]
    fn cache_placement_ignores_integer_motion_but_not_reflection_or_pixel_phase() {
        let shader = Brush::Solid(crate::prelude::Color::WHITE);
        let stage = |mask_transform| ShaderMaskStage {
            shader: &shader,
            blend_mode: BlendMode::Modulate,
            mask_size: Size::new(16., 16.),
            mask_transform,
        };
        let position = |x: f32, target: f32| {
            stage(Transform::translation(Offset::new(x, 8.)))
                .placement_in_target(Offset::new(-target, -8.))
        };
        let original = position(8., 8.);
        assert_eq!(original, position(32., 32.));
        assert_ne!(original, position(32.25, 32.));
        let reflected = stage(
            Transform::translation(Offset::new(48., 8.))
                .then(Transform::scale_non_uniform(-1., 1.)),
        )
        .placement_in_target(Offset::new(-32., -8.));
        assert_ne!(reflected, original);
        assert_eq!(
            reflected.transform_point(Offset::new(3., 7.)),
            Offset::new(13., 7.)
        );
    }
}
