//! Explicit backend outcomes for retained stages without execution.
//! Active backdrop filters that blend with a mode other than srcOver fail the
//! frame with a named error instead of drawing their children wrongly. A
//! shader mask reports nothing: the backend executes it for every brush
//! shader. SrcOver backdrop filters execute, and disabled and
//! zero-sigma ones are passthroughs; neither reports an error.
//!
//! These run against the single decision site the lowering arm consults,
//! so predicate and renderer cannot disagree. CPU-deterministic, no
//! device needed.

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{
    BlendMode, Brush, DisplayList, GaussianBlur, GradientStop, GradientStops, LayerTree,
    LinearGradient, PaintCommand, RadialGradient, SweepGradient,
};
use incular_wgpu::{RendererError, unsupported_effect};

fn mask_command(shader: Brush) -> PaintCommand {
    let mut tree = LayerTree::new();
    let layer = tree.create_picture(
        DisplayList::new(),
        Rect::from_origin_size(Offset::ZERO, Size::new(40., 40.)),
    );
    PaintCommand::PushShaderMask {
        layer,
        shader,
        blend_mode: BlendMode::Modulate,
        mask_size: Size::new(40., 40.),
        mask_transform: Transform::IDENTITY.into(),
        generation: 1,
        bounds: Rect::from_origin_size(Offset::ZERO, Size::new(40., 40.)),
    }
}

fn backdrop_command(sigma: f32, enabled: bool, blend_mode: BlendMode) -> PaintCommand {
    backdrop_with(GaussianBlur::uniform(sigma), None, enabled, blend_mode)
}

fn backdrop_with(
    blur: GaussianBlur,
    matrix: Option<Transform>,
    enabled: bool,
    blend_mode: BlendMode,
) -> PaintCommand {
    let mut tree = LayerTree::new();
    let layer = tree.create_picture(
        DisplayList::new(),
        Rect::from_origin_size(Offset::ZERO, Size::new(40., 40.)),
    );
    PaintCommand::PushBackdropFilter {
        layer,
        blur,
        blend_mode,
        enabled,
        matrix,
        image_filter: None,
        backdrop_key: None,
        generation: 1,
        bounds: Rect::from_origin_size(Offset::ZERO, Size::new(40., 40.)),
    }
}

#[test]
fn shader_masks_execute_for_every_brush_shader() {
    // Every renderer-neutral shader is a brush the WGPU mask stage fills, so
    // the decision site reports nothing for any of them.
    let stops = GradientStops::new(vec![
        GradientStop {
            offset: 0.,
            color: Color::WHITE,
        },
        GradientStop {
            offset: 1.,
            color: Color::TRANSPARENT,
        },
    ]);
    let mut focal = RadialGradient::new(Offset::new(20., 20.), 20., stops.clone());
    focal.focal = Some(Offset::new(10., 20.));
    for shader in [
        Brush::Solid(Color::WHITE),
        Brush::LinearGradient(LinearGradient::new(
            Offset::ZERO,
            Offset::new(40., 0.),
            stops.clone(),
        )),
        Brush::RadialGradient(RadialGradient::new(
            Offset::new(20., 20.),
            20.,
            stops.clone(),
        )),
        Brush::RadialGradient(focal),
        Brush::SweepGradient(SweepGradient::new(Offset::new(20., 20.), 0., stops)),
    ] {
        assert!(
            unsupported_effect(&mask_command(shader.clone())).is_none(),
            "a shader mask with {shader:?} must execute"
        );
    }
}

#[test]
fn src_over_backdrop_filters_execute() {
    assert!(
        unsupported_effect(&backdrop_command(4., true, BlendMode::SrcOver)).is_none(),
        "a srcOver blur executes"
    );
    let matrix = backdrop_with(
        GaussianBlur::uniform(0.),
        Some(Transform::scale(2.)),
        true,
        BlendMode::SrcOver,
    );
    assert!(
        unsupported_effect(&matrix).is_none(),
        "a srcOver matrix executes"
    );
}

#[test]
fn active_backdrops_with_another_blend_mode_report_unsupported_effect() {
    let error = unsupported_effect(&backdrop_command(4., true, BlendMode::Multiply))
        .expect("a multiply blur must report");
    assert!(
        matches!(error, RendererError::UnsupportedBackdropFilter),
        "unexpected outcome: {error:?}"
    );
    assert!(error.to_string().contains("blend mode other than srcOver"));
    let matrix = backdrop_with(
        GaussianBlur::uniform(0.),
        Some(Transform::scale(2.)),
        true,
        BlendMode::Screen,
    );
    assert!(matches!(
        unsupported_effect(&matrix),
        Some(RendererError::UnsupportedBackdropFilter)
    ));
}

#[test]
fn disabled_and_zero_sigma_backdrops_pass_through() {
    for blend_mode in [BlendMode::SrcOver, BlendMode::Multiply] {
        assert!(
            unsupported_effect(&backdrop_command(4., false, blend_mode)).is_none(),
            "a disabled filter is a passthrough"
        );
        assert!(
            unsupported_effect(&backdrop_command(0., true, blend_mode)).is_none(),
            "a zero sigma is a passthrough"
        );
    }
}

#[test]
fn ordinary_commands_have_no_unsupported_outcome() {
    let rect = PaintCommand::Rect {
        rect: Rect::from_origin_size(Offset::ZERO, Size::new(4., 4.)),
        color: Color::WHITE,
    };
    assert!(unsupported_effect(&rect).is_none());
    let clip = PaintCommand::PushClip {
        rect: Rect::from_origin_size(Offset::ZERO, Size::new(4., 4.)),
    };
    assert!(unsupported_effect(&clip).is_none());
}

#[test]
fn composed_backdrop_filters_execute_and_morphology_reports_unsupported() {
    // paint-001: a colour or composed backdrop filter runs as a chain of
    // blur, matrix and colour passes; dilation and erosion have no backdrop
    // pass and fail the frame instead of drawing as another filter.
    use incular_rendering::{ColorFilter, ImageFilter};
    let with_filter = |filter: ImageFilter| {
        let PaintCommand::PushBackdropFilter {
            layer,
            blur,
            blend_mode,
            enabled,
            matrix,
            backdrop_key,
            generation,
            bounds,
            ..
        } = backdrop_command(0., true, BlendMode::SrcOver)
        else {
            unreachable!();
        };
        PaintCommand::PushBackdropFilter {
            layer,
            blur,
            blend_mode,
            enabled,
            matrix,
            image_filter: Some(filter.into()),
            backdrop_key,
            generation,
            bounds,
        }
    };
    let composed = ImageFilter::compose(
        ImageFilter::color(ColorFilter::linear_to_srgb_gamma()),
        ImageFilter::compose(
            ImageFilter::matrix(Transform::scale(2.)),
            ImageFilter::blur(3., 3.),
        ),
    );
    assert!(unsupported_effect(&with_filter(composed)).is_none());
    assert!(
        unsupported_effect(&with_filter(ImageFilter::color(ColorFilter::identity()))).is_none()
    );
    assert!(matches!(
        unsupported_effect(&with_filter(ImageFilter::compose(
            ImageFilter::erode(1., 1.),
            ImageFilter::blur(1., 1.),
        ))),
        Some(RendererError::UnsupportedBackdropFilter)
    ));
}
